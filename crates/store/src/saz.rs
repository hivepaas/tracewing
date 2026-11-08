//! Fiddler's `.saz`: a zip with each session's request and response as they
//! went over the wire, in `raw/1_c.txt` and `raw/1_s.txt`, and what Fiddler saw
//! of it in `raw/1_m.xml`: when each step happened, and flags such as the
//! server's address and the program that sent it.
//!
//! Fiddler also keeps each tunnel it opened for HTTPS as a session; those whose
//! flows it read are left out, as the proxy here does not keep them either.

use crate::{
    graphql,
    import::{
        self, Connections, Imported, address, encoding_kept, header_lines, host_of, unix_micros,
        version_of,
    },
    settings::StorageSettings,
    store::FlowRecord,
};
use anyhow::{Context, Result, bail};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::{Read, Seek},
    path::Path,
};

/// Reads a `.saz` into `session`, a directory that must not hold one yet, and
/// names the session `name`.
pub fn import(
    file: impl Read + Seek,
    session: &Path,
    name: &str,
    settings: &StorageSettings,
) -> Result<Imported> {
    let mut zip = zip::ZipArchive::new(file).context("this is not a Fiddler session")?;
    let mut parts: BTreeMap<u64, Parts> = BTreeMap::new();
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .context("the session could not be unpacked")?;
        let name = entry.name().map(|n| n.to_string()).unwrap_or_default();
        let Some((number, part)) = name
            .strip_prefix("raw/")
            .and_then(|file| file.split_once('_'))
            .and_then(|(number, part)| Some((number.parse::<u64>().ok()?, part.to_owned())))
        else {
            continue;
        };
        let slot = parts.entry(number).or_default();
        let slot = match part.as_str() {
            "c.txt" => &mut slot.request,
            "s.txt" => &mut slot.response,
            "m.xml" => &mut slot.meta,
            _ => continue,
        };
        let mut bytes = Vec::with_capacity(entry.size().min(64 << 20) as usize);
        entry
            .read_to_end(&mut bytes)
            .context("the session could not be unpacked")?;
        *slot = Some(bytes);
    }
    if parts.is_empty() {
        bail!("this zip has no Fiddler sessions in it");
    }

    let sessions: Vec<Option<Session>> = parts.into_values().map(Session::read).collect();
    // Tunnels to hosts whose flows were read tell nothing more.
    let read_through: HashSet<String> = sessions
        .iter()
        .flatten()
        .filter(|s| !s.is_tunnel() && s.scheme == "https")
        .map(|s| s.host.clone())
        .collect();
    let mut connections = Connections::default();
    let mut last_start = 0;
    let records = sessions
        .into_iter()
        .filter_map(|session| match session {
            Some(s) if s.is_tunnel() && read_through.contains(&s.host) => None,
            Some(s) => Some(Some(s.record(&mut connections, &mut last_start))),
            None => Some(None),
        })
        .collect();
    import::write(records, session, name, settings)
}

#[derive(Default)]
struct Parts {
    request: Option<Vec<u8>>,
    response: Option<Vec<u8>>,
    meta: Option<Vec<u8>>,
}

/// A request or a response as it went over the wire.
struct Message {
    first: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Message {
    fn parse(raw: &[u8]) -> Option<Self> {
        let (head, body) = match find(raw, b"\r\n\r\n") {
            Some(at) => (&raw[..at], &raw[at + 4..]),
            None => match find(raw, b"\n\n") {
                Some(at) => (&raw[..at], &raw[at + 2..]),
                None => (raw, &[][..]),
            },
        };
        let head = String::from_utf8_lossy(head);
        let mut lines = head.lines();
        let first = lines.next()?.trim().to_owned();
        if first.is_empty() {
            return None;
        }
        let headers: Vec<(String, String)> = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
            .collect();
        let mut message = Self {
            first,
            headers,
            body: body.to_vec(),
        };
        let chunked = message
            .header("transfer-encoding")
            .is_some_and(|v| v.to_ascii_lowercase().contains("chunked"));
        if chunked && let Some(body) = unchunk(&message.body) {
            message.body = body;
        }
        Some(message)
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn lines(&self) -> String {
        header_lines(self.headers.iter().map(|(n, v)| (n.as_str(), v.as_str())))
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The body without its chunk sizes. One cut short keeps what came.
fn unchunk(raw: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(raw.len());
    let mut rest = raw;
    loop {
        let end = find(rest, b"\r\n")?;
        let size = std::str::from_utf8(&rest[..end]).ok()?;
        let size = usize::from_str_radix(size.split(';').next()?.trim(), 16).ok()?;
        rest = &rest[end + 2..];
        if size == 0 {
            return Some(out);
        }
        if rest.len() < size {
            out.extend_from_slice(rest);
            return Some(out);
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size..];
        rest = rest.strip_prefix(b"\r\n").unwrap_or(rest);
    }
}

/// What Fiddler noted of a session.
#[derive(Default)]
struct Meta {
    timers: HashMap<String, String>,
    /// By their names in lower case.
    flags: HashMap<String, String>,
    reused: bool,
}

impl Meta {
    fn parse(xml: &str) -> Self {
        let mut meta = Self::default();
        if let Some(timers) = tags(xml, "SessionTimers").into_iter().next() {
            meta.timers = timers.into_iter().collect();
        }
        meta.reused = tags(xml, "PipeInfo")
            .into_iter()
            .flatten()
            .any(|(n, v)| n == "Reused" && v.eq_ignore_ascii_case("true"));
        for flag in tags(xml, "SessionFlag") {
            let mut name = None;
            let mut value = String::new();
            for (n, v) in flag {
                match n.as_str() {
                    "N" => name = Some(v.to_ascii_lowercase()),
                    "V" => value = v,
                    _ => {}
                }
            }
            if let Some(name) = name {
                meta.flags.insert(name, value);
            }
        }
        meta
    }

    fn at(&self, timer: &str) -> Option<i64> {
        unix_micros(self.timers.get(timer)?)
    }

    /// One of the steps Fiddler times in whole milliseconds.
    fn ms(&self, timer: &str) -> Option<i64> {
        let ms: i64 = self.timers.get(timer)?.trim().parse().ok()?;
        (ms >= 0).then_some(ms * 1000)
    }

    fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .get(name)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
    }
}

/// The attributes of each `<name …>` tag in `xml`. Fiddler writes its notes
/// plainly, all in attributes, so this is all of XML they need.
fn tags(xml: &str, name: &str) -> Vec<Vec<(String, String)>> {
    let open = format!("<{name}");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find(&open) {
        let after = &rest[at + open.len()..];
        let Some(end) = after.find('>') else {
            break;
        };
        // `<SessionFlags>` starts as `<SessionFlag` does.
        if after.starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>') {
            out.push(attributes(&after[..end]));
        }
        rest = &after[end..];
    }
    out
}

fn attributes(tag: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = tag;
    while let Some(eq) = rest.find('=') {
        let name = rest[..eq]
            .split_whitespace()
            .last()
            .unwrap_or_default()
            .to_owned();
        let after = rest[eq + 1..].trim_start();
        let Some(quote) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            break;
        };
        let after = &after[1..];
        let Some(end) = after.find(quote) else {
            break;
        };
        out.push((name, unescape(&after[..end])));
        rest = &after[end + 1..];
    }
    out
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let entity = after.find(';').filter(|end| *end <= 10).and_then(|end| {
            let name = &after[..end];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => match name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                    Some(hex) => u32::from_str_radix(hex, 16).ok().and_then(char::from_u32),
                    None => name
                        .strip_prefix('#')
                        .and_then(|n| n.parse().ok())
                        .and_then(char::from_u32),
                },
            };
            c.map(|c| (c, end))
        });
        match entity {
            Some((c, end)) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A session read, before it is a flow.
struct Session {
    method: String,
    scheme: String,
    host: String,
    path: String,
    port: u16,
    version: String,
    request: Message,
    response: Option<Message>,
    meta: Meta,
}

impl Session {
    fn read(parts: Parts) -> Option<Self> {
        let request = Message::parse(parts.request.as_deref()?)?;
        let response = parts.response.as_deref().and_then(Message::parse);
        let meta = parts
            .meta
            .as_deref()
            .map(|xml| Meta::parse(&String::from_utf8_lossy(xml)))
            .unwrap_or_default();
        let mut words = request.first.split_whitespace();
        let method = words.next()?.to_ascii_uppercase();
        let target = words.next()?;
        let version = version_of(words.next().unwrap_or_default());

        if method == "CONNECT" {
            let uri: http::Uri = format!("https://{target}").parse().ok()?;
            let authority = uri.authority()?;
            return Some(Self {
                method,
                scheme: "https".to_owned(),
                host: host_of(authority.as_str(), "https"),
                path: String::new(),
                port: authority.port_u16().unwrap_or(443),
                version,
                request,
                response,
                meta,
            });
        }
        let uri: http::Uri = if target.contains("://") {
            target.parse().ok()?
        } else {
            // Only the path: the host is in its header, and HTTPS in Fiddler's notes.
            let https = meta.flags.keys().any(|k| k.starts_with("https-"));
            let scheme = if https { "https" } else { "http" };
            format!("{scheme}://{}{target}", request.header("host")?)
                .parse()
                .ok()?
        };
        let scheme = match uri.scheme_str()?.to_ascii_lowercase().as_str() {
            "https" | "wss" => "https",
            "http" | "ws" => "http",
            _ => return None,
        }
        .to_owned();
        let authority = uri.authority()?;
        Some(Self {
            method,
            host: host_of(authority.as_str(), &scheme),
            port: authority
                .port_u16()
                .unwrap_or(if scheme == "https" { 443 } else { 80 }),
            scheme,
            path: uri.path_and_query().map_or("/", |p| p.as_str()).to_owned(),
            version,
            request,
            response,
            meta,
        })
    }

    fn is_tunnel(&self) -> bool {
        self.method == "CONNECT"
    }

    fn record(self, connections: &mut Connections, last_start: &mut i64) -> FlowRecord {
        let tunnel = self.is_tunnel();
        let meta = &self.meta;
        let started = meta
            .at("ClientBeginRequest")
            .or_else(|| meta.at("ClientConnected"))
            .unwrap_or(*last_start);
        *last_start = started;
        let since = |timer: &str| meta.at(timer).map(|at| (at - started).max(0));

        // Fiddler answers for a server it could not reach, in a page of its own.
        let mut response = self.response;
        let error = response.as_ref().and_then(|r| {
            let text = String::from_utf8_lossy(&r.body[..r.body.len().min(4096)]).into_owned();
            text.starts_with("[Fiddler] ")
                .then(|| text.lines().next().unwrap_or_default().trim().to_owned())
        });
        if error.is_some() {
            response = None;
        }
        let error = error.or_else(|| {
            response
                .is_none()
                .then(|| "No response was saved".to_owned())
        });
        let status = response
            .as_ref()
            .and_then(|r| r.first.split_whitespace().nth(1)?.parse::<u16>().ok());

        let made = !meta.reused && meta.ms("TCPConnectTime").is_some_and(|us| us > 0);
        let server = meta.flag("x-hostip");
        let key = server
            .zip(meta.flag("x-egressport"))
            .map(|(ip, port)| format!("{ip} {port}"));
        let conn = connections.over(key);
        let remote = server.and_then(|ip| address(ip, self.port));
        let connection = match (&conn, &remote) {
            (Some((_, true)), Some(remote)) => {
                Some(serde_json::to_vec(&json!({ "remote": remote })).unwrap_or_default())
            }
            _ => None,
        };
        let client = meta
            .flag("x-clientip")
            .zip(meta.flag("x-clientport").and_then(|p| p.parse().ok()))
            .and_then(|(ip, port)| address(ip, port))
            .unwrap_or_else(|| "Fiddler".to_owned());
        // `chrome:1234`.
        let app = meta.flag("x-processinfo").and_then(|info| {
            let (name, pid) = info.rsplit_once(':').unwrap_or((info, "0"));
            import::app(name, pid.trim().parse().unwrap_or(0))
        });

        let req_content_type = self.request.header("content-type").map(str::to_owned);
        let req_encoding =
            encoding_kept(&self.request.body, self.request.header("content-encoding"));
        let graphql = match req_encoding {
            None => graphql::operation(
                &self.method,
                &self.path,
                req_content_type.as_deref(),
                &self.request.body,
            ),
            Some(_) => None,
        };
        let websocket = status == Some(101)
            && self
                .request
                .header("upgrade")
                .is_some_and(|u| u.eq_ignore_ascii_case("websocket"));
        let kind = if tunnel {
            "tunnel"
        } else if websocket {
            "websocket"
        } else {
            "http"
        };
        let (content_type, resp_encoding, resp_headers, resp_body, server_version) = match response
        {
            Some(r) => (
                r.header("content-type").map(str::to_owned),
                encoding_kept(&r.body, r.header("content-encoding")),
                r.lines(),
                r.body,
                Some(version_of(
                    r.first.split_whitespace().next().unwrap_or_default(),
                )),
            ),
            None => (None, None, String::new(), Vec::new(), None),
        };
        FlowRecord {
            started_us: started,
            client,
            kind,
            method: self.method,
            scheme: self.scheme,
            host: self.host,
            path: self.path,
            client_version: self.version,
            server_version: status.and(server_version),
            status,
            content_type,
            req_content_type,
            req_encoding,
            resp_encoding,
            req_headers: self.request.lines(),
            resp_headers,
            graphql,
            conn_id: conn.map(|(id, _)| id),
            blocked_us: made.then_some(0),
            dns_us: made.then(|| meta.ms("DNSTime")).flatten(),
            connect_us: made.then(|| meta.ms("TCPConnectTime")).flatten(),
            tls_us: made
                .then(|| meta.ms("HTTPSHandshakeTime"))
                .flatten()
                .filter(|us| *us > 0),
            connection,
            app,
            ttfb_us: status.and(since("ServerBeginResponse")),
            duration_us: since("ClientDoneResponse").or_else(|| since("ServerDoneResponse")),
            request: import::body(self.request.body),
            response: import::body(resp_body),
            error,
            ..FlowRecord::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{har, session};
    use rusqlite::Connection;
    use serde_json::Value;
    use std::io::Write;

    /// Kind, method, host, path, status, client, app and encoding.
    type Row = (
        String,
        String,
        String,
        String,
        Option<u16>,
        String,
        Option<String>,
        Option<String>,
    );

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tracewing-saz-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn saz(files: &[(&str, &[u8])]) -> std::io::Cursor<Vec<u8>> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        let mut out = zip.finish().unwrap();
        out.set_position(0);
        out
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(bytes).unwrap();
        gz.finish().unwrap()
    }

    const META_FIRST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Session SID="1" BitFlags="0">
  <SessionTimers ClientConnected="2026-11-07T17:00:00.9000000+07:00" ClientBeginRequest="2026-11-07T17:00:01.0000000+07:00" GotRequestHeaders="2026-11-07T17:00:01.0010000+07:00" ClientDoneRequest="2026-11-07T17:00:01.0020000+07:00" GatewayTime="0" DNSTime="3" TCPConnectTime="12" HTTPSHandshakeTime="25" ServerConnected="2026-11-07T17:00:01.0400000+07:00" FiddlerBeginRequest="2026-11-07T17:00:01.0410000+07:00" ServerGotRequest="2026-11-07T17:00:01.0420000+07:00" ServerBeginResponse="2026-11-07T17:00:01.0900000+07:00" GotResponseHeaders="2026-11-07T17:00:01.0910000+07:00" ServerDoneResponse="2026-11-07T17:00:01.0950000+07:00" ClientBeginResponse="2026-11-07T17:00:01.0960000+07:00" ClientDoneResponse="2026-11-07T17:00:01.1000000+07:00" />
  <PipeInfo />
  <SessionFlags>
    <SessionFlag N="x-clientip" V="127.0.0.1" />
    <SessionFlag N="x-clientport" V="51234" />
    <SessionFlag N="x-hostip" V="10.0.0.5" />
    <SessionFlag N="x-egressport" V="60001" />
    <SessionFlag N="x-ProcessInfo" V="chrome:4242" />
    <SessionFlag N="ui-comments" V="a &amp; b &#x263A;" />
    <SessionFlag N="https-Client-Version" V="Tls12" />
  </SessionFlags>
</Session>"#;

    const META_SECOND: &str = r#"<Session SID="2"><SessionTimers ClientBeginRequest="2026-11-07T17:00:02.0000000+07:00" DNSTime="0" TCPConnectTime="0" HTTPSHandshakeTime="0" ServerBeginResponse="2026-11-07T17:00:02.0300000+07:00" ClientDoneResponse="2026-11-07T17:00:02.0400000+07:00" /><PipeInfo CltReuse="true" Reused="true" /><SessionFlags><SessionFlag N="x-clientip" V="192.168.1.20" /><SessionFlag N="x-clientport" V="40000" /><SessionFlag N="x-hostip" V="10.0.0.5" /><SessionFlag N="x-egressport" V="60001" /></SessionFlags></Session>"#;

    #[test]
    fn a_saz_is_read_as_its_flows() {
        let dir = temp("read");
        let session = dir.join("imported");
        let packed = gzip(br#"{"me":null}"#);
        let mut first_response =
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\nTransfer-Encoding: chunked\r\n\r\n"
                .to_vec();
        let (a, b) = packed.split_at(5);
        for chunk in [a, b] {
            first_response.extend(format!("{:x}\r\n", chunk.len()).as_bytes());
            first_response.extend(chunk);
            first_response.extend(b"\r\n");
        }
        first_response.extend(b"0\r\n\r\n");
        let file = saz(&[
            ("[Content_Types].xml", b"<Types/>"),
            ("_index.htm", b"<html/>"),
            ("raw/01_c.txt", b"CONNECT api.test:443 HTTP/1.1\r\nHost: api.test:443\r\n\r\n"),
            ("raw/01_s.txt", b"HTTP/1.1 200 Connection Established\r\n\r\n"),
            ("raw/02_c.txt", b"POST https://api.test/graphql?x=1 HTTP/1.1\r\nHost: api.test\r\nContent-Type: application/json\r\n\r\n{\"query\":\"query Me { me { id } }\"}"),
            ("raw/02_s.txt", &first_response),
            ("raw/02_m.xml", META_FIRST.as_bytes()),
            // Only the path, from a phone on the network, over the same connection.
            ("raw/03_c.txt", b"GET /a.png HTTP/1.1\r\nHost: plain.test:8080\r\n\r\n"),
            ("raw/03_s.txt", b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"),
            ("raw/03_m.xml", META_SECOND.as_bytes()),
            ("raw/04_c.txt", b"GET http://down.test/ HTTP/1.1\r\nHost: down.test\r\n\r\n"),
            ("raw/04_s.txt", b"HTTP/1.1 502 Fiddler - DNS Lookup Failed\r\nContent-Type: text/html\r\n\r\n[Fiddler] DNS Lookup for \"down.test\" failed.\r\n<html/>"),
            ("raw/05_c.txt", b"CONNECT other.test:443 HTTP/1.1\r\n\r\n"),
            ("raw/05_s.txt", b"HTTP/1.1 200 Connection Established\r\n\r\n"),
            ("raw/06_c.txt", b"\r\n"),
        ]);
        let imported = import(file, &session, "From Fiddler", &StorageSettings::default()).unwrap();
        assert_eq!(
            imported,
            Imported {
                flows: 4,
                skipped: 1
            }
        );
        assert_eq!(session::name_of(&session).as_deref(), Some("From Fiddler"));

        let flows = Connection::open(session::flows_db(&session)).unwrap();
        let rows: Vec<Row> = flows
            .prepare("SELECT kind, method, host, path, status, client, app, resp_encoding FROM flows ORDER BY id")
            .unwrap()
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        // Sessions without times come after the one before them.
        assert_eq!(
            rows,
            [
                (
                    "http".into(),
                    "POST".into(),
                    "api.test".into(),
                    "/graphql?x=1".into(),
                    Some(200),
                    "127.0.0.1:51234".into(),
                    Some("chrome".into()),
                    Some("gzip".into())
                ),
                (
                    "http".into(),
                    "GET".into(),
                    "plain.test:8080".into(),
                    "/a.png".into(),
                    Some(404),
                    "192.168.1.20:40000".into(),
                    None,
                    None
                ),
                (
                    "http".into(),
                    "GET".into(),
                    "down.test".into(),
                    "/".into(),
                    None,
                    "Fiddler".into(),
                    None,
                    None
                ),
                (
                    "tunnel".into(),
                    "CONNECT".into(),
                    "other.test".into(),
                    "".into(),
                    Some(200),
                    "Fiddler".into(),
                    None,
                    None
                ),
            ]
        );
        let error: Option<String> = flows
            .query_row("SELECT error FROM flows WHERE id = 3", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            error.as_deref(),
            Some(r#"[Fiddler] DNS Lookup for "down.test" failed."#)
        );
        drop(flows);

        let mut out = Vec::new();
        har::export(&session, Some(&[1, 2]), Default::default(), &mut out).unwrap();
        let har: Value = serde_json::from_slice(&out).unwrap();
        let first = &har["log"]["entries"][0];
        assert_eq!(first["startedDateTime"], "2026-11-07T10:00:01Z");
        assert_eq!(first["time"], 100.0);
        assert_eq!(
            first["request"]["postData"]["text"],
            r#"{"query":"query Me { me { id } }"}"#
        );
        // Unchunked and unpacked.
        assert_eq!(first["response"]["content"]["text"], r#"{"me":null}"#);
        assert_eq!(first["serverIPAddress"], "10.0.0.5");
        let timings = &first["timings"];
        assert_eq!(
            [
                &timings["dns"],
                &timings["connect"],
                &timings["ssl"],
                &timings["wait"]
            ],
            [3.0, 37.0, 25.0, 50.0]
        );
        let second = &har["log"]["entries"][1];
        assert_eq!(second["request"]["url"], "http://plain.test:8080/a.png");
        assert_eq!(second["timings"]["connect"], -1.0);
        assert_eq!(second["connection"], first["connection"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn what_is_not_a_saz_says_so() {
        let dir = temp("bad");
        let not_zip = import(
            std::io::Cursor::new(b"PK\x03\x04 but no more".to_vec()),
            &dir.join("a"),
            "x",
            &StorageSettings::default(),
        )
        .unwrap_err();
        assert!(
            format!("{not_zip:#}").contains("not a Fiddler session"),
            "{not_zip:#}"
        );
        let empty = import(
            saz(&[("other.txt", b"hi")]),
            &dir.join("b"),
            "x",
            &StorageSettings::default(),
        )
        .unwrap_err();
        assert!(
            format!("{empty:#}").contains("no Fiddler sessions"),
            "{empty:#}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn notes_are_read_from_their_attributes() {
        let tags = tags(
            r#"<SessionFlags><SessionFlag N="a" V='x &lt;&amp;&gt; &#65;&#x42;' /><SessionFlag N="b" V="&bogus; &" /></SessionFlags>"#,
            "SessionFlag",
        );
        assert_eq!(
            tags,
            [
                vec![
                    ("N".to_owned(), "a".to_owned()),
                    ("V".to_owned(), "x <&> AB".to_owned())
                ],
                vec![
                    ("N".to_owned(), "b".to_owned()),
                    ("V".to_owned(), "&bogus; &".to_owned())
                ],
            ]
        );
    }

    #[test]
    fn chunks_are_joined() {
        assert_eq!(
            unchunk(b"3\r\nabc\r\n2;x=y\r\nde\r\n0\r\n\r\n").unwrap(),
            b"abcde"
        );
        assert_eq!(unchunk(b"5\r\nab").unwrap(), b"ab");
        assert_eq!(unchunk(b"zz\r\n"), None);
    }
}
