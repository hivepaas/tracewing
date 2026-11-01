//! One flow, in full, for the inspector.

use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;
use tracewing_store::session::{self, BodyReader, Headers, StoredBody};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    Request,
    #[default]
    Response,
}

impl Side {
    pub fn column(self) -> &'static str {
        match self {
            Self::Request => "req",
            Self::Response => "resp",
        }
    }
}

/// What can be said about a body before opening it.
#[derive(Clone, Debug, Default)]
pub struct BodyFacts {
    pub bytes: u64,
    pub stored: StoredBody,
    pub truncated: bool,
    /// Let go on purpose: the disk was short or too much was waiting.
    pub shed: bool,
    pub lost: bool,
}

#[derive(Clone, Debug)]
pub struct Detail {
    pub id: i64,
    pub method: String,
    pub url: String,
    pub host: String,
    /// `http`, `websocket`, `sse`, or `tunnel` for a host left alone.
    pub kind: String,
    /// A stream still going.
    pub open: bool,
    /// The rules that changed it.
    pub rule: Option<String>,
    pub status: Option<u16>,
    pub client_version: String,
    pub server_version: Option<String>,
    pub client: String,
    pub content_type: Option<String>,
    pub ttfb_us: Option<i64>,
    pub duration_us: Option<i64>,
    pub error: Option<String>,
    pub request: BodyFacts,
    pub response: BodyFacts,
    /// `None` if they were not kept.
    pub headers: Option<(Headers, Headers)>,
}

impl Detail {
    pub fn load(flows: &Connection, session_dir: &Path, id: i64) -> Result<Self> {
        let columns = format!(
            "rule, open, kind, scheme, host, path, method, status, client_version, server_version, client,
             content_type, ttfb_us, duration_us, error,
             req_bytes, req_truncated, req_shed, req_spool_lost, {},
             resp_bytes, resp_truncated, resp_shed, resp_spool_lost, {}",
            StoredBody::columns("req"),
            StoredBody::columns("resp")
        );
        let mut detail = flows.query_row(
            &format!("SELECT {columns} FROM flows WHERE id = ?1"),
            params![id],
            |r| {
                let kind: String = r.get(2)?;
                let scheme: String = r.get(3)?;
                let host: String = r.get(4)?;
                let path: String = r.get(5)?;
                let facts = |first: usize| -> rusqlite::Result<BodyFacts> {
                    Ok(BodyFacts {
                        bytes: r.get::<_, i64>(first)? as u64,
                        truncated: r.get(first + 1)?,
                        shed: r.get(first + 2)?,
                        lost: r.get(first + 3)?,
                        stored: StoredBody::from_row(r, first + 4)?,
                    })
                };
                Ok(Self {
                    id,
                    url: format!("{}://{host}{path}", shown_scheme(&kind, &scheme)),
                    host,
                    rule: r.get(0)?,
                    open: r.get(1)?,
                    kind,
                    method: r.get(6)?,
                    status: r.get(7)?,
                    client_version: r.get(8)?,
                    server_version: r.get(9)?,
                    client: r.get(10)?,
                    content_type: r.get(11)?,
                    ttfb_us: r.get(12)?,
                    duration_us: r.get(13)?,
                    error: r.get(14)?,
                    request: facts(15)?,
                    response: facts(25)?,
                    headers: None,
                })
            },
        )?;
        let mut reader = BodyReader::new(session_dir);
        detail.headers = session::stored_headers(flows, &mut reader, id)?;
        Ok(detail)
    }

    pub fn body(&self, side: Side) -> &BodyFacts {
        match side {
            Side::Request => &self.request,
            Side::Response => &self.response,
        }
    }

    pub fn headers(&self, side: Side) -> Option<&[(String, String)]> {
        self.headers.as_ref().map(|(request, response)| match side {
            Side::Request => request.as_slice(),
            Side::Response => response.as_slice(),
        })
    }

    /// `HTTP/2 both sides`, or each side when they differ.
    pub fn protocols(&self) -> String {
        let short = |version: &str| version.replace(".0", "");
        match &self.server_version {
            Some(server) if *server == self.client_version => {
                format!("{} both sides", short(server))
            }
            Some(server) => format!(
                "{} to the proxy, {} to the server",
                short(&self.client_version),
                short(server)
            ),
            None => short(&self.client_version),
        }
    }
}

/// A WebSocket's URL as the client wrote it: what comes through the proxy says https.
fn shown_scheme<'a>(kind: &str, scheme: &'a str) -> &'a str {
    match (kind, scheme) {
        ("websocket", "https") => "wss",
        ("websocket", "http") => "ws",
        _ => scheme,
    }
}

/// Writes a body, unpacked, where a viewer can map it. Returns its size and whether
/// it was already there. `None` if it is gone or cannot be read.
pub fn body_file(
    session_dir: &Path,
    facts: &BodyFacts,
    id: i64,
    side: Side,
    max: u64,
) -> Option<(std::path::PathBuf, u64)> {
    let dir = session_dir.join("view");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{id}-{}", side.column()));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&path).ok()?);
    let size = BodyReader::new(session_dir).unpack_into(&facts.stored, max, &mut out)?;
    std::io::Write::flush(&mut out).ok()?;
    Some((path, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detail(client: &str, server: Option<&str>) -> Detail {
        Detail {
            id: 1,
            method: "GET".into(),
            url: "https://a.dev/".into(),
            host: "a.dev".into(),
            kind: "http".into(),
            open: false,
            rule: None,
            status: Some(200),
            client_version: client.into(),
            server_version: server.map(str::to_owned),
            client: "127.0.0.1:1".into(),
            content_type: None,
            ttfb_us: None,
            duration_us: None,
            error: None,
            request: BodyFacts::default(),
            response: BodyFacts::default(),
            headers: None,
        }
    }

    #[test]
    fn a_websocket_url_says_so() {
        assert_eq!(shown_scheme("websocket", "https"), "wss");
        assert_eq!(shown_scheme("websocket", "http"), "ws");
        assert_eq!(shown_scheme("http", "https"), "https");
    }

    #[test]
    fn protocols_say_what_differs() {
        assert_eq!(
            detail("HTTP/2.0", Some("HTTP/2.0")).protocols(),
            "HTTP/2 both sides"
        );
        assert_eq!(
            detail("HTTP/1.1", Some("HTTP/2.0")).protocols(),
            "HTTP/1.1 to the proxy, HTTP/2 to the server"
        );
        assert_eq!(detail("HTTP/1.1", None).protocols(), "HTTP/1.1");
    }
}
