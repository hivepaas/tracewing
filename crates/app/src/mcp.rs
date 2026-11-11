//! An MCP server, so that AI tools can look at what tracewing recorded:
//! `tracewing mcp`, which an AI tool starts and talks to over stdin and stdout,
//! a JSON-RPC message a line.
//!
//! It only reads: the sessions on the disk, the one recording now among them,
//! with the same filter as the window's box. Tokens and cookies are left out
//! unless Settings say an AI tool may see them; the tool cannot ask for them
//! itself.

use crate::{
    detail::{BodyFacts, Detail, Side},
    devices::{Devices, device_of},
    filter::Filter,
};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
};
use tracewing_store::{
    binjson,
    session::{self, BodyReader},
    settings::StorageSettings,
};

/// The newest first: the one answered with, unless the client asks for one
/// of the others.
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const LIMIT_DEFAULT: u64 = 50;
const LIMIT_MAX: u64 = 500;
const BODY_DEFAULT: u64 = 16 * 1024;
const BODY_MAX: u64 = 1 << 20;
/// Headers whose values are left out, as an exported HAR leaves them out.
const SECRET_HEADERS: [&str; 4] = [
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
];
const REMOVED: &str = "(removed)";

const INSTRUCTIONS: &str = "tracewing is an HTTP(S) debugging proxy. These tools read the \
traffic it recorded: list_sessions, then list_flows to find requests, with the same filter \
as tracewing's search box, then get_flow for one request's headers, bodies and timing. \
Values of credential headers and cookies are removed unless the user allowed them in \
tracewing's Settings.";

/// Runs until stdin closes.
pub fn serve(data_dir: &Path) -> Result<()> {
    let server = Server::new(data_dir);
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lines() {
        let line = line.context("read from the client")?;
        if line.trim().is_empty() {
            continue;
        }
        for reply in server.handle_line(&line) {
            writeln!(stdout, "{reply}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

pub struct Server {
    data_dir: PathBuf,
}

impl Server {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_owned(),
        }
    }

    /// The replies to one line: none for a notification, one for a request,
    /// and one each for a batch's requests.
    pub fn handle_line(&self, line: &str) -> Vec<Value> {
        match serde_json::from_str::<Value>(line) {
            Ok(Value::Array(batch)) => batch.iter().filter_map(|m| self.handle(m)).collect(),
            Ok(message) => self.handle(&message).into_iter().collect(),
            Err(e) => vec![json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": format!("not JSON: {e}") },
            })],
        }
    }

    fn handle(&self, message: &Value) -> Option<Value> {
        // A reply from the client, to nothing this asked: there is nothing to say.
        let method = message.get("method")?.as_str()?;
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        let result = match method {
            "initialize" => Ok(initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => self.call(&params),
            _ => Err((-32601, format!("there is no method {method}"))),
        };
        // A notification has no id, and gets no answer.
        let id = message.get("id")?.clone();
        Some(match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": code, "message": message },
            }),
        })
    }

    fn call(&self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((-32602, "a tool's name is needed".to_owned()))?;
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let outcome = match name {
            "list_sessions" => self.list_sessions(),
            "list_flows" => self.list_flows(&args),
            "get_flow" => self.get_flow(&args),
            _ => return Err((-32602, format!("there is no tool {name}"))),
        };
        Ok(match outcome {
            Ok(value) => json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&value).unwrap_or_default(),
                }],
                "structuredContent": value,
                "isError": false,
            }),
            Err(e) => json!({
                "content": [{ "type": "text", "text": format!("{e:#}") }],
                "isError": true,
            }),
        })
    }

    fn sessions_dir(&self) -> PathBuf {
        self.data_dir.join("sessions")
    }

    /// Read at each call, so that a change in Settings counts at once.
    fn secrets_allowed(&self) -> bool {
        StorageSettings::load(&self.data_dir.join("settings.toml"))
            .is_ok_and(|settings| settings.ai_secrets)
    }

    /// The session asked for by its id, or the newest.
    fn session(&self, args: &Value) -> Result<PathBuf> {
        let sessions = session::list(&self.sessions_dir());
        let chosen = match args.get("session").and_then(Value::as_str) {
            Some(id) => sessions
                .into_iter()
                .find(|s| session_id(&s.dir) == id)
                .with_context(|| format!("there is no session {id}"))?,
            None => sessions
                .into_iter()
                .next()
                .context("tracewing has recorded no session yet")?,
        };
        if !chosen.readable {
            bail!(
                "session {} cannot be read by this version",
                session_id(&chosen.dir)
            );
        }
        Ok(chosen.dir)
    }

    fn list_sessions(&self) -> Result<Value> {
        let sessions: Vec<Value> = session::list(&self.sessions_dir())
            .into_iter()
            .enumerate()
            .map(|(at, s)| {
                json!({
                    "id": session_id(&s.dir),
                    "name": s.name,
                    "newest": at == 0,
                    "flows": s.flows,
                    "first": s.first_us.map(time_text),
                    "last": s.last_us.map(time_text),
                    "readable": s.readable,
                })
            })
            .collect();
        Ok(json!({ "sessions": sessions }))
    }

    fn list_flows(&self, args: &Value) -> Result<Value> {
        let dir = self.session(args)?;
        let conn = open(&dir)?;
        let devices = Devices::load(&self.data_dir.join("devices.toml"));
        let filter = args.get("filter").and_then(Value::as_str).unwrap_or("");
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(LIMIT_DEFAULT)
            .clamp(1, LIMIT_MAX) as usize;
        let before = args.get("before").and_then(Value::as_i64);
        let mut ids = Filter::parse_with(filter, &devices).find(&conn, &dir)?;
        ids.sort_unstable();
        // The newest first, as someone asking about traffic wants it.
        let ids: Vec<i64> = ids
            .into_iter()
            .rev()
            .filter(|id| before.is_none_or(|before| *id < before))
            .take(limit + 1)
            .collect();
        let more = ids.len() > limit;
        let shown = &ids[..ids.len().min(limit)];
        let flows = rows(&conn, shown, &devices)?;
        Ok(json!({
            "session": session_id(&dir),
            "flows": flows,
            "more": more,
            "next_before": more.then(|| shown.last().copied()).flatten(),
        }))
    }

    fn get_flow(&self, args: &Value) -> Result<Value> {
        let dir = self.session(args)?;
        let id = args
            .get("id")
            .and_then(Value::as_i64)
            .context("get_flow needs the flow's id, as list_flows gives it")?;
        let conn = open(&dir)?;
        let detail = Detail::load(&conn, &dir, id)
            .with_context(|| format!("there is no flow {id} in this session"))?;
        let bodies = args.get("bodies").and_then(Value::as_bool).unwrap_or(true);
        let max = args
            .get("max_body_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(BODY_DEFAULT)
            .min(BODY_MAX);
        let secrets = self.secrets_allowed();
        let mut reader = BodyReader::new(&dir);
        let mut side = |side: Side| {
            let facts = detail.body(side);
            let headers = detail.headers(side).map(|headers| {
                headers
                    .iter()
                    .chain(detail.trailers(side))
                    .map(|(name, value)| {
                        let secret = SECRET_HEADERS.contains(&name.to_ascii_lowercase().as_str());
                        let value = if secret && !secrets { REMOVED } else { value };
                        json!([name, value])
                    })
                    .collect::<Vec<_>>()
            });
            let content_type = match side {
                Side::Request => detail.req_content_type.clone(),
                Side::Response => detail.content_type.clone(),
            };
            let (body, note) = if bodies {
                body_text(&mut reader, facts, max, content_type.as_deref())
            } else {
                (None, None)
            };
            json!({
                "headers": headers,
                "content_type": content_type,
                "bytes": facts.bytes,
                "body": body,
                "body_note": note,
            })
        };
        let request = side(Side::Request);
        let response = side(Side::Response);
        let ms = |us: Option<i64>| us.map(|us| us as f64 / 1000.0);
        Ok(json!({
            "session": session_id(&dir),
            "id": detail.id,
            "method": detail.method,
            "url": detail.url,
            "kind": detail.kind,
            "status": detail.status,
            "grpc_status": detail.grpc_status,
            "grpc_message": detail.grpc_message(),
            "error": detail.error,
            "still_open": detail.open,
            "client_version": detail.client_version,
            "server_version": detail.server_version,
            "client": detail.client,
            "app": detail.app,
            "device": detail.app.is_none().then(|| device_of(&detail.client)).flatten(),
            "graphql": detail.graphql,
            "changed_by_rules": detail.rule,
            "timing_ms": {
                "dns": ms(detail.dns_us),
                "connect": ms(detail.connect_us),
                "tls": ms(detail.tls_us),
                "first_byte": ms(detail.ttfb_us),
                "total": ms(detail.duration_us),
            },
            "secrets_removed": !secrets,
            "request": request,
            "response": response,
        }))
    }
}

fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = asked
        .filter(|v| PROTOCOL_VERSIONS.contains(v))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "tracewing", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

fn tools() -> Value {
    json!([
        {
            "name": "list_sessions",
            "description": "The sessions tracewing has recorded or imported, the newest first. \
                The newest is usually the one recording now.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true },
        },
        {
            "name": "list_flows",
            "description": "Requests in a session, the newest first. `filter` is tracewing's \
                search box: host:api.example.com, method:POST, status:404, status:5xx, \
                type:json, app:chrome, device:phone, graphql:GetUser, grpc:error, grpc:NOT_FOUND, \
                and plain words to find in URLs and bodies. Page back with `before` set to `next_before`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": { "type": "string", "description": "A session's id; the newest if left out." },
                    "filter": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": LIMIT_MAX, "default": LIMIT_DEFAULT },
                    "before": { "type": "integer", "description": "Only flows with an id below this." },
                },
            },
            "annotations": { "readOnlyHint": true },
        },
        {
            "name": "get_flow",
            "description": "One request and its response: headers, bodies as text (unpacked; \
                binary bodies are described, not given), timing, the connection, and any error.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session": { "type": "string", "description": "A session's id; the newest if left out." },
                    "id": { "type": "integer" },
                    "bodies": { "type": "boolean", "default": true },
                    "max_body_bytes": { "type": "integer", "maximum": BODY_MAX, "default": BODY_DEFAULT },
                },
                "required": ["id"],
            },
            "annotations": { "readOnlyHint": true },
        },
    ])
}

fn session_id(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn open(dir: &Path) -> Result<Connection> {
    let path = session::flows_db(dir);
    Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("open {}", path.display()))
}

fn time_text(us: i64) -> String {
    chrono::DateTime::from_timestamp_micros(us)
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default()
}

/// The flows with these ids, in this order.
fn rows(conn: &Connection, ids: &[i64], devices: &Devices) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, started_us, method, scheme, host, path, status, kind, resp_bytes,
                duration_us, app, client, error, graphql, rule, open, grpc_status
         FROM flows WHERE id = ?1",
    )?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let row = stmt.query_row([id], |r| {
            let scheme: String = r.get(3)?;
            let host: String = r.get(4)?;
            let path: String = r.get(5)?;
            let app: Option<String> = r.get(10)?;
            let client: String = r.get(11)?;
            let device = app
                .is_none()
                .then(|| device_of(&client))
                .flatten()
                .map(|ip| devices.name(&ip).map_or(ip.clone(), str::to_owned));
            Ok(json!({
                "id": r.get::<_, i64>(0)?,
                "started": time_text(r.get(1)?),
                "method": r.get::<_, String>(2)?,
                "url": format!("{scheme}://{host}{path}"),
                "status": r.get::<_, Option<u16>>(6)?,
                "grpc_status": r.get::<_, Option<u16>>(16)?,
                "kind": r.get::<_, String>(7)?,
                "response_bytes": r.get::<_, i64>(8)?,
                "duration_ms": r.get::<_, Option<i64>>(9)?.map(|us| us as f64 / 1000.0),
                "app": app,
                "device": device,
                "error": r.get::<_, Option<String>>(12)?,
                "graphql": r.get::<_, Option<String>>(13)?,
                "changed_by_rules": r.get::<_, Option<String>>(14)?,
                "still_open": r.get::<_, bool>(15)?,
            }))
        });
        match row {
            Ok(row) => out.push(row),
            // Gone with the oldest flows since the ids were found.
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(out)
}

/// MessagePack, CBOR or Protobuf as the JSON it holds, cut at `max`, and a note that
/// says what it was. `None` for any other body, or one that cannot be read.
fn as_json(
    bytes: &[u8],
    content_type: Option<&str>,
    sent: u64,
    max: u64,
) -> Option<(String, String)> {
    let format = binjson::format_of(content_type)?;
    let json = binjson::to_json(bytes, format)?;
    let mut end = json.len().min(max as usize);
    while !json.is_char_boundary(end) {
        end -= 1;
    }
    let mut note = format!("{}, {sent} bytes sent, given as JSON", format.name());
    if format.numbered() {
        note.push_str(" with fields by number");
    }
    if end < json.len() {
        note.push_str(&format!("; only the first {end} bytes of {}", json.len()));
    }
    Some((json[..end].to_owned(), note))
}

/// A body as text, cut at `max`, and what was left out of it, if anything.
/// MessagePack, CBOR and Protobuf are given as the JSON they hold, and say so.
fn body_text(
    reader: &mut BodyReader,
    facts: &BodyFacts,
    max: u64,
    content_type: Option<&str>,
) -> (Option<String>, Option<String>) {
    if facts.bytes == 0 {
        return (None, None);
    }
    if !facts.stored.exists() {
        return (None, Some("not kept".to_owned()));
    }
    if binjson::format_of(content_type).is_some()
        && let Some(bytes) = reader.unpacked(&facts.stored, binjson::DECODE_MAX as usize)
        && let Some((json, note)) = as_json(&bytes, content_type, facts.bytes, max)
    {
        return (Some(json), Some(note));
    }
    let Some(bytes) = reader.unpacked(&facts.stored, max as usize + 1) else {
        return (None, Some("cannot be read".to_owned()));
    };
    let cut = bytes.len() as u64 > max;
    let kept = &bytes[..bytes.len().min(max as usize)];
    let text = match std::str::from_utf8(kept) {
        Ok(text) => text,
        // Cut in the middle of a character: as far as the last whole one.
        Err(e) if e.error_len().is_none() => {
            std::str::from_utf8(&kept[..e.valid_up_to()]).unwrap_or_default()
        }
        Err(_) => {
            return (None, Some(format!("binary, {} bytes sent", facts.bytes)));
        }
    };
    let note = (cut || facts.truncated).then(|| {
        format!(
            "only the first {} bytes; {} were sent",
            text.len(),
            facts.bytes
        )
    });
    (Some(text.to_owned()), note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messagepack_is_given_as_json_and_says_so() {
        let mut bytes = Vec::new();
        let value = rmpv::Value::Map(vec![("city".into(), "Hà Nội".into())]);
        rmpv::encode::write_value(&mut bytes, &value).unwrap();
        let ct = Some("application/msgpack");
        let (json, note) = as_json(&bytes, ct, 99, 1000).unwrap();
        assert_eq!(json, r#"{"city":"Hà Nội"}"#);
        assert_eq!(note, "MessagePack, 99 bytes sent, given as JSON");
        // Cut on a character's edge: "à" is two bytes, at 10 and 11.
        let (cut, note) = as_json(&bytes, ct, 99, 11).unwrap();
        assert_eq!(cut, r#"{"city":"H"#);
        assert!(note.ends_with("only the first 10 bytes of 20"), "{note}");
        assert_eq!(as_json(&bytes, Some("application/json"), 99, 1000), None);
        // Protobuf has no names for its fields with no schema.
        let (json, note) = as_json(b"\x08\x2a", Some("application/x-protobuf"), 2, 1000).unwrap();
        assert_eq!(json, r#"{"1":42}"#);
        assert_eq!(
            note,
            "Protobuf, 2 bytes sent, given as JSON with fields by number"
        );
    }

    fn ask(server: &Server, message: Value) -> Value {
        let mut replies = server.handle_line(&message.to_string());
        assert_eq!(replies.len(), 1, "{message}");
        replies.remove(0)
    }

    #[test]
    fn it_says_hello_in_the_version_asked() {
        let server = Server::new(Path::new("/nowhere"));
        let reply = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": { "protocolVersion": "2025-03-26", "capabilities": {},
                                "clientInfo": { "name": "t", "version": "1" } } }),
        );
        assert_eq!(reply["id"], 1);
        assert_eq!(reply["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(reply["result"]["serverInfo"]["name"], "tracewing");
        let unknown = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize",
                    "params": { "protocolVersion": "1999-01-01" } }),
        );
        assert_eq!(unknown["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
    }

    #[test]
    fn notifications_get_no_answer_and_unknown_methods_an_error() {
        let server = Server::new(Path::new("/nowhere"));
        assert!(
            server
                .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
                .is_empty()
        );
        let reply = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": "a", "method": "resources/list" }),
        );
        assert_eq!(reply["error"]["code"], -32601);
        let bad = server.handle_line("{not json");
        assert_eq!(bad[0]["error"]["code"], -32700);
        let tools = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }),
        );
        let names: Vec<&str> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["list_sessions", "list_flows", "get_flow"]);
    }

    #[test]
    fn a_tool_that_fails_says_why_as_its_result() {
        let dir = std::env::temp_dir().join(format!("tracewing-mcp-{}", std::process::id()));
        let server = Server::new(&dir);
        let reply = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call",
                    "params": { "name": "list_flows", "arguments": {} } }),
        );
        assert_eq!(reply["result"]["isError"], true);
        assert!(
            reply["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("no session yet")
        );
        let none = ask(
            &server,
            json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call",
                    "params": { "name": "rm_rf", "arguments": {} } }),
        );
        assert_eq!(none["error"]["code"], -32602);
    }
}
