//! One flow, in full, for the inspector.

use crate::{
    connection,
    fields::{self, Field},
};
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use tracewing_i18n::tr;
use tracewing_store::{
    prost_reflect::MessageDescriptor,
    schema::Library,
    session::{self, BodyReader, Headers, LogPlace, StoredBody},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    Request,
    #[default]
    Response,
}

impl Side {
    pub fn other(self) -> Self {
        match self {
            Self::Request => Self::Response,
            Self::Response => Self::Request,
        }
    }

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
    /// The GraphQL operation, as `query GetUser`.
    pub graphql: Option<String>,
    /// The JSON-RPC method called, as `eth_call`.
    pub rpc: Option<String>,
    /// The code a failed JSON-RPC call answered with. Its message is in the
    /// body, where the Body part already shows it.
    pub rpc_error: Option<i64>,
    pub status: Option<u16>,
    pub client_version: String,
    pub server_version: Option<String>,
    pub client: String,
    /// The program on this machine that sent it, if it was found.
    pub app: Option<String>,
    pub app_pid: Option<u32>,
    pub app_path: Option<String>,
    pub content_type: Option<String>,
    pub ttfb_us: Option<i64>,
    pub duration_us: Option<i64>,
    /// The connection to the server, counted in its run.
    pub conn_id: Option<i64>,
    /// Set when it waited for the connection to be made.
    pub blocked_us: Option<i64>,
    pub dns_us: Option<i64>,
    pub connect_us: Option<i64>,
    pub tls_us: Option<i64>,
    /// What the connection was, if that is still kept.
    pub connection: Option<connection::Connection>,
    pub req_content_type: Option<String>,
    /// The fields of a form the request sent, read when the flow is chosen.
    pub form: Option<Vec<Field>>,
    pub error: Option<String>,
    pub request: BodyFacts,
    pub response: BodyFacts,
    /// `None` if they were not kept.
    pub headers: Option<(Headers, Headers)>,
    /// What came after the response's body, as gRPC's status does.
    pub trailers: Headers,
    pub grpc_status: Option<u16>,
}

/// The columns [`Detail::load`] reads, in the order it reads them by number.
/// Kept apart so that a test can say where each one lands: a column added in
/// the middle moves every number after it.
fn columns() -> String {
    format!(
        "rule, open, kind, scheme, host, path, method, status, client_version, server_version, client,
         content_type, ttfb_us, duration_us, error,
         req_bytes, req_truncated, req_shed, req_spool_lost, {},
         resp_bytes, resp_truncated, resp_shed, resp_spool_lost, {}, graphql,
         conn_id, blocked_us, dns_us, connect_us, tls_us, conn_segment, conn_offset, conn_len,
         req_content_type, app, app_pid, app_path, grpc_status, rpc, rpc_error",
        StoredBody::columns("req"),
        StoredBody::columns("resp")
    )
}

impl Detail {
    pub fn load(flows: &Connection, session_dir: &Path, id: i64) -> Result<Self> {
        let columns = columns();
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
                    // After the response's four facts and six body columns.
                    graphql: r.get(35)?,
                    conn_id: r.get(36)?,
                    blocked_us: r.get(37)?,
                    dns_us: r.get(38)?,
                    connect_us: r.get(39)?,
                    tls_us: r.get(40)?,
                    connection: None,
                    // After the connection's place in the log.
                    req_content_type: r.get(44)?,
                    app: r.get(45)?,
                    app_pid: r.get(46)?,
                    app_path: r.get(47)?,
                    grpc_status: r.get(48)?,
                    rpc: r.get(49)?,
                    rpc_error: r.get(50)?,
                    form: None,
                    headers: None,
                    trailers: Vec::new(),
                })
            },
        )?;
        let mut reader = BodyReader::new(session_dir);
        if let Some(head) = session::stored_head(flows, &mut reader, id)? {
            detail.headers = Some((head.request, head.response));
            detail.trailers = head.trailers;
        }
        detail.connection = detail
            .conn_id
            .and_then(|conn| connection_place(flows, id, conn).ok().flatten())
            .and_then(|place| reader.read_place(place).ok())
            .and_then(|json| connection::Connection::parse(&json));
        let is_form = fields::form(detail.req_content_type.as_deref(), b"").is_some();
        if is_form
            && detail.request.stored.exists()
            && detail.request.bytes as usize <= fields::FORM_MAX
        {
            detail.form = reader
                .unpacked(&detail.request.stored, fields::FORM_MAX)
                .and_then(|body| fields::form(detail.req_content_type.as_deref(), &body));
        }
        Ok(detail)
    }

    /// Each step of the time, from the flow's start, in microseconds.
    pub fn phases(&self) -> Vec<Phase> {
        let (Some(total), Some(ttfb)) = (self.duration_us, self.ttfb_us) else {
            return Vec::new();
        };
        let ttfb = ttfb.clamp(0, total.max(0));
        let mut steps: Vec<(&'static str, i64)> = Vec::new();
        if let Some(connect) = self.connect_us {
            let blocked = self.blocked_us.unwrap_or(0);
            if blocked > 0 {
                steps.push(("Queued", blocked));
            }
            if let Some(dns) = self.dns_us {
                steps.push(("DNS", dns));
            }
            steps.push(("Connect", connect));
            if let Some(tls) = self.tls_us {
                steps.push(("TLS", tls));
            }
        }
        let setup: i64 = steps.iter().map(|(_, len)| len.max(&0)).sum();
        steps.push(("Waiting", ttfb - setup));
        steps.push(("Download", total - ttfb));
        let mut at = 0;
        steps
            .into_iter()
            .map(|(label, len)| {
                let len = len.max(0);
                let phase = Phase {
                    label,
                    start: at,
                    len,
                };
                at += len;
                phase
            })
            .collect()
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

    /// The response's trailers; a request has none kept.
    pub fn trailers(&self, side: Side) -> &[(String, String)] {
        match side {
            Side::Request => &[],
            Side::Response => &self.trailers,
        }
    }

    /// `grpc-message`, which says why a call failed.
    pub fn grpc_message(&self) -> Option<String> {
        let headers = self.headers(Side::Response).unwrap_or_default();
        crate::grpc::message(&self.trailers, headers)
    }

    /// `HTTP/2 both sides`, or each side when they differ.
    pub fn protocols(&self) -> String {
        let short = |version: &str| version.replace(".0", "");
        match &self.server_version {
            Some(server) if *server == self.client_version => {
                tr!("{version} both sides", version = short(server))
            }
            Some(server) => tr!(
                "{client} to the proxy, {server} to the server",
                client = short(&self.client_version),
                server = short(server)
            ),
            None => short(&self.client_version),
        }
    }
}

/// One step of a flow's time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phase {
    pub label: &'static str,
    pub start: i64,
    pub len: i64,
}

impl Phase {
    /// What the inspector calls it; `label` is its key.
    pub fn name(&self) -> String {
        match self.label {
            "Queued" => tr!("Queued"),
            "Connect" => tr!("Connect"),
            "Waiting" => tr!("Waiting"),
            "Download" => tr!("Download"),
            other => other.to_owned(),
        }
    }
}

/// How far back to look for the flow that has a connection's facts. A
/// connection kept open for longer than this many flows shows none.
const CONNECTION_LOOKBACK: i64 = 50_000;

/// Where a connection's facts are: with this flow, or with the first flow over
/// it, which came earlier.
fn connection_place(flows: &Connection, id: i64, conn: i64) -> Result<Option<LogPlace>> {
    let place = flows
        .query_row(
            "SELECT conn_segment, conn_offset, conn_len FROM flows
             WHERE id <= ?1 AND id > ?1 - ?2 AND conn_id = ?3 AND conn_segment IS NOT NULL
             ORDER BY id DESC LIMIT 1",
            params![id, CONNECTION_LOOKBACK, conn],
            |r| {
                Ok(LogPlace {
                    segment: r.get::<_, i64>(0)? as u32,
                    offset: r.get::<_, i64>(1)? as u64,
                    len: r.get::<_, i64>(2)? as u32,
                })
            },
        )
        .optional()?;
    Ok(place)
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

/// A MessagePack, CBOR or Protobuf body beside its file as JSON, to be shown
/// as JSON is, and what it was: `MessagePack`, or `gRPC · shop.Order` when a
/// `.proto` names its fields. `None` for any other body, one too large to
/// hold, or one that cannot be read.
pub fn json_file(
    path: &std::path::Path,
    size: u64,
    content_type: Option<&str>,
    message: Option<&MessageDescriptor>,
) -> Option<(std::path::PathBuf, String)> {
    use tracewing_store::binjson;
    let format = binjson::format_of(content_type)?;
    if size > binjson::DECODE_MAX {
        return None;
    }
    let json = binjson::to_json_as(&std::fs::read(path).ok()?, format, message)?;
    let out = path.with_extension("json");
    std::fs::write(&out, json).ok()?;
    let name = match message {
        Some(message) => format!("{} · {}", format.name(), message.full_name()),
        None => format.name().to_owned(),
    };
    Some((out, name))
}

/// The message a Protobuf or gRPC body holds, if the `.proto` files chosen
/// say: the one chosen for its path by hand, else by the call's path, or as
/// its type names it. Reads the files again if they changed, so not on the
/// UI thread.
pub fn message_of(
    protos: &Library,
    url: &str,
    content_type: Option<&str>,
    side: Side,
) -> Option<MessageDescriptor> {
    use tracewing_store::binjson;
    let format = binjson::format_of(content_type).filter(|f| f.numbered())?;
    let schema = protos.schema();
    let (host, path) = host_and_path(url);
    let request = side == Side::Request;
    protos
        .chosen(host, path, request)
        .and_then(|name| schema.message(&name))
        .or_else(|| schema.message_for(path, content_type, request, format))
}

/// The host of a URL, with its port, and its path with its query.
pub fn host_and_path(url: &str) -> (&str, &str) {
    let after = url.split_once("://").map_or(url, |(_, rest)| rest);
    match after.find('/') {
        Some(at) => (&after[..at], &after[at..]),
        None => (after, "/"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `load` reads each column by number, and nothing says they still line
    /// up with the names: this does.
    #[test]
    fn each_column_is_read_by_the_number_it_is_at() {
        let columns = columns();
        let at = |name: &str| {
            columns
                .split(',')
                .position(|column| column.trim() == name)
                .unwrap_or_else(|| panic!("no column {name} in {columns}"))
        };
        assert_eq!(at("rule"), 0);
        assert_eq!(at("graphql"), 35);
        assert_eq!(at("req_content_type"), 44);
        assert_eq!(at("grpc_status"), 48);
        assert_eq!(at("rpc"), 49);
        assert_eq!(at("rpc_error"), 50);
    }

    fn detail(client: &str, server: Option<&str>) -> Detail {
        Detail {
            id: 1,
            method: "GET".into(),
            url: "https://a.dev/".into(),
            host: "a.dev".into(),
            kind: "http".into(),
            open: false,
            rule: None,
            graphql: None,
            rpc: None,
            rpc_error: None,
            status: Some(200),
            client_version: client.into(),
            server_version: server.map(str::to_owned),
            client: "127.0.0.1:1".into(),
            app: None,
            app_pid: None,
            app_path: None,
            content_type: None,
            ttfb_us: None,
            duration_us: None,
            conn_id: None,
            blocked_us: None,
            dns_us: None,
            connect_us: None,
            tls_us: None,
            connection: None,
            req_content_type: None,
            form: None,
            error: None,
            request: BodyFacts::default(),
            response: BodyFacts::default(),
            headers: None,
            trailers: Vec::new(),
            grpc_status: None,
        }
    }

    #[test]
    fn time_is_split_into_steps() {
        let mut flow = detail("HTTP/2.0", Some("HTTP/2.0"));
        flow.ttfb_us = Some(100);
        flow.duration_us = Some(130);
        let plain = |phases: Vec<Phase>| -> Vec<(&str, i64, i64)> {
            phases.iter().map(|p| (p.label, p.start, p.len)).collect()
        };
        // Over a connection made before: all it did was wait.
        assert_eq!(
            plain(flow.phases()),
            [("Waiting", 0, 100), ("Download", 100, 30)]
        );
        flow.blocked_us = Some(5);
        flow.dns_us = Some(10);
        flow.connect_us = Some(15);
        flow.tls_us = Some(20);
        assert_eq!(
            plain(flow.phases()),
            [
                ("Queued", 0, 5),
                ("DNS", 5, 10),
                ("Connect", 15, 15),
                ("TLS", 30, 20),
                ("Waiting", 50, 50),
                ("Download", 100, 30)
            ]
        );
        // An address needs no lookup, and plain HTTP no TLS.
        flow.blocked_us = Some(0);
        flow.dns_us = None;
        flow.tls_us = None;
        assert_eq!(
            plain(flow.phases()),
            [
                ("Connect", 0, 15),
                ("Waiting", 15, 85),
                ("Download", 100, 30)
            ]
        );
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
