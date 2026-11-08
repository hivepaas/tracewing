//! What the filter box means, and finding the flows that fit.
//!
//! `host:myapp method:post status:4xx type:json graphql:GetUser app:curl
//! device:iphone` narrow by column;
//! anything else
//! is text to find in URLs and in bodies. The chips and the host chosen in
//! the sidebar narrow further.

use crate::devices::{CLIENT_IP, Devices};
use anyhow::Result;
use rusqlite::{Connection, ToSql, params_from_iter};
use std::path::Path;
use tracewing_store::search;

/// The chips in the toolbar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    All,
    Json,
    GraphQl,
    Errors,
    WebSocket,
    Media,
}

impl Kind {
    pub const ALL: [Self; 6] = [
        Self::All,
        Self::Json,
        Self::GraphQl,
        Self::Errors,
        Self::WebSocket,
        Self::Media,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Json => "JSON",
            Self::GraphQl => "GraphQL",
            Self::Errors => "Errors",
            Self::WebSocket => "WebSocket",
            Self::Media => "Media",
        }
    }

    fn clause(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::Json => Some("content_type LIKE '%json%'"),
            Self::GraphQl => Some("graphql IS NOT NULL"),
            Self::Errors => Some("(status >= 400 OR status IS NULL)"),
            Self::WebSocket => Some("kind = 'websocket'"),
            Self::Media => Some(
                "(content_type LIKE 'image/%' OR content_type LIKE 'video/%' \
                 OR content_type LIKE 'audio/%' OR content_type LIKE 'font/%')",
            ),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    /// A part of the host, from the box.
    pub host: Option<String>,
    /// The whole host, from the sidebar.
    pub exact_host: Option<String>,
    pub method: Option<String>,
    /// From, up to and not including.
    pub status: Option<(u16, u16)>,
    pub content_type: Option<String>,
    /// A part of the GraphQL operation, as `GetUser` or `mutation`.
    pub graphql: Option<String>,
    /// A part of the program's name, from the box.
    pub app: Option<String>,
    /// The whole name, from the sidebar.
    pub exact_app: Option<String>,
    /// A part of another device's address or name, from the box.
    pub device: Option<String>,
    /// The devices whose names have `device` in them, when it was typed.
    pub named: Vec<String>,
    /// The device's whole address, from the sidebar: what it sent, as no program
    /// here sent it.
    pub exact_device: Option<String>,
    pub text: Option<String>,
    pub kind: Kind,
    /// Flows before this one were cleared from view.
    pub from_id: i64,
}

impl Filter {
    pub fn parse(input: &str) -> Self {
        let mut filter = Self::default();
        let mut words = Vec::new();
        for word in input.split_whitespace() {
            match word.split_once(':') {
                Some(("host", value)) if !value.is_empty() => {
                    filter.host = Some(value.to_owned());
                }
                Some(("method", value)) if !value.is_empty() => {
                    filter.method = Some(value.to_ascii_uppercase());
                }
                Some(("type", value)) if !value.is_empty() => {
                    filter.content_type = Some(value.to_owned());
                }
                Some(("graphql" | "gql", value)) if !value.is_empty() => {
                    filter.graphql = Some(value.to_owned());
                }
                Some(("app", value)) if !value.is_empty() => {
                    filter.app = Some(value.to_owned());
                }
                Some(("device", value)) if !value.is_empty() => {
                    filter.device = Some(value.to_owned());
                }
                Some(("status", value)) if status_range(value).is_some() => {
                    filter.status = status_range(value);
                }
                _ => words.push(word),
            }
        }
        if !words.is_empty() {
            filter.text = Some(words.join(" "));
        }
        filter
    }

    /// As `parse`, with the names given to devices.
    pub fn parse_with(input: &str, devices: &Devices) -> Self {
        let mut filter = Self::parse(input);
        filter.name_devices(devices);
        filter
    }

    /// Finds again which devices `device:` means, as names change.
    pub fn name_devices(&mut self, devices: &Devices) {
        self.named = self
            .device
            .as_deref()
            .map(|part| devices.named_like(part))
            .unwrap_or_default();
    }

    /// Whether every flow fits, so that no list of ids is needed.
    pub fn is_empty(&self) -> bool {
        self.host.is_none()
            && self.exact_host.is_none()
            && self.method.is_none()
            && self.status.is_none()
            && self.content_type.is_none()
            && self.graphql.is_none()
            && self.app.is_none()
            && self.exact_app.is_none()
            && self.device.is_none()
            && self.exact_device.is_none()
            && self.text.is_none()
            && self.kind == Kind::All
    }

    /// The ids of the flows that fit, oldest first.
    pub fn find(&self, flows: &Connection, session: &Path) -> Result<Vec<i64>> {
        let first_live: i64 =
            flows.query_row("SELECT coalesce(min(id), 1) FROM flows", [], |r| r.get(0))?;
        let first = first_live.max(self.from_id);
        let mut found = self.by_columns(flows, first)?;
        if let Some(text) = &self.text {
            let by_text = text_ids(flows, session, text, first)?;
            found = match found {
                Some(columns) => Some(intersect(&columns, &by_text)),
                None => Some(by_text),
            };
        }
        match found {
            Some(ids) => Ok(ids),
            None => {
                let mut stmt =
                    flows.prepare_cached("SELECT id FROM flows WHERE id >= ?1 ORDER BY id")?;
                let ids = stmt.query_map([first], |r| r.get(0))?;
                Ok(ids.collect::<rusqlite::Result<_>>()?)
            }
        }
    }

    /// `None` if no column narrows anything.
    fn by_columns(&self, flows: &Connection, first: i64) -> Result<Option<Vec<i64>>> {
        let mut clauses: Vec<String> = Vec::new();
        let mut values: Vec<Box<dyn ToSql>> = Vec::new();
        if let Some(host) = &self.host {
            clauses.push("host LIKE ?".to_owned());
            values.push(Box::new(format!("%{}%", escape_like(host))));
            clauses
                .last_mut()
                .expect("just added")
                .push_str(" ESCAPE '\\'");
        }
        if let Some(host) = &self.exact_host {
            clauses.push("host = ?".to_owned());
            values.push(Box::new(host.clone()));
        }
        if let Some(method) = &self.method {
            clauses.push("method = ?".to_owned());
            values.push(Box::new(method.clone()));
        }
        if let Some((from, to)) = self.status {
            clauses.push("status >= ? AND status < ?".to_owned());
            values.push(Box::new(from));
            values.push(Box::new(to));
        }
        if let Some(content_type) = &self.content_type {
            clauses.push("content_type LIKE ? ESCAPE '\\'".to_owned());
            values.push(Box::new(format!("%{}%", escape_like(content_type))));
        }
        if let Some(graphql) = &self.graphql {
            clauses.push("graphql LIKE ? ESCAPE '\\'".to_owned());
            values.push(Box::new(format!("%{}%", escape_like(graphql))));
        }
        if let Some(app) = &self.app {
            clauses.push("app LIKE ? ESCAPE '\\'".to_owned());
            values.push(Box::new(format!("%{}%", escape_like(app))));
        }
        if let Some(app) = &self.exact_app {
            clauses.push("app = ?".to_owned());
            values.push(Box::new(app.clone()));
        }
        if let Some(device) = &self.device {
            let mut clause = format!("({CLIENT_IP} LIKE ? ESCAPE '\\'");
            values.push(Box::new(format!("%{}%", escape_like(device))));
            if !self.named.is_empty() {
                let marks = vec!["?"; self.named.len()].join(", ");
                clause.push_str(&format!(" OR {CLIENT_IP} IN ({marks})"));
                for ip in &self.named {
                    values.push(Box::new(ip.clone()));
                }
            }
            clause.push(')');
            clauses.push(clause);
        }
        if let Some(ip) = &self.exact_device {
            clauses.push(format!("app IS NULL AND {CLIENT_IP} = ?"));
            values.push(Box::new(ip.clone()));
        }
        if let Some(clause) = self.kind.clause() {
            clauses.push(clause.to_owned());
        }
        if clauses.is_empty() {
            return Ok(None);
        }
        values.push(Box::new(first));
        let sql = format!(
            "SELECT id FROM flows WHERE {} AND id >= ? ORDER BY id",
            clauses.join(" AND ")
        );
        let mut stmt = flows.prepare(&sql)?;
        let ids = stmt.query_map(params_from_iter(values.iter().map(|v| &**v)), |r| r.get(0))?;
        Ok(Some(ids.collect::<rusqlite::Result<_>>()?))
    }
}

/// `4xx`, or a status as it is.
fn status_range(value: &str) -> Option<(u16, u16)> {
    if let Some(class) = value.strip_suffix("xx") {
        let class: u16 = class.parse().ok()?;
        return (1..=5)
            .contains(&class)
            .then_some((class * 100, (class + 1) * 100));
    }
    let status: u16 = value.parse().ok()?;
    Some((status, status + 1))
}

fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Flows with this text in the URL or the body, oldest first. A word of fewer than
/// three letters has no trigrams to look up, so URLs are scanned for it.
fn text_ids(flows: &Connection, session: &Path, text: &str, first: i64) -> Result<Vec<i64>> {
    let mut ids = search::find(session, "bodies", text, first).unwrap_or_default();
    match search::find(session, "urls", text, first) {
        Ok(urls) if text.chars().count() >= 3 => ids.extend(urls),
        _ => {
            let mut stmt = flows.prepare_cached(
                "SELECT id FROM flows WHERE host || path LIKE ?1 ESCAPE '\\' AND id >= ?2",
            )?;
            let urls = stmt.query_map(
                rusqlite::params![format!("%{}%", escape_like(text)), first],
                |r| r.get::<_, i64>(0),
            )?;
            for id in urls {
                ids.push(id?);
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// What is in both of two sorted lists.
fn intersect(a: &[i64], b: &[i64]) -> Vec<i64> {
    let (mut i, mut j) = (0, 0);
    let mut both = Vec::new();
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                both.push(a[i]);
                i += 1;
                j += 1;
            }
        }
    }
    both
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_is_read_word_by_word() {
        let filter = Filter::parse(
            "host:myapp method:post status:4xx type:json gql:GetUser app:curl device:ipho đặt hàng",
        );
        assert_eq!(filter.host.as_deref(), Some("myapp"));
        assert_eq!(filter.method.as_deref(), Some("POST"));
        assert_eq!(filter.status, Some((400, 500)));
        assert_eq!(filter.content_type.as_deref(), Some("json"));
        assert_eq!(filter.graphql.as_deref(), Some("GetUser"));
        assert_eq!(filter.app.as_deref(), Some("curl"));
        assert_eq!(filter.device.as_deref(), Some("ipho"));
        assert_eq!(filter.text.as_deref(), Some("đặt hàng"));
    }

    #[test]
    fn what_is_not_a_filter_is_text() {
        let filter = Filter::parse("status:maybe http://a.b/c host:");
        assert_eq!(filter.status, None);
        assert_eq!(filter.host, None);
        assert_eq!(
            filter.text.as_deref(),
            Some("status:maybe http://a.b/c host:")
        );
    }

    #[test]
    fn statuses() {
        assert_eq!(status_range("404"), Some((404, 405)));
        assert_eq!(status_range("5xx"), Some((500, 600)));
        assert_eq!(status_range("9xx"), None);
        assert_eq!(status_range("abc"), None);
    }

    #[test]
    fn nothing_typed_is_empty() {
        assert!(Filter::parse("  ").is_empty());
        assert!(!Filter::parse("x").is_empty());
    }

    #[test]
    fn devices_are_found_by_address_or_name() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE flows (id INTEGER PRIMARY KEY, host TEXT, app TEXT, client TEXT);
             INSERT INTO flows (id, app, client) VALUES
               (1, 'curl', '127.0.0.1:5000'),
               (2, NULL, '192.168.1.20:5001'),
               (3, NULL, '[::ffff:192.168.1.21]:5002'),
               (4, 'curl', '192.168.1.20:5003'),
               (5, NULL, '[fd00::7]:5004');",
        )
        .unwrap();
        let dir = std::env::temp_dir().join(format!("filter-devices-{}", std::process::id()));
        let mut devices = Devices::load(&dir.join("devices.toml"));
        devices.set("192.168.1.21", "iPhone test").unwrap();
        let ids = |filter: &Filter| filter.by_columns(&conn, 1).unwrap().unwrap();
        assert_eq!(
            ids(&Filter::parse_with("device:192.168", &devices)),
            [2, 3, 4]
        );
        assert_eq!(ids(&Filter::parse_with("device:iphone", &devices)), [3]);
        assert_eq!(ids(&Filter::parse_with("device:fd00", &devices)), [5]);
        // From the sidebar: what no program here sent.
        let filter = Filter {
            exact_device: Some("192.168.1.20".into()),
            ..Filter::default()
        };
        assert_eq!(ids(&filter), [2]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn lists_cross() {
        assert_eq!(intersect(&[1, 3, 5, 7], &[2, 3, 7, 9]), [3, 7]);
        assert!(intersect(&[], &[1]).is_empty());
    }
}
