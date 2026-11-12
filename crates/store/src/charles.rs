//! Charles's JSON session (`.chlsj`), as File, Export Session writes it: a list
//! of flows, each with its request and response, when each step happened, and
//! the TLS the server spoke. Charles's own `.chls` is Java's and not read; it
//! saves to this, or to a HAR.

use crate::{
    graphql,
    import::{
        self, Connections, Imported, address, encoding_kept, header_lines, host_of, unix_micros,
        version_of,
    },
    jsonrpc,
    settings::StorageSettings,
    store::FlowRecord,
};
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashSet, io::Read, path::Path};

/// Reads a Charles JSON session into `session`, a directory that must not hold
/// one yet, and names the session `name`.
pub fn import(
    file: impl Read,
    session: &Path,
    name: &str,
    settings: &StorageSettings,
) -> Result<Imported> {
    let mut entries: Vec<Entry> = serde_json::from_reader(std::io::BufReader::new(file))
        .context("this is not a Charles JSON session")?;
    entries.sort_by_cached_key(|e| {
        e.times
            .as_ref()
            .and_then(|t| moment(t.start.as_ref()))
            .unwrap_or(0)
    });
    // Tunnels to hosts whose flows were read tell nothing more.
    let read_through: HashSet<String> = entries
        .iter()
        .filter(|e| !e.is_tunnel())
        .filter_map(|e| Some(host_of(&e.authority()?, "https")))
        .collect();
    let mut connections = Connections::default();
    let records = entries
        .into_iter()
        .filter(|e| {
            !(e.is_tunnel()
                && e.authority()
                    .is_some_and(|a| read_through.contains(&host_of(&a, "https"))))
        })
        .map(|entry| entry.record(&mut connections))
        .collect();
    import::write(records, session, name, settings)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    status: Option<String>,
    method: Option<String>,
    protocol_version: Option<String>,
    scheme: Option<String>,
    host: Option<String>,
    actual_port: Option<i64>,
    port: Option<i64>,
    path: Option<String>,
    query: Option<String>,
    #[serde(default)]
    tunnel: bool,
    #[serde(default)]
    kept_alive: bool,
    #[serde(default)]
    web_socket: bool,
    /// `host/1.2.3.4`.
    remote_address: Option<String>,
    /// `/127.0.0.1`.
    client_address: Option<String>,
    client_port: Option<i64>,
    times: Option<Times>,
    durations: Option<Durations>,
    ssl: Option<Ssl>,
    alpn: Option<Alpn>,
    request: Option<Side>,
    response: Option<Side>,
    error_message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Times {
    start: Option<Value>,
    response_begin: Option<Value>,
    end: Option<Value>,
}

/// In milliseconds.
#[derive(Deserialize, Default)]
struct Durations {
    total: Option<f64>,
    dns: Option<f64>,
    connect: Option<f64>,
    ssl: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ssl {
    protocol: Option<String>,
    cipher_suite: Option<String>,
}

#[derive(Deserialize)]
struct Alpn {
    protocol: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Side {
    status: Option<i64>,
    mime_type: Option<String>,
    charset: Option<String>,
    content_encoding: Option<String>,
    header: Option<Head>,
    body: Option<Body>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Head {
    first_line: Option<String>,
    #[serde(default)]
    headers: Vec<Pair>,
}

#[derive(Deserialize)]
struct Pair {
    name: String,
    #[serde(default)]
    value: String,
}

#[derive(Deserialize)]
struct Body {
    /// As text, unpacked when `decoded`.
    text: Option<String>,
    /// In base64, as it went.
    encoded: Option<String>,
    decoded: Option<bool>,
}

/// A time as text, or as milliseconds since 1970.
fn moment(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::String(text) => unix_micros(text),
        Value::Number(ms) => ms.as_f64().map(|ms| (ms * 1000.0).round() as i64),
        _ => None,
    }
}

fn us(ms: Option<f64>) -> Option<i64> {
    ms.filter(|ms| *ms >= 0.0)
        .map(|ms| (ms * 1000.0).round() as i64)
}

/// The address after the name, as Java writes one: `host/1.2.3.4`.
fn ip_of(text: &str) -> &str {
    text.rsplit('/').next().unwrap_or(text)
}

impl Side {
    fn headers(&self) -> &[Pair] {
        self.header.as_ref().map_or(&[], |h| &h.headers)
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers()
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
            .map(|p| p.value.as_str())
    }

    fn first_line(&self) -> &str {
        self.header
            .as_ref()
            .and_then(|h| h.first_line.as_deref())
            .unwrap_or_default()
    }

    fn content_type(&self) -> Option<String> {
        self.header("content-type").map(str::to_owned).or_else(|| {
            let mime = self.mime_type.as_deref().filter(|m| !m.is_empty())?;
            Some(match &self.charset {
                Some(charset) if !charset.is_empty() => format!("{mime}; charset={charset}"),
                _ => mime.to_owned(),
            })
        })
    }

    /// The body, and how it is packed.
    fn body(&self) -> (Vec<u8>, Option<String>) {
        let Some(body) = &self.body else {
            return (Vec::new(), None);
        };
        let bytes = match (&body.encoded, &body.text) {
            (Some(encoded), _) => STANDARD
                .decode(encoded.trim())
                .unwrap_or_else(|_| encoded.as_bytes().to_vec()),
            (None, Some(text)) => text.as_bytes().to_vec(),
            (None, None) => Vec::new(),
        };
        let header = self
            .header("content-encoding")
            .or(self.content_encoding.as_deref());
        let encoding = match body.decoded {
            Some(true) => None,
            _ => encoding_kept(&bytes, header),
        };
        (bytes, encoding)
    }

    fn lines(&self) -> String {
        header_lines(
            self.headers()
                .iter()
                .map(|p| (p.name.as_str(), p.value.as_str())),
        )
    }
}

impl Entry {
    fn is_tunnel(&self) -> bool {
        self.tunnel
            || self
                .method
                .as_deref()
                .is_some_and(|m| m.eq_ignore_ascii_case("CONNECT"))
    }

    fn scheme(&self) -> String {
        let scheme = self
            .scheme
            .as_deref()
            .unwrap_or("http")
            .to_ascii_lowercase();
        match scheme.as_str() {
            "wss" => "https".to_owned(),
            "ws" => "http".to_owned(),
            _ if self.is_tunnel() => "https".to_owned(),
            _ => scheme,
        }
    }

    fn port(&self) -> Option<u16> {
        self.actual_port
            .or(self.port)
            .and_then(|p| u16::try_from(p).ok())
            .filter(|p| *p > 0)
    }

    fn authority(&self) -> Option<String> {
        let host = self.host.as_deref().filter(|h| !h.is_empty())?;
        Some(match self.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_owned(),
        })
    }

    fn record(self, connections: &mut Connections) -> Option<FlowRecord> {
        let scheme = self.scheme();
        if scheme != "http" && scheme != "https" {
            return None;
        }
        let host = host_of(&self.authority()?, &scheme);
        let tunnel = self.is_tunnel();
        let method = self.method.as_deref().unwrap_or("GET").to_ascii_uppercase();
        let path = if tunnel {
            String::new()
        } else {
            let path = self
                .path
                .as_deref()
                .filter(|p| !p.is_empty())
                .unwrap_or("/");
            match self.query.as_deref().filter(|q| !q.is_empty()) {
                Some(query) => format!("{path}?{query}"),
                None => path.to_owned(),
            }
        };
        let times = self.times.as_ref();
        let started = moment(times?.start.as_ref())?;
        let since = |at: Option<&Value>| moment(at).map(|at| (at - started).max(0));

        let request = self.request.as_ref();
        let response = self
            .response
            .as_ref()
            .filter(|r| r.header.is_some() || r.status.is_some_and(|s| s > 0));
        let status = response.and_then(|r| {
            r.status
                .and_then(|s| u16::try_from(s).ok())
                .filter(|s| *s > 0)
                .or_else(|| r.first_line().split_whitespace().nth(1)?.parse().ok())
        });
        let alpn = self
            .alpn
            .as_ref()
            .and_then(|a| a.protocol.clone())
            .filter(|p| !p.is_empty());
        let spoken = |line: &str, at: usize| {
            line.split_whitespace()
                .nth(at)
                .map(version_of)
                .or_else(|| self.protocol_version.as_deref().map(version_of))
                .unwrap_or_else(|| version_of(""))
        };
        let client_version = spoken(request.map_or("", Side::first_line), 2);
        let server_version = status.map(|_| match alpn.as_deref() {
            Some("h2") => "HTTP/2.0".to_owned(),
            _ => spoken(response.map_or("", Side::first_line), 0),
        });

        let port = self
            .port()
            .unwrap_or(if scheme == "https" { 443 } else { 80 });
        let remote = self
            .remote_address
            .as_deref()
            .and_then(|text| address(ip_of(text), port));
        let client = self
            .client_address
            .as_deref()
            .zip(self.client_port.and_then(|p| u16::try_from(p).ok()))
            .and_then(|(ip, port)| address(ip_of(ip), port));
        let key = remote
            .as_ref()
            .map(|remote| format!("{} {remote}", client.as_deref().unwrap_or_default()));
        let conn = connections.over(key);
        let connection = match (&conn, &remote) {
            (Some((_, true)), Some(remote)) => {
                let mut facts = json!({ "remote": remote });
                if let Some(ssl) = &self.ssl {
                    facts["tls"] = json!({
                        "version": ssl.protocol.as_deref().unwrap_or_default(),
                        "cipher": ssl.cipher_suite.as_deref().unwrap_or_default(),
                        "alpn": alpn,
                        "certificates": [],
                    });
                }
                Some(serde_json::to_vec(&facts).unwrap_or_default())
            }
            _ => None,
        };
        let durations = self.durations.unwrap_or_default();
        let made = !self.kept_alive && us(durations.connect).is_some();

        let (req_body, req_encoding) = request.map(Side::body).unwrap_or_default();
        let (resp_body, resp_encoding) = response.map(Side::body).unwrap_or_default();
        let req_content_type = request.and_then(Side::content_type);
        let resp_content_type = response.and_then(Side::content_type);
        let graphql = match req_encoding {
            None => graphql::operation(&method, &path, req_content_type.as_deref(), &req_body),
            Some(_) => None,
        };
        let rpc = match req_encoding {
            None => jsonrpc::method(&method, req_content_type.as_deref(), &req_body),
            Some(_) => None,
        };
        // Only a reply to a call that was JSON-RPC is looked at, so that no
        // other JSON is parsed a second time.
        let rpc_error = match (rpc.is_some(), &resp_encoding) {
            (true, None) => jsonrpc::faults(resp_content_type.as_deref(), &resp_body)
                .first()
                .map(|fault| fault.code),
            _ => None,
        };
        let failed = self
            .status
            .as_deref()
            .is_some_and(|s| s.eq_ignore_ascii_case("FAILED"));
        let error = self
            .error_message
            .clone()
            .filter(|e| !e.is_empty())
            .or_else(|| {
                (failed || status.is_none() && !tunnel).then(|| "No response was saved".to_owned())
            });
        Some(FlowRecord {
            started_us: started,
            client: client.unwrap_or_else(|| "Charles".to_owned()),
            kind: if tunnel {
                "tunnel"
            } else if self.web_socket {
                "websocket"
            } else {
                "http"
            },
            method,
            scheme,
            host,
            path,
            client_version,
            server_version,
            status: status.or(tunnel.then_some(200)),
            content_type: resp_content_type,
            req_content_type,
            req_encoding,
            resp_encoding,
            req_headers: request.map(Side::lines).unwrap_or_default(),
            resp_headers: response.map(Side::lines).unwrap_or_default(),
            graphql,
            rpc,
            rpc_error,
            conn_id: conn.map(|(id, _)| id),
            blocked_us: made.then_some(0),
            dns_us: made.then(|| us(durations.dns)).flatten(),
            connect_us: made.then(|| us(durations.connect)).flatten(),
            tls_us: made
                .then(|| us(durations.ssl))
                .flatten()
                .filter(|us| *us > 0),
            connection,
            ttfb_us: status.and(since(times.and_then(|t| t.response_begin.as_ref()))),
            duration_us: us(durations.total).or_else(|| since(times.and_then(|t| t.end.as_ref()))),
            request: import::body(req_body),
            response: import::body(resp_body),
            error,
            ..FlowRecord::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{har, session};
    use rusqlite::Connection;
    use std::io::Write;

    /// Kind, host, path, status, client, encoding and error.
    type Row = (
        String,
        String,
        String,
        Option<u16>,
        String,
        Option<String>,
        Option<String>,
    );

    fn temp(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tracewing-charles-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample() -> String {
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(b"<p>packed</p>").unwrap();
        let packed = STANDARD.encode(gz.finish().unwrap());
        format!(
            r#"[
  {{"status":"COMPLETE","method":"POST","protocolVersion":"HTTP/1.1","scheme":"https","host":"api.test","actualPort":443,
    "path":"/graphql","query":"x=1","tunnel":false,"keptAlive":false,"webSocket":false,
    "remoteAddress":"api.test/10.0.0.5","clientAddress":"/127.0.0.1","clientPort":52000,
    "times":{{"start":"2026-11-07T17:00:01.000+07:00","requestBegin":"2026-11-07T17:00:01.040+07:00",
      "requestComplete":"2026-11-07T17:00:01.041+07:00","responseBegin":"2026-11-07T17:00:01.090+07:00","end":"2026-11-07T17:00:01.100+07:00"}},
    "durations":{{"total":100,"dns":3,"connect":12,"ssl":25,"request":1,"response":10,"latency":49}},
    "ssl":{{"protocol":"TLSv1.3","cipherSuite":"TLS_AES_128_GCM_SHA256"}},"alpn":{{"protocol":"h2"}},
    "request":{{"sizes":{{"headers":100,"body":34}},"mimeType":"application/json","charset":null,"contentEncoding":null,
      "header":{{"firstLine":"POST /graphql?x=1 HTTP/1.1","headers":[{{"name":"Host","value":"api.test"}},{{"name":"Content-Type","value":"application/json"}}]}},
      "body":{{"text":"{{\"query\":\"query Me {{ me {{ id }} }}\"}}","charset":"UTF-8"}}}},
    "response":{{"status":200,"sizes":{{"headers":80,"body":11}},"mimeType":"application/json","charset":"UTF-8","contentEncoding":"gzip",
      "header":{{"firstLine":"HTTP/1.1 200 OK","headers":[{{"name":"Content-Type","value":"application/json"}},{{"name":"Content-Encoding","value":"gzip"}}]}},
      "body":{{"text":"{{\"me\":null}}","charset":"UTF-8","decoded":true}}}}}},
  {{"status":"COMPLETE","method":"GET","protocolVersion":"HTTP/1.1","scheme":"http","host":"plain.test","actualPort":8080,
    "path":"/page","query":null,"tunnel":false,"keptAlive":true,
    "remoteAddress":"plain.test/10.0.0.6","clientAddress":"/192.168.1.20","clientPort":40000,
    "times":{{"start":1794045602000,"responseBegin":1794045602030,"end":1794045602040}},
    "durations":{{"total":null,"dns":null,"connect":null,"ssl":null}},
    "request":{{"header":{{"firstLine":"GET /page HTTP/1.1","headers":[{{"name":"Host","value":"plain.test:8080"}}]}}}},
    "response":{{"status":200,"mimeType":"text/html","charset":null,
      "header":{{"firstLine":"HTTP/1.1 200 OK","headers":[{{"name":"Content-Encoding","value":"gzip"}}]}},
      "body":{{"encoded":"{packed}"}}}}}},
  {{"status":"COMPLETE","method":"CONNECT","scheme":"https","host":"api.test","actualPort":443,"tunnel":true,
    "times":{{"start":"2026-11-07T17:00:00.500+07:00"}}}},
  {{"status":"COMPLETE","method":"CONNECT","scheme":"https","host":"pinned.test","actualPort":443,"tunnel":true,
    "times":{{"start":"2026-11-07T17:00:03.000+07:00"}}}},
  {{"status":"FAILED","method":"GET","scheme":"https","host":"down.test","actualPort":443,"path":"/",
    "times":{{"start":"2026-11-07T17:00:04.000+07:00"}},"errorMessage":"Connect failed: Connection refused",
    "request":{{"header":{{"firstLine":"GET / HTTP/1.1","headers":[]}}}},"response":{{"status":0}}}},
  {{"method":"GET","scheme":"https","host":"","path":"/"}}
]"#
        )
    }

    #[test]
    fn a_charles_session_is_read_as_its_flows() {
        let dir = temp("read");
        let session = dir.join("imported");
        let imported = import(
            sample().as_bytes(),
            &session,
            "From Charles",
            &StorageSettings::default(),
        )
        .unwrap();
        assert_eq!(
            imported,
            Imported {
                flows: 4,
                skipped: 1
            }
        );

        let flows = Connection::open(session::flows_db(&session)).unwrap();
        let rows: Vec<Row> = flows
            .prepare("SELECT kind, host, path, status, client, resp_encoding, error FROM flows ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            rows,
            [
                (
                    "http".into(),
                    "api.test".into(),
                    "/graphql?x=1".into(),
                    Some(200),
                    "127.0.0.1:52000".into(),
                    None,
                    None
                ),
                (
                    "http".into(),
                    "plain.test:8080".into(),
                    "/page".into(),
                    Some(200),
                    "192.168.1.20:40000".into(),
                    Some("gzip".into()),
                    None
                ),
                (
                    "tunnel".into(),
                    "pinned.test".into(),
                    "".into(),
                    Some(200),
                    "Charles".into(),
                    None,
                    None
                ),
                (
                    "http".into(),
                    "down.test".into(),
                    "/".into(),
                    None,
                    "Charles".into(),
                    None,
                    Some("Connect failed: Connection refused".into())
                ),
            ]
        );
        let graphql: Option<String> = flows
            .query_row("SELECT graphql FROM flows WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(graphql.as_deref(), Some("query Me"));
        drop(flows);

        let mut out = Vec::new();
        har::export(&session, Some(&[1, 2]), Default::default(), &mut out).unwrap();
        let har: Value = serde_json::from_slice(&out).unwrap();
        let first = &har["log"]["entries"][0];
        assert_eq!(first["startedDateTime"], "2026-11-07T10:00:01Z");
        assert_eq!(first["time"], 100.0);
        assert_eq!(first["response"]["httpVersion"], "HTTP/2.0");
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
        assert_eq!(second["startedDateTime"], "2026-11-07T10:00:02Z");
        assert_eq!(second["time"], 40.0);
        assert_eq!(second["response"]["content"]["text"], "<p>packed</p>");
        assert_eq!(second["timings"]["connect"], -1.0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn what_is_not_a_charles_session_says_so() {
        let dir = temp("bad");
        let error = import(
            &b"[1, 2]"[..],
            &dir.join("s"),
            "x",
            &StorageSettings::default(),
        )
        .unwrap_err();
        assert!(
            format!("{error:#}").contains("not a Charles JSON session"),
            "{error:#}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
