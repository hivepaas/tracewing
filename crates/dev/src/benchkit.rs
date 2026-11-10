//! Benchmark helpers: an HTTPS origin with its own CA, and a load generator
//! that goes through the proxy and trusts only the proxy's CA, so every
//! successful request proves the proxy intercepted it.

use anyhow::{Context, Result};
use bytes::Bytes;
use http::{Request, Response, StatusCode, header};
use http_body_util::{BodyExt, Full, StreamBody, combinators::BoxBody};
use hudsucker::{
    futures::{SinkExt, StreamExt},
    rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose},
    rustls::{
        ServerConfig,
        pki_types::{
            CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, pem::PemObject,
        },
    },
};
use hyper::{body::Frame, body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use std::{
    collections::{BTreeMap, HashMap},
    convert::Infallible,
    io::{Read, Write},
    path::Path,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_rustls::TlsAcceptor;

type OriginBody = BoxBody<Bytes, Infallible>;

/// Serves the benchmark routes over TLS (HTTP/1.1 and HTTP/2) until the task
/// is dropped. Writes the origin's CA to `ca_out` so the proxy can trust it.
///
/// Routes:
/// - `/json?size=N`: a JSON body of N bytes;
/// - `/json?size=N&enc=gzip`: JSON of varied words, about N bytes, sent packed
///   (`gzip`, `br`, `zstd`, or `identity` for none). From a pool made beforehand,
///   so that what is timed is the proxy and not the origin compressing;
/// - `/stream?events=N&gap_ms=M`: N server-sent events, M ms apart;
/// - anything else: `ok`.
pub async fn serve_origin(listener: TcpListener, ca_out: &Path) -> Result<()> {
    let config = origin_tls(ca_out)?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    loop {
        let (tcp, _) = listener.accept().await?;
        tcp.set_nodelay(true)?;
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let Ok(tls) = acceptor.accept(tcp).await else {
                return;
            };
            let _ = Builder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(tls), service_fn(origin_route))
                .await;
        });
    }
}

/// What `/page.html`, `/app.js` and `/style.css` answer with.
pub const PAGE_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Fish &amp; chips</title>
  <link rel="stylesheet" href="/style.css">
  <!-- Loaded last, so the page shows first. -->
  <script src="/app.js" defer></script>
</head>
<body class="menu" data-theme='dark'>
  <h1>Today&#39;s menu</h1>
  <ul id="dishes"></ul>
  <img src=/picture.png alt="A plate" width=96>
</body>
</html>
"#;
pub const APP_JS: &str = r#"/**
 * Lists today's dishes.
 * @param {string} url where they are
 */
export async function listDishes(url = "/json?size=3") {
  const response = await fetch(url, { headers: { accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`fetch failed: ${response.status}`);
  }
  const dishes = await response.json(); // [{ name, price }]
  for (const dish of dishes ?? []) {
    console.log(dish.name, dish.price * 1.1, 0x1f, true, null);
  }
  return dishes.length;
}
"#;
pub const STYLE_CSS: &str = r#"@import url("fonts.css");
/* The menu, on every screen. */
body.menu, .menu > ul li:hover {
  font-family: "Inter", sans-serif;
  color: #1f2937;
  margin: 0 auto !important;
  max-width: 42rem;
}
@media (max-width: 600px) {
  h1 { font-size: 1.5em; padding: 4px 8px; }
}
"#;

/// What `/v1/messages` streams: an Anthropic answer that thinks, says
/// something, and calls a tool.
pub const ANTHROPIC_STREAM: &str = concat!(
    "event: message_start\n",
    "data: {\"type\":\"message_start\",\"message\":{\"model\":\"claude-sonnet-4-5\",\"usage\":{\"input_tokens\":412,\"output_tokens\":1}}}\n\n",
    "event: content_block_start\n",
    "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n",
    "event: content_block_delta\n",
    "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"The user wants a table for two tonight.\"}}\n\n",
    "event: content_block_start\n",
    "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
    "event: content_block_delta\n",
    "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"I will look for a table \"}}\n\n",
    "event: content_block_delta\n",
    "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"near the Old Quarter.\"}}\n\n",
    "event: content_block_start\n",
    "data: {\"type\":\"content_block_start\",\"index\":2,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_01\",\"name\":\"find_table\",\"input\":{}}}\n\n",
    "event: content_block_delta\n",
    "data: {\"type\":\"content_block_delta\",\"index\":2,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"area\\\":\\\"Hoan Kiem\\\",\"}}\n\n",
    "event: content_block_delta\n",
    "data: {\"type\":\"content_block_delta\",\"index\":2,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"people\\\":2}\"}}\n\n",
    "event: message_delta\n",
    "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":58}}\n\n",
    "event: message_stop\n",
    "data: {\"type\":\"message_stop\"}\n\n",
);

/// What `/picture.png` answers with: a 96 by 96 PNG.
pub const PICTURE: &[u8] = include_bytes!("picture.png");

/// What `/json?cookies=1` sets as its session: HS256, for user 42, expiring in 2100.
pub const SESSION_JWT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI0MiIsIm5hbWUiOiJBbmEiLCJpYXQiOjE3NjcyMjU2MDAsImV4cCI6NDEwMjQ0NDgwMH0.c2lnbmF0dXJl";

async fn origin_route(req: Request<Incoming>) -> Result<Response<OriginBody>, Infallible> {
    // Reads the whole body before it answers, as a server taking an upload does,
    // and says how much came.
    if req.uri().path() == "/upload" {
        let got = req
            .into_body()
            .collect()
            .await
            .map_or(0, |body| body.to_bytes().len());
        return Ok(Response::new(
            Full::new(Bytes::from(got.to_string())).boxed(),
        ));
    }
    // What reached the server, to check what was changed on the way: the query,
    // each header, and the body, a line each.
    if req.uri().path() == "/echo" {
        let mut text = format!("query={}\n", req.uri().query().unwrap_or(""));
        for (name, value) in req.headers() {
            text.push_str(&format!("{name}: {}\n", value.to_str().unwrap_or("?")));
        }
        let body = req
            .into_body()
            .collect()
            .await
            .map(|body| body.to_bytes())
            .unwrap_or_default();
        text.push_str(&format!("body={}", String::from_utf8_lossy(&body)));
        return Ok(Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Full::new(Bytes::from(text)).boxed())
            .expect("valid response"));
    }
    let query = parse_query(req.uri().query().unwrap_or(""));
    let number = |key: &str, default: u64| {
        query
            .get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };

    let response = match req.uri().path() {
        "/json" => {
            let size = number("size", 1024) as usize;
            let packed = query.get("enc").map(|enc| (*enc, packed_json(enc, size)));
            let body = match &packed {
                Some((_, body)) => body.clone(),
                None if query.contains_key("rich") => rich_json(),
                None => json_body(size),
            };
            let mut response = Response::builder().header(header::CONTENT_TYPE, "application/json");
            if query.contains_key("cookies") {
                response = response
                    .header(
                        header::SET_COOKIE,
                        format!("session={SESSION_JWT}; Path=/; HttpOnly; SameSite=Lax"),
                    )
                    .header(header::SET_COOKIE, "theme=dark; Max-Age=3600");
            }
            if let Some((enc, _)) = packed.filter(|(enc, _)| *enc != "identity") {
                response = response.header(header::CONTENT_ENCODING, enc);
            }
            response.body(Full::new(body).boxed())
        }
        // What a server that also speaks HTTP/3 says, or with `only=h3`, all it says.
        "/alt-svc" => Response::builder()
            .header(
                header::ALT_SVC,
                if query.get("only") == Some(&"h3") {
                    r#"h3=":443"; ma=86400, h3-29=":443""#
                } else {
                    r#"h3=":443"; ma=86400, h2="alt.example:443"; ma=60"#
                },
            )
            .body(Full::new(Bytes::from_static(b"ok")).boxed()),
        "/v1/messages" => Response::builder()
            .header(header::CONTENT_TYPE, "text/event-stream")
            .body(Full::new(Bytes::from_static(ANTHROPIC_STREAM.as_bytes())).boxed()),
        "/v1/chat/completions" => Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(
                Full::new(Bytes::from_static(
                    br#"{"model":"gpt-4.1-mini","choices":[{"message":{"role":"assistant","content":"Two tables are free at 7pm."},"finish_reason":"stop"}],"usage":{"prompt_tokens":96,"completion_tokens":9}}"#,
                ))
                .boxed(),
            ),
        "/page.html" => Response::builder()
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Full::new(Bytes::from_static(PAGE_HTML.as_bytes())).boxed()),
        "/app.js" => Response::builder()
            .header(header::CONTENT_TYPE, "text/javascript")
            .body(Full::new(Bytes::from_static(APP_JS.as_bytes())).boxed()),
        "/style.css" => Response::builder()
            .header(header::CONTENT_TYPE, "text/css")
            .body(Full::new(Bytes::from_static(STYLE_CSS.as_bytes())).boxed()),
        "/picture.png" => Response::builder()
            .header(header::CONTENT_TYPE, "image/png")
            .body(Full::new(Bytes::from_static(PICTURE)).boxed()),
        "/stream" => {
            let events = number("events", 5);
            let gap = Duration::from_millis(number("gap_ms", 200));
            // `format=ndjson`: a JSON object a line, each frame ending inside the
            // next line, as a server writing as it goes may cut them.
            let ndjson = query.get("format") == Some(&"ndjson");
            // `enc=gzip`, `br` or `zstd`: packed, and flushed after each event,
            // as a server that packs a stream must for it to come as it goes.
            let enc = query.get("enc").map(|enc| (*enc).to_owned());
            let packer = enc.as_deref().map(StreamPacker::new);
            let stream = hudsucker::futures::stream::unfold((0, packer), move |(i, mut packer)| async move {
                if i >= events {
                    return None;
                }
                if i > 0 {
                    tokio::time::sleep(gap).await;
                }
                let event = if !ndjson {
                    format!("data: event {i}\n\n")
                } else {
                    let head = if i == 0 { r#"{"n":"# } else { "" };
                    let tail = if i + 1 < events { r#"{"n":"# } else { "" };
                    format!("{head}{i}}}\n{tail}")
                };
                let bytes = match &mut packer {
                    Some(packer) => packer.pack(event.as_bytes(), i + 1 == events),
                    None => event.into_bytes(),
                };
                Some((Ok::<_, Infallible>(Frame::data(Bytes::from(bytes))), (i + 1, packer)))
            });
            let mut response = Response::builder();
            if let Some(enc) = &enc {
                response = response.header(header::CONTENT_ENCODING, enc.as_str());
            }
            response
                .header(
                    header::CONTENT_TYPE,
                    if ndjson {
                        "application/x-ndjson"
                    } else {
                        "text/event-stream"
                    },
                )
                .body(BodyExt::boxed(StreamBody::new(stream)))
        }
        _ => Response::builder().body(Full::new(Bytes::from_static(b"ok")).boxed()),
    };
    Ok(response.unwrap_or_else(|_| {
        let mut res = Response::new(Full::new(Bytes::new()).boxed());
        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        res
    }))
}

/// Packs a stream a piece at a time, each piece flushed so that it can be
/// unpacked before the next one comes.
enum StreamPacker {
    Gzip(flate2::write::GzEncoder<Vec<u8>>),
    Brotli(Box<brotli::CompressorWriter<Vec<u8>>>),
    Zstd(zstd::stream::write::Encoder<'static, Vec<u8>>),
}

impl StreamPacker {
    fn new(encoding: &str) -> Self {
        match encoding {
            "br" => Self::Brotli(Box::new(brotli::CompressorWriter::new(
                Vec::new(),
                4096,
                5,
                22,
            ))),
            "zstd" => Self::Zstd(zstd::stream::write::Encoder::new(Vec::new(), 3).unwrap()),
            _ => Self::Gzip(flate2::write::GzEncoder::new(
                Vec::new(),
                flate2::Compression::new(6),
            )),
        }
    }

    /// `data` packed and flushed; with `last`, the stream's end as well.
    fn pack(&mut self, data: &[u8], last: bool) -> Vec<u8> {
        use std::io::Write;
        let out = match self {
            Self::Gzip(p) => {
                p.write_all(data).unwrap();
                p.flush().unwrap();
                if last {
                    p.try_finish().unwrap();
                }
                p.get_mut()
            }
            Self::Brotli(p) => {
                p.write_all(data).unwrap();
                p.flush().unwrap();
                p.get_mut()
            }
            Self::Zstd(p) => {
                p.write_all(data).unwrap();
                p.flush().unwrap();
                if last {
                    p.do_finish().unwrap();
                }
                p.get_mut()
            }
        };
        std::mem::take(out)
    }
}

/// A different body of about 2 KB each time, the kind search has to work for.
fn rich_json() -> Bytes {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut rng = crate::seed::Rng(n.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    Bytes::from(crate::seed::rich_body(&mut rng, n, n))
}

const POOL: u64 = 4096;

/// Makes the bodies `/json?size=..&enc=..` serves, which the origin otherwise
/// does on the first request for them: for big bodies in a debug build, that
/// takes many seconds. Whatever times traffic should do this first.
pub fn prepare_bodies(encoding: &str, size: usize) {
    packed_json(encoding, size);
}

type Pools = Mutex<HashMap<(String, usize), Arc<Vec<Bytes>>>>;

/// One of `POOL` bodies, each different, in the encoding asked for.
fn packed_json(encoding: &str, size: usize) -> Bytes {
    static POOLS: OnceLock<Pools> = OnceLock::new();
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let pool = {
        let mut pools = POOLS
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        Arc::clone(
            pools
                .entry((encoding.to_owned(), size))
                .or_insert_with(|| Arc::new(make_pool(encoding, size))),
        )
    };
    pool[(NEXT.fetch_add(1, Ordering::Relaxed) % POOL) as usize].clone()
}

fn make_pool(encoding: &str, size: usize) -> Vec<Bytes> {
    (0..POOL)
        .map(|n| {
            let mut rng = crate::seed::Rng((n + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
            let text = crate::seed::rich_text(&mut rng, size).into_bytes();
            Bytes::from(pack(&text, encoding))
        })
        .collect()
}

fn pack(text: &[u8], encoding: &str) -> Vec<u8> {
    match encoding {
        "gzip" => {
            let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
            out.write_all(text).expect("a Vec takes any bytes");
            out.finish().expect("a Vec takes any bytes")
        }
        "br" => {
            let mut out = Vec::new();
            brotli::CompressorReader::new(text, 4096, 5, 22)
                .read_to_end(&mut out)
                .expect("a Vec takes any bytes");
            out
        }
        "zstd" => zstd::encode_all(text, 3).expect("a Vec takes any bytes"),
        _ => text.to_vec(),
    }
}

fn json_body(size: usize) -> Bytes {
    const HEAD: &[u8] = br#"{"data":""#;
    const TAIL: &[u8] = br#""}"#;
    let filler = size.saturating_sub(HEAD.len() + TAIL.len());
    let mut body = Vec::with_capacity(size.max(HEAD.len() + TAIL.len()));
    body.extend_from_slice(HEAD);
    body.extend(std::iter::repeat_n(b'x', filler));
    body.extend_from_slice(TAIL);
    Bytes::from(body)
}

fn parse_query(query: &str) -> BTreeMap<&str, &str> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .collect()
}

/// A WebSocket origin over TLS that sends back whatever it gets. It has a CA of its
/// own, written to `ca_out`.
pub async fn serve_ws_origin(listener: TcpListener, ca_out: &Path) -> Result<()> {
    let mut config = origin_tls(ca_out)?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(config));
    loop {
        let (tcp, _) = listener.accept().await?;
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let Ok(tls) = acceptor.accept(tcp).await else {
                return;
            };
            let Ok(mut ws) = hudsucker::tokio_tungstenite::accept_async(tls).await else {
                return;
            };
            while let Some(Ok(message)) = ws.next().await {
                if message.is_close() {
                    break;
                }
                if (message.is_text() || message.is_binary()) && ws.send(message).await.is_err() {
                    break;
                }
            }
        });
    }
}

pub type ClientWebSocket = hudsucker::tokio_tungstenite::WebSocketStream<
    tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
>;

/// Opens a WebSocket through the proxy, as a browser does: CONNECT, TLS with the
/// proxy's certificate, and the upgrade inside.
pub async fn websocket_through_proxy(
    proxy: std::net::SocketAddr,
    proxy_ca: &Path,
    host: &str,
    port: u16,
    path: &str,
) -> Result<ClientWebSocket> {
    let mut tcp = tokio::net::TcpStream::connect(proxy).await?;
    let connect = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n");
    tcp.write_all(connect.as_bytes()).await?;
    let mut reply = Vec::new();
    while !reply.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        tcp.read_exact(&mut byte).await?;
        reply.push(byte[0]);
    }
    anyhow::ensure!(reply.starts_with(b"HTTP/1.1 200"), "CONNECT refused");

    let mut roots = hudsucker::rustls::RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(proxy_ca)? {
        roots.add(cert?)?;
    }
    let config = hudsucker::rustls::ClientConfig::builder_with_provider(Arc::new(
        tracewing_proxy::crypto_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth();
    let tls = tokio_rustls::TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from(host.to_owned())?, tcp)
        .await?;
    let url = format!("wss://{host}:{port}{path}");
    let (ws, _) = hudsucker::tokio_tungstenite::client_async(url, tls).await?;
    Ok(ws)
}

/// A CA and a `localhost`/`127.0.0.1` leaf for the origin.
fn origin_tls(ca_out: &Path) -> Result<ServerConfig> {
    let ca_key = KeyPair::generate()?;
    let mut ca_params = CertificateParams::default();
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "tracewing bench origin CA");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca_cert = ca_params.self_signed(&ca_key)?;
    std::fs::write(ca_out, ca_cert.pem()).with_context(|| format!("write {}", ca_out.display()))?;
    let issuer = Issuer::new(ca_params, ca_key);

    let leaf_key = KeyPair::generate()?;
    let leaf_params = CertificateParams::new(vec!["localhost".to_owned(), "127.0.0.1".to_owned()])?;
    let leaf = leaf_params.signed_by(&leaf_key, &issuer)?;

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

pub struct LoadOptions {
    pub proxy: String,
    pub url: String,
    pub proxy_ca: std::path::PathBuf,
    pub requests: u64,
    pub concurrency: usize,
    pub http1_only: bool,
    /// Samples this process's resident memory while the load runs.
    pub watch_pid: Option<u32>,
}

#[derive(Debug)]
pub struct LoadReport {
    pub ok: u64,
    pub failed: u64,
    pub first_error: Option<String>,
    pub elapsed: Duration,
    pub latencies_us: Vec<u32>,
    pub versions: BTreeMap<String, u64>,
    pub peak_rss_mib: Option<f64>,
}

impl LoadReport {
    pub fn percentile_ms(&self, p: f64) -> f64 {
        if self.latencies_us.is_empty() {
            return 0.0;
        }
        let index = ((self.latencies_us.len() - 1) as f64 * p).round() as usize;
        self.latencies_us[index] as f64 / 1000.0
    }

    pub fn print(&self) {
        let total = self.ok + self.failed;
        println!(
            "{total} requests in {:.1}s: {:.0} req/s, {} ok, {} failed",
            self.elapsed.as_secs_f64(),
            total as f64 / self.elapsed.as_secs_f64(),
            self.ok,
            self.failed
        );
        println!(
            "latency ms: p50 {:.2}, p95 {:.2}, p99 {:.2}, max {:.2}",
            self.percentile_ms(0.50),
            self.percentile_ms(0.95),
            self.percentile_ms(0.99),
            self.percentile_ms(1.0)
        );
        println!("client versions: {:?}", self.versions);
        if let Some(peak) = self.peak_rss_mib {
            println!("proxy peak rss: {peak:.0} MiB");
        }
        if let Some(e) = &self.first_error {
            println!("first error: {e}");
        }
    }
}

pub fn client_through_proxy(
    proxy: &str,
    proxy_ca: &Path,
    http1_only: bool,
) -> Result<reqwest::Client> {
    let ca = std::fs::read(proxy_ca).with_context(|| format!("read {}", proxy_ca.display()))?;
    let mut builder = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(proxy)?)
        .tls_certs_only([reqwest::Certificate::from_pem(&ca)?]);
    if http1_only {
        builder = builder.http1_only();
    }
    Ok(builder.build()?)
}

pub async fn run_load(opts: LoadOptions) -> Result<LoadReport> {
    let client = client_through_proxy(&opts.proxy, &opts.proxy_ca, opts.http1_only)?;
    let next = Arc::new(AtomicU64::new(0));
    let running = Arc::new(AtomicBool::new(true));

    let watcher = opts.watch_pid.map(|pid| {
        let running = Arc::clone(&running);
        tokio::spawn(async move {
            let pid = pid.to_string();
            let mut peak: Option<f64> = None;
            while running.load(Ordering::Relaxed) {
                if let Some(rss) = tracewing_proxy::peak_rss_mib_of(&pid) {
                    peak = Some(peak.map_or(rss, |p| p.max(rss)));
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            peak
        })
    });

    let started = Instant::now();
    let mut workers = Vec::with_capacity(opts.concurrency);
    for _ in 0..opts.concurrency {
        let (client, next, url, total) = (
            client.clone(),
            Arc::clone(&next),
            opts.url.clone(),
            opts.requests,
        );
        workers.push(tokio::spawn(async move {
            let mut latencies = Vec::new();
            let mut versions = BTreeMap::<String, u64>::new();
            let (mut failed, mut first_error) = (0u64, None);
            while next.fetch_add(1, Ordering::Relaxed) < total {
                let sent = Instant::now();
                let result = async {
                    let res = client.get(&url).send().await?.error_for_status()?;
                    let version = format!("{:?}", res.version());
                    res.bytes().await?;
                    Ok::<_, reqwest::Error>(version)
                }
                .await;
                match result {
                    Ok(version) => {
                        latencies.push(sent.elapsed().as_micros().min(u32::MAX as u128) as u32);
                        *versions.entry(version).or_default() += 1;
                    }
                    Err(e) => {
                        failed += 1;
                        first_error
                            .get_or_insert_with(|| tracewing_proxy::capture::error_chain(&e));
                    }
                }
            }
            (latencies, versions, failed, first_error)
        }));
    }

    let mut report = LoadReport {
        ok: 0,
        failed: 0,
        first_error: None,
        elapsed: Duration::ZERO,
        latencies_us: Vec::new(),
        versions: BTreeMap::new(),
        peak_rss_mib: None,
    };
    for worker in workers {
        let (latencies, versions, failed, first_error) = worker.await?;
        report.ok += latencies.len() as u64;
        report.latencies_us.extend(latencies);
        for (version, count) in versions {
            *report.versions.entry(version).or_default() += count;
        }
        report.failed += failed;
        if report.first_error.is_none() {
            report.first_error = first_error;
        }
    }
    report.elapsed = started.elapsed();
    report.latencies_us.sort_unstable();

    running.store(false, Ordering::Relaxed);
    if let Some(watcher) = watcher {
        report.peak_rss_mib = watcher.await?;
    }
    Ok(report)
}
