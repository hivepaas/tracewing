//! HAR 1.2 files: a session's flows written out for other tools, and a HAR
//! from elsewhere read into a session of its own.
//!
//! Bodies go whole, as text when they are text and in base64 when not, up to
//! [`BODY_MAX`] each. A WebSocket's messages go as Chrome writes them, in
//! `_webSocketMessages`. Times are in milliseconds, and `connect` includes
//! `ssl`, as the format says.

use crate::{
    decode, graphql,
    import::{self, Connections, host_of, unix_micros, version_of},
    session::{self, BodyReader, Headers, LogPlace, StoredBody},
    settings::StorageSettings,
    store::FlowRecord,
};
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// The most of one body that goes in a HAR. Past it the text stops, and the
/// entry's `comment` says so.
pub const BODY_MAX: usize = 64 << 20;
/// The most messages of one WebSocket that go in a HAR.
const MESSAGES_MAX: usize = 100_000;
/// Headers whose values are left out when asked to, with the cookies.
const SECRET_HEADERS: [&str; 4] = [
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
];
const REMOVED: &str = "(removed)";
/// How far back to look for the flow that has a connection's facts.
const CONNECTION_LOOKBACK: i64 = 50_000;

#[derive(Clone, Copy, Debug, Default)]
pub struct ExportOptions {
    /// Leave out the values of headers that carry credentials, and the cookies.
    pub without_secrets: bool,
}

/// Writes these flows of a session, all of them if `ids` is `None`, as HAR.
/// Returns how many went. Tunnels are left out: nothing in them was read.
pub fn export(
    session: &Path,
    ids: Option<&[i64]>,
    options: ExportOptions,
    out: &mut impl Write,
) -> Result<u64> {
    let flows = Connection::open_with_flags(
        session::flows_db(session),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("open {}", session.display()))?;
    let mut reader = BodyReader::new(session);
    let mut remotes: HashMap<i64, Option<String>> = HashMap::new();
    write!(
        out,
        r#"{{"log":{{"version":"1.2","creator":{{"name":"tracewing","version":"{}"}},"pages":[],"entries":["#,
        env!("CARGO_PKG_VERSION")
    )?;
    let all: Vec<i64>;
    let ids = match ids {
        Some(ids) => ids,
        None => {
            let mut stmt = flows.prepare("SELECT id FROM flows ORDER BY id")?;
            all = stmt
                .query_map([], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            &all
        }
    };
    let mut written = 0u64;
    for &id in ids {
        let Some(row) = Row::load(&flows, id)? else {
            continue;
        };
        if row.kind == "tunnel" {
            continue;
        }
        let remote = match row.conn_id {
            Some(conn) => remotes
                .entry(conn)
                .or_insert_with(|| remote_of(&flows, &mut reader, id, conn))
                .clone(),
            None => None,
        };
        let entry = row.entry(&flows, &mut reader, remote, options)?;
        if written > 0 {
            out.write_all(b",")?;
        }
        serde_json::to_writer(&mut *out, &entry)?;
        written += 1;
    }
    out.write_all(b"]}}")?;
    out.flush()?;
    Ok(written)
}

/// A flow's row, as much of it as a HAR needs.
struct Row {
    id: i64,
    started_us: i64,
    kind: String,
    scheme: String,
    host: String,
    path: String,
    method: String,
    status: Option<u16>,
    client_version: String,
    server_version: Option<String>,
    content_type: Option<String>,
    req_content_type: Option<String>,
    ttfb_us: Option<i64>,
    duration_us: Option<i64>,
    error: Option<String>,
    req_bytes: i64,
    resp_bytes: i64,
    request: StoredBody,
    response: StoredBody,
    conn_id: Option<i64>,
    blocked_us: Option<i64>,
    dns_us: Option<i64>,
    connect_us: Option<i64>,
    tls_us: Option<i64>,
}

impl Row {
    fn load(flows: &Connection, id: i64) -> Result<Option<Self>> {
        let sql = format!(
            "SELECT started_us, kind, scheme, host, path, method, status, client_version,
                    server_version, content_type, req_content_type, ttfb_us, duration_us, error,
                    req_bytes, resp_bytes, conn_id, blocked_us, dns_us, connect_us, tls_us,
                    {}, {}
             FROM flows WHERE id = ?1",
            StoredBody::columns("req"),
            StoredBody::columns("resp"),
        );
        Ok(flows
            .query_row(&sql, params![id], |r| {
                Ok(Self {
                    id,
                    started_us: r.get(0)?,
                    kind: r.get(1)?,
                    scheme: r.get(2)?,
                    host: r.get(3)?,
                    path: r.get(4)?,
                    method: r.get(5)?,
                    status: r.get(6)?,
                    client_version: r.get(7)?,
                    server_version: r.get(8)?,
                    content_type: r.get(9)?,
                    req_content_type: r.get(10)?,
                    ttfb_us: r.get(11)?,
                    duration_us: r.get(12)?,
                    error: r.get(13)?,
                    req_bytes: r.get(14)?,
                    resp_bytes: r.get(15)?,
                    conn_id: r.get(16)?,
                    blocked_us: r.get(17)?,
                    dns_us: r.get(18)?,
                    connect_us: r.get(19)?,
                    tls_us: r.get(20)?,
                    request: StoredBody::from_row(r, 21)?,
                    response: StoredBody::from_row(r, 27)?,
                })
            })
            .optional()?)
    }

    fn url(&self) -> String {
        let scheme = match (self.kind.as_str(), self.scheme.as_str()) {
            ("websocket", "https") => "wss",
            ("websocket", "http") => "ws",
            (_, scheme) => scheme,
        };
        format!("{scheme}://{}{}", self.host, self.path)
    }

    fn entry(
        &self,
        flows: &Connection,
        reader: &mut BodyReader,
        remote: Option<String>,
        options: ExportOptions,
    ) -> Result<Value> {
        let (request_headers, response_headers) =
            session::stored_headers(flows, reader, self.id)?.unwrap_or_default();
        let mut cut = Vec::new();
        let mut body = |stored: &StoredBody, side: &str| -> Option<Vec<u8>> {
            if !stored.exists() {
                return None;
            }
            let mut bytes = reader.unpacked(stored, BODY_MAX + 1)?;
            if bytes.len() > BODY_MAX {
                bytes.truncate(BODY_MAX);
                cut.push(side.to_owned());
            }
            Some(bytes)
        };
        let request_body = body(&self.request, "request");
        let response_body = body(&self.response, "response");
        let secret = options.without_secrets;
        let url = self.url();

        let mut request = json!({
            "method": self.method,
            "url": url,
            "httpVersion": self.client_version,
            "cookies": if secret { Vec::new() } else { request_cookies(&request_headers) },
            "headers": header_list(&request_headers, secret),
            "queryString": query_string(&self.path),
            "headersSize": -1,
            "bodySize": self.req_bytes,
        });
        if let Some(bytes) = &request_body {
            let mime = self.req_content_type.clone().unwrap_or_default();
            let mut post = json!({ "mimeType": mime });
            put_text(&mut post, bytes, Some(&mime));
            request["postData"] = post;
        }

        let mime = self.content_type.clone().unwrap_or_default();
        let mut content = json!({
            "size": response_body.as_ref().map_or(self.resp_bytes, |b| b.len() as i64),
            "mimeType": mime,
        });
        if let Some(bytes) = &response_body {
            put_text(&mut content, bytes, Some(&mime));
            let saved = bytes.len() as i64 - self.resp_bytes;
            if saved > 0 {
                content["compression"] = json!(saved);
            }
        }
        let status = self.status.unwrap_or(0);
        let response = json!({
            "status": status,
            "statusText": http::StatusCode::from_u16(status)
                .ok()
                .and_then(|s| s.canonical_reason())
                .unwrap_or(""),
            "httpVersion": self.server_version.clone().unwrap_or_else(|| self.client_version.clone()),
            "cookies": if secret { Vec::new() } else { response_cookies(&response_headers) },
            "headers": header_list(&response_headers, secret),
            "content": content,
            "redirectURL": header(&response_headers, "location").unwrap_or_default(),
            "headersSize": -1,
            "bodySize": self.resp_bytes,
        });

        let millis = |us: i64| us as f64 / 1000.0;
        let or_none = |us: Option<i64>| us.map_or(-1.0, millis);
        let total = self.duration_us.unwrap_or(0).max(0);
        let ttfb = self.ttfb_us.unwrap_or(total).clamp(0, total);
        let setup: i64 = [self.blocked_us, self.dns_us, self.connect_us, self.tls_us]
            .iter()
            .flatten()
            .map(|us| (*us).max(0))
            .sum();
        let timings = json!({
            "blocked": or_none(self.blocked_us),
            "dns": or_none(self.dns_us),
            "connect": self
                .connect_us
                .map_or(-1.0, |c| millis(c + self.tls_us.unwrap_or(0))),
            "ssl": or_none(self.tls_us),
            "send": 0,
            "wait": millis((ttfb - setup).max(0)),
            "receive": millis(total - ttfb),
        });

        let mut entry = json!({
            "startedDateTime": date_text(self.started_us),
            "time": millis(total),
            "request": request,
            "response": response,
            "cache": {},
            "timings": timings,
        });
        if let Some(remote) = remote {
            entry["serverIPAddress"] = json!(remote);
        }
        if let Some(conn) = self.conn_id {
            entry["connection"] = json!(conn.to_string());
        }
        if let Some(error) = &self.error {
            entry["_error"] = json!(error);
        }
        if !cut.is_empty() {
            entry["comment"] = json!(format!(
                "The {} body is cut at {} MiB.",
                cut.join(" and "),
                BODY_MAX >> 20
            ));
        }
        if self.kind == "websocket" {
            entry["_webSocketMessages"] = json!(messages(flows, reader, self.id)?);
        }
        Ok(entry)
    }
}

/// `text`, or `text` in base64 with `encoding`, as HAR has it.
fn put_text(object: &mut Value, bytes: &[u8], mime: Option<&str>) {
    match std::str::from_utf8(bytes) {
        Ok(text) if decode::is_text(mime) || !text.contains('\0') => {
            object["text"] = json!(text);
        }
        _ => {
            object["text"] = json!(STANDARD.encode(bytes));
            object["encoding"] = json!("base64");
        }
    }
}

fn messages(flows: &Connection, reader: &mut BodyReader, id: i64) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let mut after = 0;
    while out.len() < MESSAGES_MAX {
        let batch = session::stored_messages(flows, id, after, 1000)?;
        let Some(last) = batch.last() else {
            break;
        };
        after = last.id;
        for message in batch {
            let opcode = match message.kind.as_str() {
                "text" => 1,
                "binary" => 2,
                "close" => 8,
                "ping" => 9,
                "pong" => 10,
                _ => continue,
            };
            let data = message
                .place
                .and_then(|place| reader.read_place(place).ok())
                .unwrap_or_default();
            let data = match (opcode, String::from_utf8(data)) {
                (1, Ok(text)) => text,
                (_, Ok(text)) if opcode != 2 => text,
                (_, Ok(text)) => STANDARD.encode(text),
                (_, Err(e)) => STANDARD.encode(e.into_bytes()),
            };
            out.push(json!({
                "type": if message.from_client { "send" } else { "receive" },
                "time": message.at_us as f64 / 1e6,
                "opcode": opcode,
                "data": data,
            }));
        }
    }
    Ok(out)
}

/// The server's address, from the facts of the connection a flow went over.
fn remote_of(flows: &Connection, reader: &mut BodyReader, id: i64, conn: i64) -> Option<String> {
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
        .optional()
        .ok()??;
    let facts: Value = serde_json::from_slice(&reader.read_place(place).ok()?).ok()?;
    let remote = facts.get("remote")?.as_str()?;
    // `1.2.3.4:443` or `[::1]:443`: HAR wants the address alone.
    let address = remote
        .rsplit_once(':')
        .map_or(remote, |(address, _)| address);
    Some(
        address
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned(),
    )
}

fn header<'a>(headers: &'a Headers, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn header_list(headers: &Headers, without_secrets: bool) -> Vec<Value> {
    headers
        .iter()
        .map(|(name, value)| {
            let secret =
                without_secrets && SECRET_HEADERS.contains(&name.to_ascii_lowercase().as_str());
            json!({ "name": name, "value": if secret { REMOVED } else { value } })
        })
        .collect()
}

fn request_cookies(headers: &Headers) -> Vec<Value> {
    headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        .flat_map(|(_, value)| value.split(';'))
        .filter_map(|pair| {
            let pair = pair.trim();
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (!name.is_empty()).then(|| json!({ "name": name.trim(), "value": value.trim() }))
        })
        .collect()
}

fn response_cookies(headers: &Headers) -> Vec<Value> {
    headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .filter_map(|(_, value)| {
            let mut parts = value.split(';').map(str::trim);
            let (name, value) = parts.next()?.split_once('=')?;
            let mut cookie = json!({ "name": name, "value": value });
            for part in parts {
                let (key, value) = part.split_once('=').unwrap_or((part, ""));
                match key.to_ascii_lowercase().as_str() {
                    "path" => cookie["path"] = json!(value),
                    "domain" => cookie["domain"] = json!(value),
                    "expires" => cookie["expires"] = json!(value),
                    "httponly" => cookie["httpOnly"] = json!(true),
                    "secure" => cookie["secure"] = json!(true),
                    "samesite" => cookie["sameSite"] = json!(value),
                    _ => {}
                }
            }
            Some(cookie)
        })
        .collect()
}

fn query_string(path: &str) -> Vec<Value> {
    let Some((_, query)) = path.split_once('?') else {
        return Vec::new();
    };
    decode::form_pairs(query)
        .into_iter()
        .map(|(name, value)| json!({ "name": name, "value": value }))
        .collect()
}

fn date_text(unix_us: i64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(unix_us) * 1000)
        .ok()
        .and_then(|t| t.format(&Rfc3339).ok())
        .unwrap_or_default()
}

// Reading a HAR. Fields are as the format names them; anything missing is
// taken as empty, since tools leave out different things.

#[derive(Deserialize)]
struct Har {
    log: Log,
}

#[derive(Deserialize)]
struct Log {
    #[serde(default)]
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    started_date_time: String,
    #[serde(default)]
    time: f64,
    request: Request,
    response: Response,
    #[serde(default)]
    timings: Option<Timings>,
    #[serde(default, rename = "serverIPAddress")]
    server_ip_address: Option<String>,
    #[serde(default)]
    connection: Option<String>,
    #[serde(default, rename = "_error")]
    error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    method: String,
    url: String,
    #[serde(default)]
    http_version: String,
    #[serde(default)]
    headers: Vec<Pair>,
    #[serde(default)]
    post_data: Option<PostData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    #[serde(default)]
    status: i64,
    #[serde(default)]
    http_version: String,
    #[serde(default)]
    headers: Vec<Pair>,
    #[serde(default)]
    content: Option<Content>,
}

#[derive(Deserialize)]
struct Pair {
    name: String,
    #[serde(default)]
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostData {
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    encoding: Option<String>,
    #[serde(default)]
    params: Vec<Pair>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Content {
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    encoding: Option<String>,
}

#[derive(Deserialize, Default)]
struct Timings {
    #[serde(default = "absent")]
    blocked: f64,
    #[serde(default = "absent")]
    dns: f64,
    #[serde(default = "absent")]
    connect: f64,
    #[serde(default = "absent")]
    ssl: f64,
    #[serde(default)]
    send: f64,
    #[serde(default)]
    wait: f64,
    #[serde(default)]
    receive: f64,
}

fn absent() -> f64 {
    -1.0
}

pub use crate::import::Imported;

/// Reads a HAR into `session`, a directory that must not hold one yet, and
/// names the session `name`. Bodies are kept as the HAR has them, unpacked.
pub fn import(
    har: impl Read,
    session: &Path,
    name: &str,
    settings: &StorageSettings,
) -> Result<Imported> {
    let har: Har =
        serde_json::from_reader(std::io::BufReader::new(har)).context("this is not a HAR file")?;
    let mut entries = har.log.entries;
    // Connections are counted in the order they were made.
    entries.sort_by_cached_key(|e| unix_micros(&e.started_date_time).unwrap_or(0));
    let mut connections = Connections::default();
    let records = entries
        .into_iter()
        .map(|entry| record_of(entry, &mut connections))
        .collect();
    import::write(records, session, name, settings)
}

fn record_of(entry: Entry, connections: &mut Connections) -> Option<FlowRecord> {
    let uri: http::Uri = entry.request.url.parse().ok()?;
    let scheme = uri.scheme_str()?.to_ascii_lowercase();
    let authority = uri.authority()?.as_str().to_owned();
    let (scheme, websocket) = match scheme.as_str() {
        "wss" => ("https".to_owned(), true),
        "ws" => ("http".to_owned(), true),
        "http" | "https" => (scheme, entry.response.status == 101),
        _ => return None,
    };
    let host = host_of(&authority, &scheme);
    let path = uri.path_and_query().map_or("/", |p| p.as_str()).to_owned();
    let lines = |pairs: &[Pair]| -> String {
        pairs
            .iter()
            .map(|p| format!("{}: {}\n", p.name, p.value))
            .collect()
    };
    let content_type = |pairs: &[Pair]| {
        pairs
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case("content-type"))
            .map(|p| p.value.clone())
    };

    let request_body = entry.request.post_data.as_ref().map(|post| {
        match &post.text {
            Some(text) => body_bytes(text, post.encoding.as_deref()),
            // A form, given as fields.
            None => post
                .params
                .iter()
                .map(|p| format!("{}={}", p.name, p.value))
                .collect::<Vec<_>>()
                .join("&")
                .into_bytes(),
        }
    });
    let response_body = entry.response.content.as_ref().and_then(|c| {
        c.text
            .as_ref()
            .map(|t| body_bytes(t, c.encoding.as_deref()))
    });
    let req_content_type = content_type(&entry.request.headers).or_else(|| {
        entry
            .request
            .post_data
            .as_ref()
            .and_then(|p| p.mime_type.clone())
    });
    let resp_content_type = content_type(&entry.response.headers).or_else(|| {
        entry
            .response
            .content
            .as_ref()
            .and_then(|c| c.mime_type.clone())
            .filter(|m| !m.is_empty())
    });

    let timings = entry.timings.unwrap_or_default();
    let us = |ms: f64| (ms * 1000.0).round() as i64;
    let present = |ms: f64| (ms >= 0.0).then(|| us(ms));
    let made = timings.connect >= 0.0;
    let ssl = present(timings.ssl);
    let before_wait: f64 = [
        timings.blocked,
        timings.dns,
        timings.connect,
        timings.send,
        timings.wait,
    ]
    .iter()
    .filter(|ms| **ms > 0.0)
    .sum();
    let status = u16::try_from(entry.response.status).ok().filter(|s| *s > 0);
    let duration = if entry.time > 0.0 {
        us(entry.time)
    } else {
        us(before_wait + timings.receive.max(0.0))
    };

    // Each connection, or each address if there are none, is counted from 1.
    let key = entry
        .connection
        .clone()
        .filter(|c| !c.is_empty())
        .or_else(|| entry.server_ip_address.clone());
    let conn = connections.over(key);
    let connection = match (&conn, &entry.server_ip_address) {
        (Some((_, true)), Some(ip)) => {
            Some(serde_json::to_vec(&json!({ "remote": ip })).unwrap_or_default())
        }
        _ => None,
    };

    let method = entry.request.method.to_ascii_uppercase();
    let graphql = graphql::operation(
        &method,
        &path,
        req_content_type.as_deref(),
        request_body.as_deref().unwrap_or_default(),
    );
    Some(FlowRecord {
        started_us: unix_micros(&entry.started_date_time)?,
        client: "HAR".to_owned(),
        kind: if websocket { "websocket" } else { "http" },
        method,
        scheme,
        host,
        path,
        client_version: version_of(&entry.request.http_version),
        server_version: status.map(|_| version_of(&entry.response.http_version)),
        status,
        content_type: resp_content_type,
        req_content_type,
        // Bodies in a HAR are unpacked already.
        req_encoding: None,
        resp_encoding: None,
        req_headers: lines(&entry.request.headers),
        resp_headers: lines(&entry.response.headers),
        graphql,
        conn_id: conn.map(|(id, _)| id),
        blocked_us: made.then(|| present(timings.blocked).unwrap_or(0)),
        dns_us: made.then(|| present(timings.dns)).flatten(),
        connect_us: made.then(|| us(timings.connect) - ssl.unwrap_or(0).max(0)),
        tls_us: made.then_some(ssl).flatten(),
        connection,
        ttfb_us: status.map(|_| us(before_wait)),
        duration_us: Some(duration),
        request: import::body(request_body.unwrap_or_default()),
        response: import::body(response_body.unwrap_or_default()),
        error: entry.error,
        ..FlowRecord::default()
    })
}

fn body_bytes(text: &str, encoding: Option<&str>) -> Vec<u8> {
    match encoding {
        Some(e) if e.eq_ignore_ascii_case("base64") => STANDARD
            .decode(text.trim())
            .unwrap_or_else(|_| text.as_bytes().to_vec()),
        _ => text.as_bytes().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tracewing-har-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const SAMPLE: &str = r#"{"log":{"version":"1.2","creator":{"name":"x","version":"1"},"entries":[
      {"startedDateTime":"2026-11-07T10:00:01.500Z","time":120,
       "request":{"method":"POST","url":"https://api.test/graphql?x=1","httpVersion":"h2",
         "headers":[{"name":"content-type","value":"application/json"},{"name":"authorization","value":"Bearer abc"}],
         "postData":{"mimeType":"application/json","text":"{\"query\":\"query Me { me { id } }\"}"}},
       "response":{"status":200,"httpVersion":"h2",
         "headers":[{"name":"content-type","value":"application/json"},{"name":"set-cookie","value":"sid=1; Path=/; HttpOnly"}],
         "content":{"size":11,"mimeType":"application/json","text":"{\"me\":null}"}},
       "timings":{"blocked":1,"dns":2,"connect":30,"ssl":20,"send":0,"wait":70,"receive":17},
       "serverIPAddress":"10.0.0.5","connection":"77"},
      {"startedDateTime":"2026-11-07T10:00:00Z","time":5,
       "request":{"method":"GET","url":"http://plain.test:8080/a.png","headers":[]},
       "response":{"status":200,"headers":[],
         "content":{"size":3,"mimeType":"image/png","text":"iVBO","encoding":"base64"}},
       "timings":{"send":0,"wait":4,"receive":1}},
      {"startedDateTime":"2026-11-07T10:00:02Z","request":{"method":"GET","url":"not a url"},"response":{"status":0}}
    ]}}"#;

    #[test]
    fn a_har_goes_in_and_comes_out_the_same() {
        let dir = temp("round");
        let session = dir.join("imported");
        let settings = StorageSettings::default();
        let imported = import(SAMPLE.as_bytes(), &session, "From a HAR", &settings).unwrap();
        assert_eq!(
            imported,
            Imported {
                flows: 2,
                skipped: 1
            }
        );
        assert_eq!(session::name_of(&session).as_deref(), Some("From a HAR"));

        let mut out = Vec::new();
        let count = export(&session, None, ExportOptions::default(), &mut out).unwrap();
        assert_eq!(count, 2);
        let har: Value = serde_json::from_slice(&out).unwrap();
        let entries = har["log"]["entries"].as_array().unwrap();
        // In the order they started, not as the file had them.
        let first = &entries[0];
        assert_eq!(first["request"]["url"], "http://plain.test:8080/a.png");
        assert_eq!(first["response"]["content"]["encoding"], "base64");
        assert_eq!(first["response"]["content"]["text"], "iVBO");
        assert_eq!(first["timings"]["connect"], -1.0);
        assert_eq!(first["timings"]["wait"], 4.0);

        let second = &entries[1];
        assert_eq!(second["startedDateTime"], "2026-11-07T10:00:01.5Z");
        assert_eq!(second["request"]["httpVersion"], "HTTP/2.0");
        assert_eq!(
            second["request"]["postData"]["text"],
            r#"{"query":"query Me { me { id } }"}"#
        );
        assert_eq!(second["request"]["queryString"][0]["name"], "x");
        assert_eq!(second["response"]["content"]["text"], r#"{"me":null}"#);
        assert_eq!(second["response"]["cookies"][0]["httpOnly"], true);
        assert_eq!(second["serverIPAddress"], "10.0.0.5");
        let timings = &second["timings"];
        assert_eq!(
            [
                &timings["blocked"],
                &timings["dns"],
                &timings["connect"],
                &timings["ssl"],
                &timings["wait"],
                &timings["receive"],
            ],
            [1.0, 2.0, 30.0, 20.0, 70.0, 17.0]
        );
        assert_eq!(second["time"], 120.0);

        // The GraphQL operation was read on the way in.
        let flows = Connection::open(session::flows_db(&session)).unwrap();
        let operation: Option<String> = flows
            .query_row("SELECT graphql FROM flows WHERE id = 2", [], |r| r.get(0))
            .unwrap();
        assert_eq!(operation.as_deref(), Some("query Me"));
        // Windows will not delete a database that is still open.
        drop(flows);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn secrets_can_be_left_out() {
        let dir = temp("secrets");
        let session = dir.join("imported");
        import(
            SAMPLE.as_bytes(),
            &session,
            "x",
            &StorageSettings::default(),
        )
        .unwrap();
        let mut out = Vec::new();
        export(
            &session,
            Some(&[2]),
            ExportOptions {
                without_secrets: true,
            },
            &mut out,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(!text.contains("Bearer abc"), "{text}");
        assert!(!text.contains("sid=1"), "{text}");
        let har: Value = serde_json::from_str(&text).unwrap();
        let entry = &har["log"]["entries"][0];
        assert_eq!(entry["response"]["cookies"], json!([]));
        assert!(
            entry["request"]["headers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["name"] == "authorization" && h["value"] == REMOVED)
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn what_is_not_a_har_says_so() {
        let dir = temp("bad");
        let error = import(
            &b"{\"not\":1}"[..],
            &dir.join("s"),
            "x",
            &StorageSettings::default(),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("not a HAR"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn versions_read_alike() {
        assert_eq!(version_of("h2"), "HTTP/2.0");
        assert_eq!(version_of("http/2.0"), "HTTP/2.0");
        assert_eq!(version_of("HTTP/1.1"), "HTTP/1.1");
        assert_eq!(version_of(""), "HTTP/1.1");
    }
}
