//! Rows for the list, read from the session as they are needed. Ids have no gaps,
//! so a place in the list is an id; with a filter on, a list of ids says which.

use crate::devices::device_of;
use anyhow::{Context as _, Result};
use gpui::SharedString;
use rusqlite::{Connection, OpenFlags, params};
use std::{collections::HashMap, path::Path};
use tracewing_i18n::tr;
use tracewing_store::session;

/// More rows than this and the cache starts again. A screen is a few dozen.
const CACHE_MAX: usize = 20_000;

#[derive(Clone, Debug)]
pub struct Row {
    pub id: i64,
    pub number: SharedString,
    pub method: SharedString,
    pub status: Option<u16>,
    /// gRPC's status, when the call failed.
    pub grpc_failed: Option<u16>,
    pub host: SharedString,
    pub path: SharedString,
    /// The program on this machine that sent it, when it was found.
    pub app: Option<SharedString>,
    /// Else the address of the other device it came from.
    pub device: Option<SharedString>,
    /// The GraphQL operation, as `query GetUser`.
    pub graphql: Option<SharedString>,
    /// What went wrong, or that it is a WebSocket.
    pub note: Option<SharedString>,
    pub size: SharedString,
    pub time: SharedString,
    pub slow: bool,
    /// A stream still going, which will change.
    pub going: bool,
}

pub struct Flows {
    conn: Connection,
    cache: HashMap<i64, Row>,
    /// The session recording now. In an earlier one, what was open never finished.
    live: bool,
}

impl Flows {
    pub fn open(session_dir: &Path, live: bool) -> Result<Self> {
        let path = session::flows_db(session_dir);
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("open {}", path.display()))?;
        Ok(Self {
            conn,
            cache: HashMap::new(),
            live,
        })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// The oldest and the newest flow.
    pub fn bounds(&self) -> Result<Option<(i64, i64)>> {
        let (first, last): (Option<i64>, Option<i64>) = self
            .conn
            // As two queries: SQLite reads the whole table for both in one.
            .query_row(
                "SELECT (SELECT min(id) FROM flows), (SELECT max(id) FROM flows)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
        Ok(first.zip(last))
    }

    /// These rows, in this order, with `None` for one that cannot be read now: it
    /// has gone with the oldest flows, or the session is busy.
    pub fn rows(&mut self, ids: &[i64]) -> Vec<Option<Row>> {
        if self.cache.len() > CACHE_MAX {
            self.cache.clear();
        }
        let missing: Vec<i64> = ids
            .iter()
            .copied()
            .filter(|id| !self.cache.contains_key(id))
            .collect();
        let _ = self.fetch(&missing);
        ids.iter()
            .map(|id| {
                let row = self.cache.get(id).cloned();
                // Not kept: it says something else when it ends.
                if row.as_ref().is_some_and(|row| row.going) {
                    self.cache.remove(id);
                }
                row
            })
            .collect()
    }

    /// A row still going changes when it ends, so it is read again.
    pub fn forget(&mut self, id: i64) {
        self.cache.remove(&id);
    }

    /// Every row is read again: their notes are in another language now.
    pub fn forget_all(&mut self) {
        self.cache.clear();
    }

    fn fetch(&mut self, ids: &[i64]) -> Result<()> {
        let live = self.live;
        // Scrolling asks for runs of ids, which one query reads far faster.
        let mut at = 0;
        while at < ids.len() {
            let mut end = at + 1;
            while end < ids.len() && ids[end] == ids[end - 1] + 1 {
                end += 1;
            }
            let mut stmt = self.conn.prepare_cached(
                "SELECT id, method, status, host, path, resp_bytes, duration_us, error, kind, open, rule, graphql, app, client,
                        grpc_status
                 FROM flows WHERE id >= ?1 AND id <= ?2",
            )?;
            let rows = stmt.query_map(params![ids[at], ids[end - 1]], |r| {
                let id: i64 = r.get(0)?;
                let duration: Option<i64> = r.get(6)?;
                let error: Option<String> = r.get(7)?;
                let kind: String = r.get(8)?;
                let open: bool = r.get(9)?;
                let going = open && live;
                let rule: Option<String> = r.get(10)?;
                let grpc_failed = r.get::<_, Option<u16>>(14)?.filter(|code| *code != 0);
                Ok(Row {
                    id,
                    number: group_digits(id as u64).into(),
                    method: r.get::<_, String>(1)?.into(),
                    status: r.get(2)?,
                    grpc_failed,
                    host: r.get::<_, String>(3)?.into(),
                    path: r.get::<_, String>(4)?.into(),
                    app: r.get::<_, Option<String>>(12)?.map(SharedString::from),
                    device: device_of(&r.get::<_, String>(13)?).map(SharedString::from),
                    graphql: r
                        .get::<_, Option<String>>(11)?
                        .map(|operation| short_operation(&operation).into()),
                    note: error
                        .map(SharedString::from)
                        .or_else(|| {
                            grpc_failed.map(|code| {
                                format!("gRPC {}", crate::grpc::code_name(code).unwrap_or("error"))
                                    .into()
                            })
                        })
                        .or_else(|| rule.map(|rule| tr!("Rule: {rule}", rule = rule).into()))
                        .or_else(|| match kind.as_str() {
                            "websocket" => Some("WebSocket".into()),
                            "sse" => Some(tr!("Events").into()),
                            "ndjson" => Some(tr!("JSON lines").into()),
                            "tunnel" => Some(tr!("Not decrypted").into()),
                            _ => None,
                        }),
                    // What goes through a tunnel is not looked at, nor counted.
                    size: if kind == "tunnel" {
                        "–".into()
                    } else {
                        size_text(r.get::<_, i64>(5)? as u64).into()
                    },
                    time: if going {
                        tr!("open").into()
                    } else {
                        duration.map(time_text).unwrap_or_default().into()
                    },
                    slow: duration.is_some_and(|d| d >= 1_000_000),
                    going,
                })
            })?;
            for row in rows {
                let row = row?;
                self.cache.insert(row.id, row);
            }
            at = end;
        }
        Ok(())
    }
}

/// `GetUser` of `query GetUser`: the list is narrow, and the name says most.
/// One without a name keeps its kind.
pub fn short_operation(operation: &str) -> String {
    match operation.split_once(' ') {
        Some(("query" | "mutation" | "subscription", name)) => name.to_owned(),
        _ => operation.to_owned(),
    }
}

pub fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

pub fn size_text(bytes: u64) -> String {
    const KB: f64 = 1024.;
    let b = bytes as f64;
    if b >= KB * KB * KB {
        format!("{:.1} GB", b / (KB * KB * KB))
    } else if b >= KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

pub fn time_text(micros: i64) -> String {
    if micros >= 1_000_000 {
        format!("{:.1} s", micros as f64 / 1e6)
    } else if micros >= 1_000 {
        format!("{} ms", micros / 1_000)
    } else {
        format!("{micros} µs")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_well() {
        assert_eq!(group_digits(7), "7");
        assert_eq!(group_digits(1_284_551), "1,284,551");
        assert_eq!(size_text(612), "612 B");
        assert_eq!(size_text(18 * 1024 + 410), "18.4 KB");
        assert_eq!(time_text(142_000), "142 ms");
        assert_eq!(time_text(2_400_000), "2.4 s");
        assert_eq!(time_text(800), "800 µs");
    }

    #[test]
    fn operations_go_by_their_names() {
        assert_eq!(short_operation("query GetUser"), "GetUser");
        assert_eq!(short_operation("mutation A +2"), "A +2");
        assert_eq!(short_operation("query"), "query");
        assert_eq!(short_operation("Feed"), "Feed");
    }
}
