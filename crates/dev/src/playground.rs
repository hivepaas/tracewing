//! A server to try tracewing on by hand: each kind of body at a path of its
//! own, big ones too, and streams, WebSocket and gRPC. Plain HTTP, so that
//! nothing has to trust it; HTTP/2 to a client that speaks it from the start,
//! as gRPC does.
//!
//! `/` lists every path, with links.

use crate::{
    benchkit::{self, OriginBody},
    chain, readings,
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
/// Each path the playground answers, what it sends, and the parameters worth
/// trying with it: one worded exactly as it goes on the end of the path
/// (`?mb=1`), or written whole where it is not a query (`/status/201`). The
/// listing offers them as links, since a parameter nobody can click is a
/// parameter everybody retypes.
pub const ROUTES: &[(&str, &str, &[&str])] = &[
    ("/json", "JSON: a few orders", &[]),
    (
        "/json_big",
        "JSON of 10 MB (?mb=N for another size)",
        &["?mb=1", "?mb=100"],
    ),
    (
        "/json_long",
        "JSON whose one value is too long for a line: base64 of more JSON",
        &[],
    ),
    ("/xml", "XML: the same orders", &[]),
    ("/xml_big", "XML of 10 MB (?mb=N)", &["?mb=1"]),
    ("/soap", "A SOAP answer, all on one line", &[]),
    (
        "/html",
        "An HTML page with a table, its CSS, JS and picture",
        &[],
    ),
    ("/html_big", "HTML of 10 MB (?mb=N)", &["?mb=1"]),
    ("/csv", "CSV, with quotes and commas inside cells", &[]),
    ("/csv_big", "CSV of 10 MB (?mb=N)", &["?mb=1"]),
    ("/tsv", "TSV", &[]),
    ("/tsv_big", "TSV of 10 MB (?mb=N)", &["?mb=1"]),
    ("/text", "Plain text: log lines", &[]),
    ("/text_big", "Plain text of 10 MB (?mb=N)", &["?mb=1"]),
    ("/js", "JavaScript", &[]),
    ("/css", "CSS", &[]),
    ("/svg", "An SVG picture", &[]),
    ("/png", "A PNG picture", &[]),
    ("/binary", "64 KB of bytes that are no format", &[]),
    ("/msgpack", "MessagePack", &[]),
    ("/cbor", "CBOR", &[]),
    ("/protobuf", "Protobuf, with no .proto", &[]),
    ("/grpc-web", "gRPC-Web: a message, then trailers", &[]),
    (
        "/shop.proto",
        "The .proto the Protobuf and gRPC answers are made by",
        &[],
    ),
    ("/gzip", "JSON packed with gzip", &[]),
    ("/br", "JSON packed with Brotli", &[]),
    ("/zstd", "JSON packed with zstd", &[]),
    (
        "/sse",
        "Server-sent events, one each 500 ms (?n=10&ms=500)",
        &["?n=3&ms=200", "?n=100&ms=20"],
    ),
    (
        "/ndjson",
        "A JSON line each 500 ms (?n=10&ms=500)",
        &["?n=3&ms=200", "?n=100&ms=20"],
    ),
    ("/slow", "JSON after 3 s (?ms=N)", &["?ms=500", "?ms=10000"]),
    (
        "/status/404",
        "Any status at all",
        &["/status/201", "/status/304", "/status/418", "/status/500"],
    ),
    (
        "/redirect",
        "Redirects 3 times, then JSON (?n=N)",
        &["?n=1", "?n=10"],
    ),
    ("/cookies", "Sets cookies, and says which came", &[]),
    (
        "/auth",
        "401 unless a header Authorization: Bearer … comes",
        &[],
    ),
    (
        "/hop",
        "Headers for the next hop alone, which a proxy must not pass on",
        &[],
    ),
    ("/echo", "Any method: says what came, body and all", &[]),
    ("/ws.html", "A page to talk to /ws", &[]),
    (
        "/ws",
        "WebSocket: says what came, echoes, ticks each second (?ticks=N), \
         sends N bytes (?big=N), closes with a code (?close=4000), refuses (?refuse=403)",
        &["?ticks=1", "?big=65536", "?close=4000", "?refuse=403"],
    ),
    (
        "/shop.Orders/Get",
        "gRPC (HTTP/2, POST): the order asked for; 42 is not found",
        &[],
    ),
    (
        "/rpc",
        "JSON-RPC (POST), as an Ethereum node answers; ?fail=1 reverts, in a 200",
        &["?fail=1"],
    ),
    (
        "/web3",
        "A page of web3 calls to send, one button each: calls, reverts, transactions, logs, receipts, batches",
        &[],
    ),
    (
        "/web3-ws",
        "JSON-RPC over a WebSocket, as a node that is subscribed to: /web3 opens one",
        &[],
    ),
    (
        "/readings",
        "One value of every kind the window can read, in headers and in the body: times, tokens, ids, encodings, packed bodies",
        &[],
    ),
];

/// Paths that want a body, and a body to send them. A link cannot ask for
/// these: a browser follows one with a GET and no body, which is how
/// `/rpc` came to answer "invalid request" to a click. The listing offers a
/// button instead, and sends this.
pub const POSTS: &[(&str, &str, &str)] = &[
    (
        "/rpc",
        "application/json",
        concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"eth_getBalance","#,
            r#""params":["0x5b24a0f0cbab79d9e7cf05a5cf5c28af0dc20ccc","latest"]}"#
        ),
    ),
    (
        "/echo",
        "text/plain; charset=utf-8",
        "Sent by the playground",
    ),
];

/// What the listing sends to `path`, if a link will not do.
pub fn post_body(path: &str) -> Option<(&'static str, &'static str)> {
    POSTS
        .iter()
        .find(|(at, _, _)| *at == path)
        .map(|(_, kind, body)| (*kind, *body))
}

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

/// Where it listens for clients with a certificate unless told otherwise.
pub const MTLS_PORT: u16 = 10444;

/// The same paths again, but only for a client that shows a certificate from
/// the playground's authority: see [`client_certificate`]. Each answer says
/// which certificate came, in `x-client-certificate`, by its size.
pub async fn serve_mtls(listener: TcpListener, dir: &Path) -> Result<()> {
    use hudsucker::rustls::pki_types::pem::PemObject as _;
    use hudsucker::rustls::{RootCertStore, server::WebPkiClientVerifier};
    let mut roots = RootCertStore::empty();
    roots.add(
        CertificateDer::from_pem_file(authority(dir)?)
            .context("read the playground's authority")?,
    )?;
    let provider = Arc::new(tracewing_proxy::crypto_provider());
    let verifier =
        WebPkiClientVerifier::builder_with_provider(Arc::new(roots), Arc::clone(&provider))
            .build()?;
    let mut config = tls_config_asking(dir, Some(verifier))?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    loop {
        let (tcp, _) = listener.accept().await?;
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let Ok(tls) = acceptor.accept(tcp).await else {
                return;
            };
            let shown = tls
                .get_ref()
                .1
                .peer_certificates()
                .and_then(|chain| chain.first())
                .map_or(0, |cert| cert.len());
            let _ = Builder::new(TokioExecutor::new())
                .serve_connection(
                    TokioIo::new(tls),
                    service_fn(move |req| async move {
                        let mut res = route(req).await?;
                        res.headers_mut()
                            .insert("x-client-certificate", HeaderValue::from(shown));
                        Ok::<_, Infallible>(res)
                    }),
                )
                .await;
        });
    }
}

/// A certificate for a client, from the playground's authority, and its key:
/// `client.pem` and `client-key.pem` in `dir`, made the first time.
pub fn client_certificate(dir: &Path) -> Result<(PathBuf, PathBuf)> {
    use hudsucker::rcgen::ExtendedKeyUsagePurpose;
    let (cert, key) = (dir.join("client.pem"), dir.join("client-key.pem"));
    if cert.exists() && key.exists() {
        return Ok((cert, key));
    }
    let ca = authority(dir)?;
    let ca_key = KeyPair::from_pem(&std::fs::read_to_string(dir.join("ca-key.pem"))?)?;
    let issuer = Issuer::from_ca_cert_pem(&std::fs::read_to_string(&ca)?, ca_key)?;
    let client_key = KeyPair::generate()?;
    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, "tracewing playground client");
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let client = params.signed_by(&client_key, &issuer)?;
    std::fs::write(&key, client_key.serialize_pem())?;
    std::fs::write(&cert, client.pem())?;
    Ok((cert, key))
}

fn tls_config(dir: &Path) -> Result<ServerConfig> {
    tls_config_asking(dir, None)
}

/// With `clients`, it asks each client for a certificate that it checks.
fn tls_config_asking(
    dir: &Path,
    clients: Option<Arc<dyn hudsucker::rustls::server::danger::ClientCertVerifier>>,
) -> Result<ServerConfig> {
    let cert = authority(dir)?;
    let ca_key = KeyPair::from_pem(&std::fs::read_to_string(dir.join("ca-key.pem"))?)?;
    let issuer = Issuer::from_ca_cert_pem(&std::fs::read_to_string(&cert)?, ca_key)?;
    let leaf_key = KeyPair::generate()?;
    let leaf = CertificateParams::new(vec!["localhost".to_owned(), "127.0.0.1".to_owned()])?
        .signed_by(&leaf_key, &issuer)?;
    let chain = vec![CertificateDer::from(leaf.der().to_vec())];
    let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));
    let builder = ServerConfig::builder_with_provider(Arc::new(tracewing_proxy::crypto_provider()))
        .with_safe_default_protocol_versions()?;
    let mut config = match clients {
        Some(verifier) => builder
            .with_client_cert_verifier(verifier)
            .with_single_cert(chain, key)?,
        None => builder.with_no_client_auth().with_single_cert(chain, key)?,
    };
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
        "/json_long" => body("application/json", long_value_json()),
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
            // A gateway that turns the request away, and a node that is slow
            // to answer: both are what a client meets, and neither is a
            // broken request.
            let limited = query.iter().any(|(key, _)| key == "limit");
            let wait = number("ms", 0).min(30_000);
            let asked = read_body(req).await;
            if wait > 0 {
                tokio::time::sleep(Duration::from_millis(wait)).await;
            }
            if limited {
                return Ok(with_status(
                    StatusCode::TOO_MANY_REQUESTS,
                    body("application/json", chain::too_many()),
                ));
            }
            body("application/json", chain::answer(&asked, fail))
        }
        "/web3" => body("text/html; charset=utf-8", web3_page()),
        "/web3-ws" => web3_socket(req),
        "/readings" => {
            let mut response = body("application/json; charset=utf-8", readings::body());
            // A route to look at over and over: the browser has to ask every
            // time, or the window has nothing to show the second time.
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            for (name, value) in readings::headers() {
                if let (Ok(name), Ok(value)) = (
                    http::HeaderName::from_bytes(name.as_bytes()),
                    HeaderValue::from_str(&value),
                ) {
                    response.headers_mut().append(name, value);
                }
            }
            response
        }
        // A value a JSON string cannot carry, served on its own.
        other if other.starts_with("/readings/") => {
            match readings::alone(other.trim_start_matches("/readings/")) {
                Some(sample) => body("text/plain; charset=utf-8", sample.value.clone()),
                None => with_status(
                    StatusCode::NOT_FOUND,
                    body("text/plain; charset=utf-8", "No such value here.\n"),
                ),
            }
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

/// Where one of a route's listed parameters leads: `?mb=1` goes on the end of
/// its path, `/status/201` is a path of its own.
pub fn variant_url(path: &str, variant: &str) -> String {
    if variant.starts_with('/') {
        variant.to_owned()
    } else {
        format!("{path}{variant}")
    }
}

fn index() -> String {
    let mut rows = String::new();
    for (path, what, variants) in ROUTES {
        // A path that wants a body is sent, not followed: a link would GET it
        // with nothing in it, which is not what the path is there to show.
        let sends = post_body(path).is_some();
        let one = |url: &str, text: &str| {
            if sends {
                format!(
                    r#" <button class="p" data-path="{}" data-url="{}"><code>{}</code></button>"#,
                    escape_xml(path),
                    escape_xml(url),
                    escape_xml(text)
                )
            } else {
                format!(
                    r#" <a class="p" href="{}"><code>{}</code></a>"#,
                    escape_xml(url),
                    escape_xml(text)
                )
            }
        };
        let mut links = String::new();
        for variant in *variants {
            links.push_str(&one(&variant_url(path, variant), variant));
        }
        let head = if sends {
            format!(
                r#"<code>{}</code>{}"#,
                escape_xml(path),
                one(path, "POST").trim_start()
            )
        } else {
            format!(r#"<a href="{path}"><code>{path}</code></a>"#)
        };
        let _ = writeln!(
            rows,
            r#"<tr><td>{head}{links}</td><td>{} <span class="said"></span></td></tr>"#,
            escape_xml(what)
        );
    }
    // What each button sends, by path.
    let mut sends = String::from("{");
    for (at, (path, kind, body)) in POSTS.iter().enumerate() {
        if at > 0 {
            sends.push(',');
        }
        let _ = write!(
            sends,
            r#""{}":["{}","{}"]"#,
            js_text(path),
            js_text(kind),
            js_text(body)
        );
    }
    sends.push('}');
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>tracewing playground</title>
<style>body{{font:15px system-ui,sans-serif;margin:2rem;max-width:60rem}}td{{padding:.25rem .75rem}}code{{font-size:14px}}a.p{{margin-left:.5rem;color:#666}}a.p code{{font-size:13px}}
button.p{{margin-left:.5rem;color:#666;background:#f2f2f2;border:1px solid #ddd;border-radius:.4rem;padding:.1rem .4rem;cursor:pointer}}
button.p:hover{{background:#e8e8e8}}.said{{margin-left:.5rem;color:#888;font:13px ui-monospace,monospace}}</style>
</head><body>
<h1>tracewing playground</h1>
<p>A server to try tracewing on. Each path answers with one kind of body.
A path that wants a body has a button instead of a link: a link would GET it
with nothing in it.</p>
<table>
{rows}</table>
<script>
const sends = {sends};
for (const button of document.querySelectorAll("button[data-url]")) {{
  button.addEventListener("click", async () => {{
    const [kind, body] = sends[button.dataset.path];
    const said = button.closest("tr").querySelector(".said");
    said.textContent = "sending…";
    try {{
      const answer = await fetch(button.dataset.url, {{
        method: "POST",
        headers: {{ "content-type": kind }},
        body,
      }});
      const text = await answer.text();
      said.textContent = answer.status + " " + text.slice(0, 120);
    }} catch (trouble) {{
      said.textContent = String(trouble);
    }}
  }});
}}
</script>
</body></html>
"#
    )
}

/// The web3 page: every case of `chain::cases()` as a button that sends it,
/// so that a window can be filled with one kind of call after another
/// without a wallet or a chain anywhere.
fn web3_page() -> String {
    let mut rows = String::new();
    let mut group = "";
    for case in chain::cases() {
        if case.group != group {
            group = case.group;
            let _ = writeln!(
                rows,
                r#"<tr><th colspan="2">{}</th></tr>"#,
                escape_xml(group)
            );
        }
        let _ = writeln!(
            rows,
            concat!(
                r#"<tr><td><button class="p" data-case="{}">{}</button></td>"#,
                r#"<td>{} <span class="said"></span></td></tr>"#
            ),
            escape_xml(case.id),
            escape_xml(case.name),
            escape_xml(case.what)
        );
    }
    let mut sends = String::from("{");
    for (at, case) in chain::cases().iter().enumerate() {
        if at > 0 {
            sends.push(',');
        }
        let _ = write!(
            sends,
            r#""{}":["{}","{}"]"#,
            js_text(case.id),
            js_text(&case.url),
            js_text(&case.body)
        );
    }
    sends.push('}');
    format!(
        r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>web3</title>
<style>body{{font:15px system-ui,sans-serif;margin:2rem;max-width:64rem}}
td,th{{padding:.25rem .75rem;text-align:left;vertical-align:top}}
th{{padding-top:1.2rem;color:#555;border-bottom:1px solid #ddd}}
button.p{{background:#f2f2f2;border:1px solid #ddd;border-radius:.4rem;padding:.15rem .5rem;cursor:pointer;font:13px ui-monospace,monospace;white-space:nowrap}}
button.p:hover{{background:#e8e8e8}}
.said{{margin-left:.5rem;color:#888;font:12px ui-monospace,monospace;word-break:break-all}}
h2{{margin-top:2rem}}
pre{{background:#f6f7f9;padding:.75rem;height:11rem;overflow:auto;font:12px ui-monospace,monospace;white-space:pre-wrap;word-break:break-all}}</style>
</head><body>
<h1>web3</h1>
<p>Each button sends one JSON-RPC request to <code>/rpc</code>, as a wallet or
a library would. What comes back is what a node said: the calls and answers
in <code>crates/store/testdata/jsonrpc</code> were captured from Ethereum
mainnet, and the rest is written in the same shape.</p>
<p><button id="all" class="p">Send every one, in turn</button>
<span id="toll" class="said"></span></p>
<h2>Live</h2>
<p>A node at the end of a WebSocket: calls go down it and are answered on it,
and what it is subscribed to arrives on its own, as notifications with no id.</p>
<p><button id="open" class="p">Open</button>
<button id="heads" class="p">subscribe newHeads</button>
<button id="logs" class="p">subscribe logs</button>
<button id="pending" class="p">subscribe pending</button>
<button id="ask" class="p">eth_blockNumber</button>
<button id="drop" class="p">unsubscribe heads</button>
<button id="shut" class="p">Close</button>
<span id="live" class="said">not open</span></p>
<pre id="feed"></pre>
<table>
{rows}</table>
<script>
const sends = {sends};
async function send(button) {{
  const [url, body] = sends[button.dataset.case];
  const said = button.closest("tr").querySelector(".said");
  said.textContent = "sending…";
  try {{
    const answer = await fetch(url, {{
      method: "POST",
      headers: {{ "content-type": "application/json" }},
      body,
    }});
    const text = await answer.text();
    said.textContent = answer.status + " " + text.slice(0, 160);
  }} catch (trouble) {{
    said.textContent = String(trouble);
  }}
}}
for (const button of document.querySelectorAll("button[data-case]")) {{
  button.addEventListener("click", () => send(button));
}}
let socket = null;
let asked = 0;
const feed = document.getElementById("feed");
const live = document.getElementById("live");
function note(way, text) {{
  feed.textContent += way + " " + text + "\n";
  feed.scrollTop = feed.scrollHeight;
}}
function down(call) {{
  if (!socket || socket.readyState !== WebSocket.OPEN) {{
    live.textContent = "open it first";
    return;
  }}
  asked += 1;
  const text = JSON.stringify({{ jsonrpc: "2.0", id: asked, ...call }});
  socket.send(text);
  note("→", text);
}}
document.getElementById("open").addEventListener("click", () => {{
  if (socket && socket.readyState <= WebSocket.OPEN) {{
    return;
  }}
  const where = (location.protocol === "https:" ? "wss://" : "ws://") + location.host + "/web3-ws";
  socket = new WebSocket(where);
  live.textContent = "opening " + where;
  socket.addEventListener("open", () => {{ live.textContent = "open"; }});
  socket.addEventListener("message", (event) => note("←", event.data));
  socket.addEventListener("close", (event) => {{ live.textContent = "closed " + event.code; }});
  socket.addEventListener("error", () => {{ live.textContent = "could not open"; }});
}});
document.getElementById("heads").addEventListener("click", () =>
  down({{ method: "eth_subscribe", params: ["newHeads"] }}));
document.getElementById("logs").addEventListener("click", () =>
  down({{ method: "eth_subscribe", params: ["logs", {{ address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48" }}] }}));
document.getElementById("pending").addEventListener("click", () =>
  down({{ method: "eth_subscribe", params: ["newPendingTransactions"] }}));
document.getElementById("ask").addEventListener("click", () =>
  down({{ method: "eth_blockNumber", params: [] }}));
document.getElementById("drop").addEventListener("click", () =>
  down({{ method: "eth_unsubscribe", params: ["0xcd0c3e8af590364c09d0fa6a1210faf5"] }}));
document.getElementById("shut").addEventListener("click", () => {{
  if (socket) {{
    socket.close(1000, "done");
  }}
}});
document.getElementById("all").addEventListener("click", async () => {{
  const buttons = [...document.querySelectorAll("button[data-case]")];
  const toll = document.getElementById("toll");
  for (const [at, button] of buttons.entries()) {{
    toll.textContent = `${{at + 1}} of ${{buttons.length}}`;
    await send(button);
  }}
  toll.textContent = `${{buttons.length}} sent`;
}});
</script>
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

/// Text to go inside a JavaScript string: its quotes and backslashes kept,
/// and no `</` to end the script before its time.
fn js_text(text: &str) -> String {
    text.replace('\\', r"\\")
        .replace('"', "\\\"")
        .replace('\n', r"\n")
        .replace("</", r"<\/")
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// One value far past what a line of the window shows, so that what is cut
/// off, and the way to the rest of it, can be tried by hand. It is base64 of
/// JSON, so the value reads as something once it is opened.
fn long_value_json() -> String {
    use base64::Engine as _;
    let inside = orders_json(3 * 1024);
    let blob = base64::engine::general_purpose::STANDARD.encode(inside.as_bytes());
    format!(r#"{{"id":1,"blob":"{blob}","note":"the blob is base64 of the orders"}}"#)
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
/// A node at the end of a WebSocket: it answers calls as `/rpc` does, and
/// what it is subscribed to comes down on its own, as notifications with no
/// id. This is how a wallet or an indexer watches a chain.
fn web3_socket(mut req: Request<Incoming>) -> Response<OriginBody> {
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
                "A node's WebSocket goes here. /web3 opens one.\n",
            ),
        );
    };
    tokio::spawn(async move {
        let Ok(upgraded) = hyper::upgrade::on(&mut req).await else {
            return;
        };
        let ws = WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, None).await;
        let (mut sink, mut incoming) = ws.split();
        // What this socket is subscribed to, and how much of each feed has
        // come down it.
        let mut feeds: Vec<(chain::Feed, u64)> = Vec::new();
        let mut clock = tokio::time::interval(Duration::from_millis(1200));
        clock.tick().await;
        loop {
            tokio::select! {
                _ = clock.tick(), if !feeds.is_empty() => {
                    let mut gone = false;
                    for (feed, sent) in &mut feeds {
                        if sink.send(Message::text(feed.notice(*sent))).await.is_err() {
                            gone = true;
                            break;
                        }
                        *sent += 1;
                    }
                    if gone {
                        break;
                    }
                }
                message = incoming.next() => match message {
                    Some(Ok(Message::Text(asked))) => {
                        let mut replies = Vec::new();
                        for call in tracewing_store::jsonrpc::calls_in(asked.as_bytes()) {
                            replies.push(match call.method.as_str() {
                                "eth_subscribe" => match chain::Feed::asked(&call) {
                                    Some(feed) => {
                                        if !feeds.iter().any(|(had, _)| *had == feed) {
                                            feeds.push((feed, 0));
                                        }
                                        chain::answered(&call, &format!(r#""{}""#, feed.handle()))
                                    }
                                    // A feed this node does not keep.
                                    None => chain::not_known(&call),
                                },
                                "eth_unsubscribe" => {
                                    let handle = call
                                        .params
                                        .as_ref()
                                        .and_then(|p| p.get(0))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or_default();
                                    let had = chain::Feed::of_handle(handle)
                                        .is_some_and(|feed| {
                                            let before = feeds.len();
                                            feeds.retain(|(kept, _)| *kept != feed);
                                            feeds.len() < before
                                        });
                                    chain::answered(&call, if had { "true" } else { "false" })
                                }
                                _ => chain::answer(asked.as_bytes(), false),
                            });
                        }
                        // A message that is not JSON-RPC at all is echoed, so
                        // that a socket is still a socket to try.
                        if replies.is_empty() {
                            replies.push(asked.to_string());
                        }
                        for reply in replies {
                            if sink.send(Message::text(reply)).await.is_err() {
                                return;
                            }
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
    Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "Upgrade")
        .header(
            header::SEC_WEBSOCKET_ACCEPT,
            derive_accept_key(key.as_bytes()),
        )
        .body(Full::new(Bytes::new()).boxed())
        .expect("a valid answer")
}

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
    // As UTF-8, to show a value that is not ASCII as it was sent.
    let header_of = |name: &str| {
        req.headers()
            .get(name)
            .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned())
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
  // Over TLS the socket must be `wss`, or it meets the TLS port speaking
  // plain HTTP and the handshake fails with "invalid HTTP version".
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  ws = new WebSocket(`${scheme}://${location.host}/ws?ticks=5`);
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
        for (path, _, variants) in ROUTES {
            let url = format!("{base}{path}?mb=1&n=2&ms=1");
            let response = match post_body(path) {
                Some((kind, body)) => client
                    .post(&url)
                    .header("content-type", kind)
                    .body(body)
                    .send()
                    .await
                    .unwrap(),
                None => client.get(&url).send().await.unwrap(),
            };
            let status = response.status().as_u16();
            let expected = match *path {
                "/status/404" => 404,
                "/redirect" => 302,
                "/auth" => 401,
                "/ws" | "/web3-ws" => 400,
                _ => 200,
            };
            assert_eq!(status, expected, "{path}");
            let size = response.bytes().await.unwrap().len();
            if path.ends_with("_big") {
                assert!((1024 * 1024..1100 * 1024).contains(&size), "{path}: {size}");
            } else if *path != "/redirect" {
                assert!(size > 0, "{path}");
            }
            // What the listing offers as a link had better answer. The large
            // and slow ones are left out: being large and slow is what those
            // links are for, and the suite should not wait for them.
            for variant in *variants {
                if is_slow(variant) {
                    continue;
                }
                let url = format!("{base}{}", variant_url(path, variant));
                let answer = match post_body(path) {
                    Some((kind, body)) => client
                        .post(&url)
                        .header("content-type", kind)
                        .body(body)
                        .send()
                        .await
                        .unwrap(),
                    None => client.get(&url).send().await.unwrap(),
                };
                let status = answer.status().as_u16();
                let wanted = variant
                    .strip_prefix("/status/")
                    .and_then(|code| code.parse().ok())
                    .unwrap_or(expected);
                assert_eq!(status, wanted, "{url}");
            }
        }
        let missing = client.get(format!("{base}/nope")).send().await.unwrap();
        assert_eq!(missing.status(), 404);
    }

    /// What the listing's buttons send has to be answered for real. A link
    /// would GET `/rpc` with no body, which it answers "invalid request" --
    /// a 200 that reads as the path being broken.
    #[tokio::test]
    async fn what_the_listing_sends_is_answered() {
        let base = start().await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let said = |url: String, kind: &'static str, body: &'static str| {
            let client = client.clone();
            async move {
                client
                    .post(url)
                    .header("content-type", kind)
                    .body(body)
                    .send()
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap()
            }
        };
        let (kind, body) = post_body("/rpc").unwrap();
        let answered = said(format!("{base}/rpc"), kind, body).await;
        assert!(
            answered.contains(r#""result""#) && !answered.contains("invalid request"),
            "{answered}"
        );
        let reverted = said(format!("{base}/rpc?fail=1"), kind, body).await;
        assert!(reverted.contains("execution reverted"), "{reverted}");
        let (kind, body) = post_body("/echo").unwrap();
        let echoed = said(format!("{base}/echo"), kind, body).await;
        assert!(echoed.contains(body), "{echoed}");

        // And the page offers them that way: a button, not a link.
        let page = client
            .get(base.clone())
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(
            page.contains(r#"<button class="p" data-path="/rpc""#),
            "{page}"
        );
        assert!(!page.contains(r#"<a href="/rpc">"#), "{page}");
    }

    /// The node's socket answers what is asked on it, and what it is
    /// subscribed to comes down on its own until it is unsubscribed.
    #[tokio::test]
    async fn the_node_s_socket_answers_and_then_keeps_sending() {
        let base = start().await;
        let url = format!("{}/web3-ws", base.replace("http", "ws"));
        let (mut ws, _) = hudsucker::tokio_tungstenite::connect_async(url)
            .await
            .unwrap();
        // A closure cannot hand back a future that borrows the socket, so
        // the next message is taken where it is wanted.
        macro_rules! said {
            () => {
                ws.next()
                    .await
                    .unwrap()
                    .unwrap()
                    .to_text()
                    .unwrap()
                    .to_owned()
            };
        }

        // A call on the socket is answered on the socket, by its id.
        ws.send(Message::text(
            r#"{"jsonrpc":"2.0","id":3,"method":"eth_blockNumber","params":[]}"#,
        ))
        .await
        .unwrap();
        let reply = said!();
        assert!(
            reply.contains(r#""id":3"#) && reply.contains("0x192bbde"),
            "{reply}"
        );

        // Subscribing is answered with a handle, and then the heads come.
        ws.send(Message::text(
            r#"{"jsonrpc":"2.0","id":4,"method":"eth_subscribe","params":["newHeads"]}"#,
        ))
        .await
        .unwrap();
        let handle = said!();
        assert!(handle.contains(r#""id":4"#), "{handle}");
        let handle = handle
            .split(r#""result":""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap()
            .to_owned();

        let first = said!();
        assert!(first.contains(r#""method":"eth_subscription""#), "{first}");
        assert!(first.contains(&handle), "{first}");
        assert!(
            !first.contains(r#""id""#),
            "a notification wants no reply: {first}"
        );
        let second = said!();
        assert_ne!(
            block_of(&first),
            block_of(&second),
            "each head is a block further on"
        );

        // And unsubscribing stops it: what comes next is the answer to the
        // next call, not another head.
        ws.send(Message::text(format!(
            r#"{{"jsonrpc":"2.0","id":5,"method":"eth_unsubscribe","params":["{handle}"]}}"#
        )))
        .await
        .unwrap();
        let mut stopped = said!();
        // A head already on its way is allowed before the answer.
        while stopped.contains("eth_subscription") {
            stopped = said!();
        }
        assert!(stopped.contains(r#""result":true"#), "{stopped}");
        ws.send(Message::text(
            r#"{"jsonrpc":"2.0","id":6,"method":"eth_chainId","params":[]}"#,
        ))
        .await
        .unwrap();
        let after = said!();
        assert!(
            after.contains(r#""id":6"#),
            "nothing is still coming: {after}"
        );
    }

    /// The block a head notice is of.
    fn block_of(notice: &str) -> String {
        notice
            .split(r#""number":""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_default()
            .to_owned()
    }

    /// A feed this node does not keep is turned away, not quietly dropped.
    #[tokio::test]
    async fn a_feed_that_is_not_kept_is_said_so() {
        let base = start().await;
        let url = format!("{}/web3-ws", base.replace("http", "ws"));
        let (mut ws, _) = hudsucker::tokio_tungstenite::connect_async(url)
            .await
            .unwrap();
        ws.send(Message::text(
            r#"{"jsonrpc":"2.0","id":1,"method":"eth_subscribe","params":["syncing"]}"#,
        ))
        .await
        .unwrap();
        let reply = ws
            .next()
            .await
            .unwrap()
            .unwrap()
            .to_text()
            .unwrap()
            .to_owned();
        assert!(reply.contains("-32601"), "{reply}");
    }

    /// Every button on the web3 page has to send something the node answers,
    /// and the page has to offer every case there is.
    #[tokio::test]
    async fn the_web3_page_sends_what_the_node_answers() {
        let base = start().await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let page = client
            .get(format!("{base}/web3"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        for case in chain::cases() {
            assert!(
                page.contains(&format!(r#"data-case="{}""#, case.id)),
                "{} is not on the page",
                case.id
            );
            let answer = client
                .post(format!("{base}{}", case.url))
                .header("content-type", "application/json")
                .body(case.body.clone())
                .send()
                .await
                .unwrap();
            let status = answer.status().as_u16();
            let wanted = if case.id == "rate-limited" { 429 } else { 200 };
            assert_eq!(status, wanted, "{}", case.id);
            let said = answer.text().await.unwrap();
            assert!(said.contains("jsonrpc"), "{}: {said}", case.id);
            if case.group != "Batches and trouble" {
                assert!(!said.contains("-32600"), "{}: {said}", case.id);
            }
        }
    }

    /// Everything the readings route carries has to arrive: the headers as
    /// headers, the body as JSON, and what a JSON string cannot hold on its
    /// own path.
    #[tokio::test]
    async fn the_readings_route_carries_every_kind() {
        let base = start().await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let answer = client.get(format!("{base}/readings")).send().await.unwrap();
        assert_eq!(answer.status(), 200);
        let headers = answer.headers().clone();
        let said = answer.text().await.unwrap();
        let body: serde_json::Value = serde_json::from_str(&said).unwrap();
        for sample in readings::samples() {
            match sample.at {
                readings::At::Header => assert_eq!(
                    headers.get(sample.name).map(|v| v.to_str().unwrap()),
                    Some(sample.value.as_str()),
                    "{}",
                    sample.name
                ),
                readings::At::Body => assert_eq!(
                    body.get(sample.name).and_then(|v| v.as_str()),
                    Some(sample.value.as_str()),
                    "{}",
                    sample.name
                ),
                readings::At::Alone => {
                    let alone = client
                        .get(format!("{base}/readings/{}", sample.name))
                        .send()
                        .await
                        .unwrap();
                    assert_eq!(alone.status(), 200, "{}", sample.name);
                    assert_eq!(alone.text().await.unwrap(), sample.value, "{}", sample.name);
                }
            }
        }
        let missing = client
            .get(format!("{base}/readings/nothing.pem"))
            .send()
            .await
            .unwrap();
        assert_eq!(missing.status(), 404);
    }

    /// A route for looking at again and again must not tell the browser to
    /// keep it: a cached answer never leaves the browser, so the window has
    /// nothing to catch the second time. This is what happened.
    #[tokio::test]
    async fn the_readings_route_is_never_kept_by_a_browser() {
        let base = start().await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let answer = client.get(format!("{base}/readings")).send().await.unwrap();
        assert_eq!(
            answer
                .headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
        // And nothing else in it tells a browser to keep it either: the
        // sample that says an hour is carried under a name nobody obeys.
        for (name, value) in answer.headers() {
            let said = value.to_str().unwrap_or_default();
            if said.contains("max-age") {
                assert_eq!(
                    name.as_str(),
                    "x-cache-control",
                    "{name} would be obeyed: {said}"
                );
            }
        }
    }

    /// The slow case waits before it answers, which is the point of it.
    #[tokio::test]
    async fn a_slow_call_takes_the_time_it_was_asked_for() {
        let base = start().await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let began = std::time::Instant::now();
        let said = client
            .post(format!("{base}/rpc?ms=300"))
            .header("content-type", "application/json")
            .body(r#"{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber"}"#)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(began.elapsed() >= Duration::from_millis(300));
        assert!(said.contains("0x192bbde"), "{said}");
    }

    /// Whether a listed parameter asks for something big or slow on purpose.
    fn is_slow(variant: &str) -> bool {
        benchkit::parse_query(variant.trim_start_matches('?'))
            .into_iter()
            .any(|(key, value)| {
                let n: u64 = value.parse().unwrap_or(0);
                matches!(key, "mb" if n > 10)
                    || matches!(key, "ms" if n > 1000)
                    || matches!(key, "n" if n > 10)
                    || matches!(key, "big" if n > 1024)
            })
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
