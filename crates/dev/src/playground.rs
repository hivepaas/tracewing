//! A server to try tracewing on by hand: each kind of body at a path of its
//! own, big ones too, and streams, WebSocket and gRPC. Plain HTTP, so that
//! nothing has to trust it; HTTP/2 to a client that speaks it from the start,
//! as gRPC does.
//!
//! `/` lists every path, with links.

use crate::{
    benchkit::{self, OriginBody},
    seed::Rng,
};
use anyhow::Context as _;
use anyhow::Result;
use bytes::Bytes;
use http::{HeaderValue, Request, Response, StatusCode, header};
use http_body_util::{BodyExt, Full, StreamBody};
use hudsucker::{
    futures::{SinkExt, StreamExt, stream},
    tokio_tungstenite::{
        WebSocketStream,
        tungstenite::{Message, handshake::derive_accept_key, protocol::Role},
    },
};
use hudsucker::{
    rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose},
    rustls::{
        ServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    },
};
use hyper::{body::Frame, body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use std::{
    convert::Infallible,
    fmt::Write as _,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpListener;

/// Where it listens unless told otherwise.
pub const PORT: u16 = 10000;

/// How big a `_big` body is, in MB, unless `?mb=` says otherwise.
const BIG_MB: usize = 10;
const BIG_MB_MAX: usize = 500;

/// Every path, and what it answers with: the list `/` shows.
pub const ROUTES: &[(&str, &str)] = &[
    ("/json", "JSON: a few orders"),
    ("/json_big", "JSON of 10 MB (?mb=N for another size)"),
    ("/xml", "XML: the same orders"),
    ("/xml_big", "XML of 10 MB (?mb=N)"),
    ("/soap", "A SOAP answer, all on one line"),
    (
        "/html",
        "An HTML page with a table, its CSS, JS and picture",
    ),
    ("/html_big", "HTML of 10 MB (?mb=N)"),
    ("/csv", "CSV, with quotes and commas inside cells"),
    ("/csv_big", "CSV of 10 MB (?mb=N)"),
    ("/tsv", "TSV"),
    ("/tsv_big", "TSV of 10 MB (?mb=N)"),
    ("/text", "Plain text: log lines"),
    ("/text_big", "Plain text of 10 MB (?mb=N)"),
    ("/js", "JavaScript"),
    ("/css", "CSS"),
    ("/svg", "An SVG picture"),
    ("/png", "A PNG picture"),
    ("/binary", "64 KB of bytes that are no format"),
    ("/msgpack", "MessagePack"),
    ("/cbor", "CBOR"),
    ("/protobuf", "Protobuf, with no .proto"),
    ("/grpc-web", "gRPC-Web: a message, then trailers"),
    (
        "/shop.proto",
        "The .proto the Protobuf and gRPC answers are made by",
    ),
    ("/gzip", "JSON packed with gzip"),
    ("/br", "JSON packed with Brotli"),
    ("/zstd", "JSON packed with zstd"),
    ("/sse", "Server-sent events, one each 500 ms (?n=10&ms=500)"),
    ("/ndjson", "A JSON line each 500 ms (?n=10&ms=500)"),
    ("/slow", "JSON after 3 s (?ms=N)"),
    ("/status/404", "Any status: /status/201, /status/500…"),
    ("/redirect", "Redirects 3 times, then JSON (?n=N)"),
    ("/cookies", "Sets cookies, and says which came"),
    ("/auth", "401 unless a header Authorization: Bearer … comes"),
    (
        "/hop",
        "Headers for the next hop alone, which a proxy must not pass on",
    ),
    ("/echo", "Any method: says what came, body and all"),
    ("/ws.html", "A page to talk to /ws"),
    (
        "/ws",
        "WebSocket: says what came, echoes, ticks each second (?ticks=N), \
         sends N bytes (?big=N), closes with a code (?close=4000), refuses (?refuse=403)",
    ),
    (
        "/shop.Orders/Get",
        "gRPC (HTTP/2, POST): the order asked for; 42 is not found",
    ),
    (
        "/rpc",
        "JSON-RPC (POST), as an Ethereum node answers; ?fail=1 reverts, in a 200",
    ),
];

/// Serves until the task is dropped.
pub async fn serve(listener: TcpListener) -> Result<()> {
    loop {
        let (tcp, _) = listener.accept().await?;
        tcp.set_nodelay(true)?;
        tokio::spawn(async move {
            let _ = Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(TokioIo::new(tcp), service_fn(route))
                .await;
        });
    }
}

/// Where it listens with TLS unless told otherwise.
pub const TLS_PORT: u16 = 10443;

/// The same paths over TLS, HTTP/2 to a client that offers it, with a
/// certificate for `localhost` and `127.0.0.1` from the authority in `dir`.
pub async fn serve_tls(listener: TcpListener, dir: &Path) -> Result<()> {
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls_config(dir)?));
    loop {
        let (tcp, _) = listener.accept().await?;
        tcp.set_nodelay(true)?;
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let Ok(tls) = acceptor.accept(tcp).await else {
                return;
            };
            let _ = Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(TokioIo::new(tls), service_fn(route))
                .await;
        });
    }
}

/// The playground's authority, made the first time and kept in `dir`, so
/// that it is trusted once: its certificate is `ca.pem` there.
pub fn authority(dir: &Path) -> Result<PathBuf> {
    let (cert, key) = (dir.join("ca.pem"), dir.join("ca-key.pem"));
    if cert.exists() && key.exists() {
        return Ok(cert);
    }
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let ca_key = KeyPair::generate()?;
    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, "tracewing playground CA");
    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca = params.self_signed(&ca_key)?;
    std::fs::write(&key, ca_key.serialize_pem())?;
    std::fs::write(&cert, ca.pem())?;
    Ok(cert)
}

fn tls_config(dir: &Path) -> Result<ServerConfig> {
    let cert = authority(dir)?;
    let ca_key = KeyPair::from_pem(&std::fs::read_to_string(dir.join("ca-key.pem"))?)?;
    let issuer = Issuer::from_ca_cert_pem(&std::fs::read_to_string(&cert)?, ca_key)?;
    let leaf_key = KeyPair::generate()?;
    let leaf = CertificateParams::new(vec!["localhost".to_owned(), "127.0.0.1".to_owned()])?
        .signed_by(&leaf_key, &issuer)?;
    let mut config =
        ServerConfig::builder_with_provider(Arc::new(tracewing_proxy::crypto_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(leaf.der().to_vec())],
                PrivateKeyDer::from(PrivatePkcs8KeyDer::from(leaf_key.serialize_der())),
            )?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

fn body(content_type: &str, data: impl Into<Bytes>) -> Response<OriginBody> {
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .body(Full::new(data.into()).boxed())
        .expect("a valid answer")
}

fn with_status(status: StatusCode, mut response: Response<OriginBody>) -> Response<OriginBody> {
    *response.status_mut() = status;
    response
}

async fn route(req: Request<Incoming>) -> Result<Response<OriginBody>, Infallible> {
    let path = req.uri().path().to_owned();
    let query: Vec<(String, String)> = benchkit::parse_query(req.uri().query().unwrap_or(""))
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    let number = |key: &str, default: u64| {
        query
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.parse().ok())
            .unwrap_or(default)
    };
    let big = || (number("mb", BIG_MB as u64) as usize).clamp(1, BIG_MB_MAX) * 1024 * 1024;

    if tracewing_store::reflection::PATHS.contains(&path.as_str()) {
        let data = read_body(req).await;
        return Ok(benchkit::reflection_answer(&data));
    }
    let response = match path.as_str() {
        "/" | "/index.html" => body("text/html; charset=utf-8", index()),
        "/json" => body("application/json", orders_json(0)),
        "/json_big" => body("application/json", orders_json(big())),
        "/xml" => body("application/xml; charset=utf-8", orders_xml(0)),
        "/xml_big" => body("application/xml; charset=utf-8", orders_xml(big())),
        "/soap" => body("text/xml; charset=utf-8", benchkit::SOAP_XML),
        "/html" => body("text/html; charset=utf-8", orders_html(0)),
        "/html_big" => body("text/html; charset=utf-8", orders_html(big())),
        "/csv" => body("text/csv; charset=utf-8", orders_table(0, ',')),
        "/csv_big" => body("text/csv; charset=utf-8", orders_table(big(), ',')),
        "/tsv" => body(
            "text/tab-separated-values; charset=utf-8",
            orders_table(0, '\t'),
        ),
        "/tsv_big" => body(
            "text/tab-separated-values; charset=utf-8",
            orders_table(big(), '\t'),
        ),
        "/text" => body("text/plain; charset=utf-8", log_text(0)),
        "/text_big" => body("text/plain; charset=utf-8", log_text(big())),
        "/js" | "/app.js" => body("text/javascript", benchkit::APP_JS),
        "/css" | "/style.css" => body("text/css", benchkit::STYLE_CSS),
        "/svg" => body("image/svg+xml", SVG),
        "/png" | "/picture.png" => body("image/png", benchkit::PICTURE),
        "/binary" => body("application/octet-stream", noise(64 * 1024)),
        "/msgpack" => body("application/msgpack", benchkit::order_msgpack()),
        "/cbor" => body("application/cbor", benchkit::order_cbor()),
        "/protobuf" => body("application/x-protobuf", benchkit::order_protobuf()),
        "/grpc-web" => body("application/grpc-web+proto", benchkit::order_grpc_web()),
        "/shop.proto" => body("text/plain; charset=utf-8", benchkit::SHOP_PROTO),
        "/gzip" | "/br" | "/zstd" => {
            let encoding = &path[1..];
            let mut response = body(
                "application/json",
                benchkit::pack(orders_json(64 * 1024).as_bytes(), encoding),
            );
            response.headers_mut().insert(
                header::CONTENT_ENCODING,
                HeaderValue::from_str(encoding).expect("a plain word"),
            );
            response
        }
        "/sse" | "/ndjson" => events(
            path == "/sse",
            number("n", 10),
            Duration::from_millis(number("ms", 500)),
        ),
        "/slow" => {
            let ms = number("ms", 3000).min(120_000);
            tokio::time::sleep(Duration::from_millis(ms)).await;
            body("application/json", format!(r#"{{"waited_ms":{ms}}}"#))
        }
        "/redirect" => {
            let n = number("n", 3).min(20);
            let to = if n > 1 {
                format!("/redirect?n={}", n - 1)
            } else {
                "/json".to_owned()
            };
            Response::builder()
                .status(StatusCode::FOUND)
                .header(header::LOCATION, to)
                .body(Full::new(Bytes::new()).boxed())
                .expect("a valid answer")
        }
        "/cookies" => cookies(&req),
        "/auth" => auth(&req),
        "/hop" => {
            let mut response = body("application/json", r#"{"hop":true}"#);
            let headers = response.headers_mut();
            for (name, value) in [
                ("connection", "keep-alive, X-Hop"),
                ("x-hop", "1"),
                ("keep-alive", "timeout=5"),
                ("proxy-authenticate", "Basic realm=\"playground\""),
                ("x-end", "1"),
            ] {
                headers.insert(name, HeaderValue::from_static(value));
            }
            response
        }
        "/echo" => echo(req).await,
        "/rpc" => {
            let fail = query.iter().any(|(key, _)| key == "fail");
            body(
                "application/json",
                benchkit::rpc_reply(&read_body(req).await, fail),
            )
        }
        "/ws.html" => body("text/html; charset=utf-8", WS_PAGE),
        "/ws" => websocket(
            req,
            WsOptions {
                ticks: number("ticks", 5).min(10_000),
                big: query
                    .iter()
                    .find(|(k, _)| k == "big")
                    .and_then(|(_, v)| v.parse().ok())
                    .map(|n: usize| n.min(64 << 20)),
                close: query
                    .iter()
                    .find(|(k, _)| k == "close")
                    .and_then(|(_, v)| v.parse().ok()),
                refuse: query
                    .iter()
                    .find(|(k, _)| k == "refuse")
                    .and_then(|(_, v)| v.parse().ok()),
            },
        ),
        "/shop.Orders/Get" => grpc_get(req).await,
        _ => match path.strip_prefix("/status/").map(str::parse::<u16>) {
            Some(Ok(code)) if (200..=599).contains(&code) => {
                let status = StatusCode::from_u16(code).expect("in range");
                with_status(
                    status,
                    body(
                        "application/json",
                        format!(
                            r#"{{"status":{code},"reason":"{}"}}"#,
                            status.canonical_reason().unwrap_or("")
                        ),
                    ),
                )
            }
            _ => with_status(
                StatusCode::NOT_FOUND,
                body(
                    "text/plain; charset=utf-8",
                    format!("No such path: {path}. / lists them all.\n"),
                ),
            ),
        },
    };
    Ok(response)
}

async fn read_body(req: Request<Incoming>) -> Bytes {
    req.into_body()
        .collect()
        .await
        .map(|body| body.to_bytes())
        .unwrap_or_default()
}

fn index() -> String {
    let mut rows = String::new();
    for (path, what) in ROUTES {
        let _ = writeln!(
            rows,
            r#"<tr><td><a href="{path}"><code>{path}</code></a></td><td>{}</td></tr>"#,
            escape_xml(what)
        );
    }
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>tracewing playground</title>
<style>body{{font:15px system-ui,sans-serif;margin:2rem;max-width:60rem}}td{{padding:.25rem .75rem}}code{{font-size:14px}}</style>
</head><body>
<h1>tracewing playground</h1>
<p>A server to try tracewing on. Each path answers with one kind of body.</p>
<table>
{rows}</table>
</body></html>
"#
    )
}

/// One made-up order: the same for the same `n`, every time.
struct Order {
    id: u64,
    customer: String,
    email: String,
    city: &'static str,
    total: u64,
    paid: bool,
    note: &'static str,
}

const FIRST: [&str; 8] = ["An", "Bình", "Chi", "Dũng", "Emma", "Liam", "Mai", "Noah"];
const LAST: [&str; 6] = ["Nguyễn", "Trần", "Lê", "Smith", "García", "Phạm"];
const CITIES: [&str; 6] = ["Hà Nội", "Đà Nẵng", "Huế", "Berlin", "Lyon", "Osaka"];
/// Notes with what each format has to escape: quotes, commas, `<` and `&`.
const NOTES: [&str; 6] = [
    "",
    "Giao trước 5pm",
    r#"Said "leave it at the door""#,
    "Red, blue, and green",
    "<b>fragile</b> & heavy",
    "Gói quà 🎁",
];

fn order(n: u64) -> Order {
    let mut rng = Rng(n.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let first = FIRST[rng.below(FIRST.len() as u64) as usize];
    let last = LAST[rng.below(LAST.len() as u64) as usize];
    Order {
        id: n,
        customer: format!("{first} {last}"),
        email: format!("{}.{n}@example.com", first.to_lowercase()),
        city: CITIES[rng.below(CITIES.len() as u64) as usize],
        total: 10_000 + rng.below(5_000_000),
        paid: rng.below(4) != 0,
        note: NOTES[rng.below(NOTES.len() as u64) as usize],
    }
}

/// Orders written by `write` until the body is `target` bytes, or five of
/// them when `target` is 0.
fn fill(target: usize, out: &mut String, mut write: impl FnMut(&mut String, &Order)) -> u64 {
    let mut n = 0;
    while (target == 0 && n < 5) || (target > 0 && out.len() < target) {
        n += 1;
        write(out, &order(n));
    }
    n
}

fn escape_json(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn orders_json(target: usize) -> String {
    let mut out = String::with_capacity(target + 1024);
    out.push_str(r#"{"orders":["#);
    let count = fill(target, &mut out, |out, o| {
        if o.id > 1 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"id":{},"customer":"{}","email":"{}","city":"{}","total":{},"paid":{},"note":{},"items":[{{"sku":"A-{}","qty":{}}}]}}"#,
            o.id,
            o.customer,
            o.email,
            o.city,
            o.total,
            o.paid,
            if o.note.is_empty() {
                "null".to_owned()
            } else {
                format!(r#""{}""#, escape_json(o.note))
            },
            o.id % 97,
            o.id % 5 + 1
        );
    });
    let _ = write!(out, r#"],"count":{count},"currency":"VND"}}"#);
    out
}

fn orders_xml(target: usize) -> String {
    let mut out = String::with_capacity(target + 1024);
    out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<orders currency=\"VND\">\n");
    fill(target, &mut out, |out, o| {
        let _ = writeln!(
            out,
            r#"  <order id="{}" paid="{}"><customer email="{}">{}</customer><city>{}</city><total>{}</total>{}</order>"#,
            o.id,
            o.paid,
            o.email,
            escape_xml(&o.customer),
            o.city,
            o.total,
            if o.note.is_empty() {
                String::new()
            } else {
                format!("<note>{}</note>", escape_xml(o.note))
            }
        );
    });
    out.push_str("</orders>\n");
    out
}

fn orders_html(target: usize) -> String {
    let mut out = String::with_capacity(target + 2048);
    out.push_str(
        r#"<!doctype html>
<html lang="vi">
<head>
  <meta charset="utf-8">
  <title>Orders</title>
  <link rel="stylesheet" href="/style.css">
  <script src="/app.js" defer></script>
</head>
<body>
  <h1>Orders</h1>
  <img src="/picture.png" alt="A picture" width="96" height="96">
  <table>
    <tr><th>Id</th><th>Customer</th><th>City</th><th>Total</th><th>Note</th></tr>
"#,
    );
    fill(target, &mut out, |out, o| {
        let _ = writeln!(
            out,
            "    <tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            o.id,
            escape_xml(&o.customer),
            o.city,
            o.total,
            escape_xml(o.note)
        );
    });
    out.push_str("  </table>\n</body>\n</html>\n");
    out
}

/// CSV with `,`, TSV with a tab.
fn orders_table(target: usize, separator: char) -> String {
    let cell = |text: &str| {
        if separator == ',' && (text.contains([',', '"', '\n'])) {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_owned()
        }
    };
    let mut out = String::with_capacity(target + 1024);
    let head = ["id", "customer", "email", "city", "total", "paid", "note"];
    out.push_str(&head.join(&separator.to_string()));
    out.push('\n');
    fill(target, &mut out, |out, o| {
        let row = [
            o.id.to_string(),
            cell(&o.customer),
            o.email.clone(),
            cell(o.city),
            o.total.to_string(),
            o.paid.to_string(),
            cell(o.note),
        ];
        out.push_str(&row.join(&separator.to_string()));
        out.push('\n');
    });
    out
}

fn log_text(target: usize) -> String {
    let mut out = String::with_capacity(target + 1024);
    fill(target, &mut out, |out, o| {
        let level = if o.paid { "INFO " } else { "WARN " };
        let _ = writeln!(
            out,
            "2026-11-12T{:02}:{:02}:{:02}Z {level} order {} for {} in {}: {} VND{}",
            o.id / 3600 % 24,
            o.id / 60 % 60,
            o.id % 60,
            o.id,
            o.customer,
            o.city,
            o.total,
            if o.paid { "" } else { ", not paid yet" }
        );
    });
    out
}

fn noise(size: usize) -> Vec<u8> {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    (0..size).map(|_| rng.next_u64() as u8).collect()
}

/// Server-sent events, or JSON lines, `gap` apart.
fn events(sse: bool, count: u64, gap: Duration) -> Response<OriginBody> {
    let count = count.min(100_000);
    let events = stream::unfold(0, move |i| async move {
        if i >= count {
            return None;
        }
        if i > 0 {
            tokio::time::sleep(gap).await;
        }
        let o = order(i + 1);
        let json = format!(
            r#"{{"n":{i},"order":{},"customer":"{}","total":{}}}"#,
            o.id, o.customer, o.total
        );
        let text = if sse {
            format!("event: order\nid: {i}\ndata: {json}\n\n")
        } else {
            format!("{json}\n")
        };
        Some((Ok::<_, Infallible>(Frame::data(Bytes::from(text))), i + 1))
    });
    Response::builder()
        .header(
            header::CONTENT_TYPE,
            if sse {
                "text/event-stream"
            } else {
                "application/x-ndjson"
            },
        )
        .header(header::CACHE_CONTROL, "no-cache")
        .body(BodyExt::boxed(StreamBody::new(events)))
        .expect("a valid answer")
}

fn cookies(req: &Request<Incoming>) -> Response<OriginBody> {
    let came: Vec<String> = req
        .headers()
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .map(|c| format!(r#""{}""#, escape_json(c.trim())))
        .collect();
    let mut response = body(
        "application/json",
        format!(r#"{{"cookies_that_came":[{}]}}"#, came.join(",")),
    );
    let headers = response.headers_mut();
    for cookie in [
        format!(
            "session={}; Path=/; HttpOnly; SameSite=Lax",
            benchkit::SESSION_JWT
        ),
        "theme=dark; Path=/; Max-Age=3600".to_owned(),
        "lang=vi; Path=/".to_owned(),
    ] {
        headers.append(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookie).expect("plain text"),
        );
    }
    response
}

fn auth(req: &Request<Incoming>) -> Response<OriginBody> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match token {
        Some(token) if !token.trim().is_empty() => body(
            "application/json",
            format!(r#"{{"ok":true,"token":"{}"}}"#, escape_json(token.trim())),
        ),
        _ => {
            let mut response = with_status(
                StatusCode::UNAUTHORIZED,
                body(
                    "application/json",
                    r#"{"error":"send a header Authorization: Bearer <token>"}"#,
                ),
            );
            response.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static(r#"Bearer realm="playground""#),
            );
            response
        }
    }
}

/// What came: the method, path, query, each header and the body.
async fn echo(req: Request<Incoming>) -> Response<OriginBody> {
    let mut out = format!(
        "{{\"method\":\"{}\",\"version\":\"{:?}\",\"path\":\"{}\",\"query\":\"{}\",\"headers\":{{",
        req.method(),
        req.version(),
        escape_json(req.uri().path()),
        escape_json(req.uri().query().unwrap_or(""))
    );
    let headers: Vec<String> = req
        .headers()
        .iter()
        .map(|(name, value)| {
            format!(
                r#""{name}":"{}""#,
                escape_json(&String::from_utf8_lossy(value.as_bytes()))
            )
        })
        .collect();
    out.push_str(&headers.join(","));
    let data = read_body(req).await;
    let text = String::from_utf8_lossy(&data);
    let _ = write!(
        out,
        r#"}},"body_bytes":{},"body":"{}"}}"#,
        data.len(),
        escape_json(&text)
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    );
    body("application/json", out)
}

/// What `/ws` is asked to do beside echoing.
struct WsOptions {
    /// Ticks, a second apart.
    ticks: u64,
    /// A binary message of this many bytes, after the hello.
    big: Option<usize>,
    /// Closing, with this code, after the hello.
    close: Option<u16>,
    /// No upgrade: this status instead.
    refuse: Option<u16>,
}

/// A WebSocket that says hello with what came (some headers, and the
/// subprotocol it took: the first offered), then does what `options` say,
/// and sends back whatever it gets: a close with its code and reason too.
fn websocket(mut req: Request<Incoming>, options: WsOptions) -> Response<OriginBody> {
    use hudsucker::tokio_tungstenite::tungstenite::protocol::{
        CloseFrame, frame::coding::CloseCode,
    };
    let upgrade = req
        .headers()
        .get(header::UPGRADE)
        .is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"websocket"));
    let key = req.headers().get(header::SEC_WEBSOCKET_KEY).cloned();
    let Some(key) = key.filter(|_| upgrade) else {
        return with_status(
            StatusCode::BAD_REQUEST,
            body(
                "text/plain; charset=utf-8",
                "A WebSocket goes here. /ws.html opens one.\n",
            ),
        );
    };
    if let Some(status) = options
        .refuse
        .and_then(|s| StatusCode::from_u16(s).ok())
        .filter(|s| s.as_u16() >= 200)
    {
        return with_status(
            status,
            body("text/plain; charset=utf-8", "No WebSocket for you.\n"),
        );
    }
    let header_of = |name: &str| {
        req.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let protocol = header_of("sec-websocket-protocol")
        .and_then(|offered| offered.split(',').next().map(|p| p.trim().to_owned()))
        .filter(|p| !p.is_empty());
    let hello = serde_json_like(&[
        ("hello", Some("tracewing playground".to_owned())),
        ("origin", header_of("origin")),
        ("cookie", header_of("cookie")),
        ("x_test", header_of("x-test")),
        ("protocol", protocol.clone()),
        ("extensions", header_of("sec-websocket-extensions")),
    ]);
    tokio::spawn(async move {
        let Ok(upgraded) = hyper::upgrade::on(&mut req).await else {
            return;
        };
        let ws = WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, None).await;
        let (mut sink, mut incoming) = ws.split();
        if sink.send(Message::text(hello)).await.is_err() {
            return;
        }
        if let Some(size) = options.big {
            let data: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
            if sink.send(Message::binary(data)).await.is_err() {
                return;
            }
        }
        if let Some(code) = options.close {
            let _ = sink
                .send(Message::Close(Some(CloseFrame {
                    code: CloseCode::from(code),
                    reason: "bye".into(),
                })))
                .await;
            // What the client says back, then the end.
            while let Some(Ok(message)) = incoming.next().await {
                if message.is_close() {
                    break;
                }
            }
            return;
        }
        let ticks = options.ticks;
        let mut clock = tokio::time::interval(Duration::from_secs(1));
        clock.tick().await;
        let mut ticked = 0;
        loop {
            tokio::select! {
                _ = clock.tick(), if ticked < ticks => {
                    ticked += 1;
                    let tick = format!(r#"{{"tick":{ticked},"of":{ticks}}}"#);
                    if sink.send(Message::text(tick)).await.is_err() {
                        break;
                    }
                }
                message = incoming.next() => match message {
                    Some(Ok(message)) if message.is_text() || message.is_binary() => {
                        if sink.send(message).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let _ = sink.send(Message::Close(frame)).await;
                        break;
                    }
                    Some(Ok(_)) => {}
                    _ => break,
                },
            }
        }
    });
    let mut response = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "Upgrade")
        .header(
            header::SEC_WEBSOCKET_ACCEPT,
            derive_accept_key(key.as_bytes()),
        );
    if let Some(protocol) = protocol {
        response = response.header("sec-websocket-protocol", protocol);
    }
    response
        .body(Full::new(Bytes::new()).boxed())
        .expect("a valid answer")
}

/// A flat JSON object of text values, those that are there.
fn serde_json_like(fields: &[(&str, Option<String>)]) -> String {
    let inner: Vec<String> = fields
        .iter()
        .filter_map(|(k, v)| v.as_ref().map(|v| format!(r#""{k}":"{}""#, escape_json(v))))
        .collect();
    format!("{{{}}}", inner.join(","))
}

/// `shop.Orders/Get`: the order with the id asked for, or for 42, the
/// status that it is not found.
async fn grpc_get(req: Request<Incoming>) -> Response<OriginBody> {
    let data = read_body(req).await;
    let id = data.get(5..).and_then(first_varint).unwrap_or(0);
    if id == 42 {
        return benchkit::grpc_not_found();
    }
    let order = benchkit::order_protobuf_for(id);
    let mut frame = vec![0u8];
    frame.extend_from_slice(&(order.len() as u32).to_be_bytes());
    frame.extend_from_slice(&order);
    let mut trailers = http::HeaderMap::new();
    trailers.insert("grpc-status", HeaderValue::from_static("0"));
    let frames = [Frame::data(Bytes::from(frame)), Frame::trailers(trailers)];
    Response::builder()
        .header(header::CONTENT_TYPE, "application/grpc")
        .body(BodyExt::boxed(StreamBody::new(stream::iter(
            frames.map(Ok::<_, Infallible>),
        ))))
        .expect("a valid answer")
}

/// Field 1 of a message, when it is a varint and comes first.
fn first_varint(data: &[u8]) -> Option<u64> {
    let (&tag, rest) = data.split_first()?;
    if tag != 1 << 3 {
        return None;
    }
    let mut value = 0u64;
    for (i, byte) in rest.iter().take(10).enumerate() {
        value |= u64::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120" viewBox="0 0 120 120">
  <rect width="120" height="120" rx="16" fill="#1e293b"/>
  <circle cx="60" cy="60" r="34" fill="none" stroke="#f59e0b" stroke-width="8"/>
  <path d="M42 60h36M60 42v36" stroke="#e2e8f0" stroke-width="6" stroke-linecap="round"/>
</svg>
"##;

const WS_PAGE: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>WebSocket</title>
<style>body{font:15px system-ui,sans-serif;margin:2rem;max-width:50rem}pre{background:#f1f5f9;padding:1rem;min-height:12rem;white-space:pre-wrap}</style>
</head><body>
<h1>WebSocket</h1>
<p><button id="open">Open</button> <button id="close">Close</button>
<input id="text" value="hello" size="30"> <button id="send">Send</button></p>
<pre id="log"></pre>
<script>
let ws;
const log = (line) => { document.getElementById("log").textContent += line + "\n"; };
document.getElementById("open").onclick = () => {
  ws = new WebSocket(`ws://${location.host}/ws?ticks=5`);
  ws.onopen = () => log("open");
  ws.onmessage = (e) => log("< " + e.data);
  ws.onclose = () => log("closed");
};
document.getElementById("close").onclick = () => ws && ws.close();
document.getElementById("send").onclick = () => {
  const text = document.getElementById("text").value;
  if (ws && ws.readyState === 1) { ws.send(text); log("> " + text); }
};
</script>
</body></html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    async fn start() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(serve(listener));
        format!("http://{address}")
    }

    #[tokio::test]
    async fn every_path_answers() {
        let base = start().await;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        for (path, _) in ROUTES {
            let response = client
                .get(format!("{base}{path}?mb=1&n=2&ms=1"))
                .send()
                .await
                .unwrap();
            let status = response.status().as_u16();
            let expected = match *path {
                "/status/404" => 404,
                "/redirect" => 302,
                "/auth" => 401,
                "/ws" => 400,
                _ => 200,
            };
            assert_eq!(status, expected, "{path}");
            let size = response.bytes().await.unwrap().len();
            if path.ends_with("_big") {
                assert!((1024 * 1024..1100 * 1024).contains(&size), "{path}: {size}");
            } else if *path != "/redirect" {
                assert!(size > 0, "{path}");
            }
        }
        let missing = client.get(format!("{base}/nope")).send().await.unwrap();
        assert_eq!(missing.status(), 404);
    }

    #[test]
    fn bodies_are_well_formed() {
        let json: serde_json::Value = serde_json::from_str(&orders_json(64 * 1024)).unwrap();
        let count = json["orders"].as_array().unwrap().len() as u64;
        assert_eq!(json["count"], count);
        assert!(count > 100);
        let small: serde_json::Value = serde_json::from_str(&orders_json(0)).unwrap();
        assert_eq!(small["count"], 5);

        // Every line has as many cells as the head, quotes and all.
        let csv = orders_table(32 * 1024, ',');
        for line in csv.lines() {
            let mut cells = 1;
            let mut quoted = false;
            for c in line.chars() {
                match c {
                    '"' => quoted = !quoted,
                    ',' if !quoted => cells += 1,
                    _ => {}
                }
            }
            assert_eq!(cells, 7, "{line}");
        }
        assert!(
            orders_table(0, '\t')
                .lines()
                .all(|l| l.split('\t').count() == 7)
        );
        let xml = orders_xml(16 * 1024);
        assert!(xml.contains("<note>&lt;b&gt;fragile&lt;/b&gt; &amp; heavy</note>"));
    }

    #[tokio::test]
    async fn the_websocket_says_hello_ticks_and_echoes() {
        let base = start().await;
        let url = format!("{}/ws?ticks=1", base.replace("http", "ws"));
        let (mut ws, _) = hudsucker::tokio_tungstenite::connect_async(url)
            .await
            .unwrap();
        let hello = ws.next().await.unwrap().unwrap();
        assert!(hello.to_text().unwrap().contains("hello"));
        ws.send(Message::text("ping me")).await.unwrap();
        let mut got = Vec::new();
        while got.len() < 2 {
            got.push(
                ws.next()
                    .await
                    .unwrap()
                    .unwrap()
                    .to_text()
                    .unwrap()
                    .to_owned(),
            );
        }
        assert!(got.contains(&"ping me".to_owned()), "{got:?}");
        assert!(got.iter().any(|m| m.contains(r#""tick":1"#)), "{got:?}");
    }

    #[tokio::test]
    async fn grpc_answers_the_order_asked_for() {
        let base = start().await;
        let client = reqwest::Client::builder()
            .no_proxy()
            .http2_prior_knowledge()
            .build()
            .unwrap();
        let call = |id: u8| {
            client
                .post(format!("{base}/shop.Orders/Get"))
                .header("content-type", "application/grpc")
                .header("te", "trailers")
                .body(vec![0, 0, 0, 0, 2, 8, id])
                .send()
        };
        let found = call(7).await.unwrap();
        assert_eq!(found.status(), 200);
        let data = found.bytes().await.unwrap();
        assert_eq!(&data[5..7], &[8, 7], "the order has the id asked for");

        let gone = call(42).await.unwrap();
        assert_eq!(gone.status(), 200);
        assert_eq!(first_varint(&[8, 0x96, 1]), Some(150));
    }
}
