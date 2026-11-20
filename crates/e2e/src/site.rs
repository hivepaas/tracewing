//! What S5 opens in a browser: a page served over plain HTTP, so HTTP/1.1,
//! whose script then asks an HTTPS server for JSON (HTTP/2, the browser
//! speaking it to the proxy), opens a WebSocket to it and reads its
//! server-sent events, reaches the Set up test page, and says how each went
//! by asking for `/done?…` on the page's own server.
//!
//! Both are on this computer's own address on its network, not on loopback:
//! a browser sends a loopback address past any proxy the system names. The
//! HTTPS server's answers offer HTTP/3 in `Alt-Svc`, and the server listens
//! for UDP on its port, counting what arrives: a browser that tried HTTP/3
//! would be heard.

use bytes::Bytes;
use http::{Request, Response, StatusCode, header};
use http_body_util::{BodyExt as _, Full, combinators::BoxBody};
use hudsucker::{
    futures::{SinkExt, StreamExt},
    rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose},
    rustls::{
        ServerConfig,
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    },
    tokio_tungstenite::{
        WebSocketStream,
        tungstenite::{handshake::derive_accept_key, protocol::Role},
    },
};
use hyper::{body::Incoming, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use std::{
    collections::BTreeMap,
    convert::Infallible,
    net::{IpAddr, UdpSocket},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

type Body = BoxBody<Bytes, Infallible>;

/// What the WebSocket is sent, and sends back.
pub const PING: &str = "ping-from-the-browser";
/// The events `/sse` sends, in order.
pub const EVENTS: [&str; 3] = ["first", "second", "third"];

/// This computer's address on its network: the one it would reach the rest
/// of the network from. Nothing is sent to find it.
pub fn this_computers_address() -> IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("192.0.2.1:9").unwrap();
    let ip = socket.local_addr().unwrap().ip();
    assert!(!ip.is_loopback(), "no address but loopback");
    ip
}

#[derive(Default)]
struct Seen {
    /// The last `/done`'s query, by name.
    done: Mutex<Option<BTreeMap<String, String>>>,
    datagrams: AtomicU64,
}

/// The page's server and the HTTPS one, until dropped.
pub struct Site {
    pub ip: IpAddr,
    /// The page's, plain HTTP.
    pub plain: u16,
    /// The HTTPS server's, TCP and UDP.
    pub tls: u16,
    /// The HTTPS server's authority, which only tracewing is told to trust.
    pub ca: PathBuf,
    seen: Arc<Seen>,
    _runtime: tokio::runtime::Runtime,
}

impl Site {
    pub fn start(dir: &Path) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let ip = this_computers_address();
        let ca = dir.join("site-ca.pem");
        let config = Arc::new(tls_config(ip, &ca));
        let seen = Arc::new(Seen::default());
        let (plain_listener, tls_listener) = runtime.block_on(async {
            let plain = TcpListener::bind((ip, 0)).await.unwrap();
            let tls = TcpListener::bind((ip, 0)).await.unwrap();
            (plain, tls)
        });
        let plain = plain_listener.local_addr().unwrap().port();
        let tls = tls_listener.local_addr().unwrap().port();
        let udp = runtime
            .block_on(tokio::net::UdpSocket::bind((ip, tls)))
            .unwrap();
        let routes = Arc::new(Routes {
            seen: seen.clone(),
            tls_origin: format!("https://{}", host_port(ip, tls)),
            tls_port: tls,
        });

        let r = routes.clone();
        runtime.spawn(async move {
            loop {
                let Ok((tcp, _)) = plain_listener.accept().await else {
                    return;
                };
                let r = r.clone();
                tokio::spawn(async move {
                    let _ = Builder::new(TokioExecutor::new())
                        .serve_connection_with_upgrades(
                            TokioIo::new(tcp),
                            service_fn(move |req| {
                                let r = r.clone();
                                async move { Ok::<_, Infallible>(r.answer(req).await) }
                            }),
                        )
                        .await;
                });
            }
        });
        let acceptor = TlsAcceptor::from(config);
        let r = routes.clone();
        runtime.spawn(async move {
            loop {
                let Ok((tcp, _)) = tls_listener.accept().await else {
                    return;
                };
                let (r, acceptor) = (r.clone(), acceptor.clone());
                tokio::spawn(async move {
                    let Ok(tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let _ = Builder::new(TokioExecutor::new())
                        .serve_connection_with_upgrades(
                            TokioIo::new(tls),
                            service_fn(move |req| {
                                let r = r.clone();
                                async move { Ok::<_, Infallible>(r.answer(req).await) }
                            }),
                        )
                        .await;
                });
            }
        });
        let s = seen.clone();
        runtime.spawn(async move {
            let mut buf = [0u8; 2048];
            while udp.recv_from(&mut buf).await.is_ok() {
                s.datagrams.fetch_add(1, Ordering::Relaxed);
            }
        });
        Site {
            ip,
            plain,
            tls,
            ca,
            seen,
            _runtime: runtime,
        }
    }

    /// The page, which does the rest itself.
    pub fn page(&self) -> String {
        format!("http://{}/", host_port(self.ip, self.plain))
    }

    pub fn tls_origin(&self) -> String {
        format!("https://{}", host_port(self.ip, self.tls))
    }

    /// What the page said last, by name, once it has; and forgets it.
    pub fn take_report(&self) -> Option<BTreeMap<String, String>> {
        self.seen.done.lock().unwrap().take()
    }

    /// How many datagrams reached the HTTPS server's port.
    pub fn datagrams(&self) -> u64 {
        self.seen.datagrams.load(Ordering::Relaxed)
    }
}

fn host_port(ip: IpAddr, port: u16) -> String {
    match ip {
        IpAddr::V4(ip) => format!("{ip}:{port}"),
        IpAddr::V6(ip) => format!("[{ip}]:{port}"),
    }
}

struct Routes {
    seen: Arc<Seen>,
    tls_origin: String,
    tls_port: u16,
}

fn text(kind: &str) -> http::response::Builder {
    Response::builder().header(header::CONTENT_TYPE, kind)
}

fn full(body: impl Into<Bytes>) -> Body {
    Full::new(body.into()).boxed()
}

impl Routes {
    async fn answer(&self, req: Request<Incoming>) -> Response<Body> {
        let cors = |builder: http::response::Builder| {
            builder
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .header(header::ACCESS_CONTROL_EXPOSE_HEADERS, "alt-svc")
                .header(
                    header::ALT_SVC,
                    format!("h3=\":{}\"; ma=86400", self.tls_port),
                )
        };
        match req.uri().path() {
            "/" => text("text/html; charset=utf-8")
                .body(full(page(&self.tls_origin)))
                .unwrap(),
            "/json" => cors(text("application/json"))
                .body(full(format!(
                    r#"{{"answered":"over {:?}"}}"#,
                    req.version()
                )))
                .unwrap(),
            "/sse" => {
                let events: String = EVENTS
                    .iter()
                    .map(|event| format!("data: {event}\n\n"))
                    .collect();
                cors(text("text/event-stream"))
                    .header(header::CACHE_CONTROL, "no-store")
                    .body(full(events))
                    .unwrap()
            }
            "/ws" => websocket(req),
            "/done" => {
                let report = parse_query(req.uri().query().unwrap_or(""));
                *self.seen.done.lock().unwrap() = Some(report);
                Response::builder()
                    .status(StatusCode::NO_CONTENT)
                    .body(full(""))
                    .unwrap()
            }
            _ => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(full(""))
                .unwrap(),
        }
    }
}

/// An echo, over HTTP/1.1's upgrade.
fn websocket(req: Request<Incoming>) -> Response<Body> {
    let Some(key) = req.headers().get(header::SEC_WEBSOCKET_KEY) else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(full("not a WebSocket"))
            .unwrap();
    };
    let accept = derive_accept_key(key.as_bytes());
    tokio::spawn(async move {
        let Ok(upgraded) = hyper::upgrade::on(req).await else {
            return;
        };
        let mut ws =
            WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Server, None).await;
        while let Some(Ok(message)) = ws.next().await {
            if message.is_close() {
                break;
            }
            if (message.is_text() || message.is_binary()) && ws.send(message).await.is_err() {
                break;
            }
        }
    });
    Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::CONNECTION, "upgrade")
        .header(header::UPGRADE, "websocket")
        .header(header::SEC_WEBSOCKET_ACCEPT, accept)
        .body(full(""))
        .unwrap()
}

/// `a=1&b=x%20y`, by name. `+` is a space, as a browser's
/// `URLSearchParams` writes one.
fn parse_query(query: &str) -> BTreeMap<String, String> {
    fn decode(text: &str) -> String {
        let bytes = text.replace('+', " ").into_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%'
                && let Some(byte) = bytes
                    .get(i + 1..i + 3)
                    .and_then(|hex| std::str::from_utf8(hex).ok())
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            {
                out.push(byte);
                i += 3;
                continue;
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(name), decode(value))
        })
        .collect()
}

/// The page: each thing tried in turn, each one's outcome or its error, and
/// all of it said at the end.
fn page(tls: &str) -> String {
    let wss = tls.replacen("https://", "wss://", 1);
    let [first, second, third] = EVENTS;
    format!(
        r#"<!doctype html>
<meta charset="utf-8">
<title>tracewing e2e</title>
<script>
const said = {{}};
const within = (ms, what) => Promise.race([what, new Promise((_, no) => setTimeout(() => no(new Error("timed out")), ms))]);
(async () => {{
  // The first request a new browser sends can go before it has read the
  // proxy it is to use, and fail at once: tried a few times, half a second
  // apart, before it counts. Every request after it already has the proxy.
  for (let attempt = 1; attempt <= 5; attempt++) {{
    try {{
      const answer = await within(10000, fetch("{tls}/json"));
      said.json = (await answer.json()).answered;
      said.alt_svc = answer.headers.get("alt-svc") || "";
      said.json_attempts = String(attempt);
      break;
    }} catch (e) {{
      said.json = "error: " + e;
      await new Promise((yes) => setTimeout(yes, 500));
    }}
  }}
  try {{
    said.ws = await within(10000, new Promise((yes, no) => {{
      const ws = new WebSocket("{wss}/ws");
      ws.onopen = () => ws.send("{PING}");
      ws.onmessage = (m) => {{ yes(m.data); ws.close(); }};
      ws.onerror = () => no(new Error("WebSocket failed"));
    }}));
  }} catch (e) {{ said.ws = "error: " + e; }}
  try {{
    said.sse = await within(10000, new Promise((yes, no) => {{
      const got = [];
      const events = new EventSource("{tls}/sse");
      events.onmessage = (m) => {{
        got.push(m.data);
        if (got.length === 3) {{ events.close(); yes(got.join(",")); }}
      }};
      events.onerror = () => {{ events.close(); no(new Error("EventSource failed after " + got)); }};
    }}));
  }} catch (e) {{ said.sse = "error: " + e; }}
  try {{
    await within(10000, fetch("https://tracewing.test/", {{ mode: "no-cors" }}));
    said.check = "reached";
  }} catch (e) {{ said.check = "error: " + e; }}
  said.user_agent = navigator.userAgent;
  await fetch("/done?" + new URLSearchParams(said));
}})();
</script>
<p>{first}, {second}, {third}</p>
"#
    )
}

/// A certificate for `ip`, from an authority of its own written to `ca_out`.
fn tls_config(ip: IpAddr, ca_out: &Path) -> ServerConfig {
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::default();
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "tracewing e2e site CA");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca_cert = ca_params.self_signed(&ca_key).unwrap();
    std::fs::write(ca_out, ca_cert.pem()).unwrap();
    let issuer = Issuer::new(ca_params, ca_key);
    let leaf_key = KeyPair::generate().unwrap();
    let leaf = CertificateParams::new(vec![ip.to_string()])
        .unwrap()
        .signed_by(&leaf_key, &issuer)
        .unwrap();
    let mut config =
        ServerConfig::builder_with_provider(Arc::new(tracewing_proxy::crypto_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(leaf.der().to_vec())],
                PrivateKeyDer::from(PrivatePkcs8KeyDer::from(leaf_key.serialize_der())),
            )
            .unwrap();
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    config
}
