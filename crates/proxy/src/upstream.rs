//! Connections to servers, made step by step so that each step is timed: the
//! name looked up, TCP, then TLS. What the server showed of itself, its address,
//! TLS version and certificates, goes with every response over the connection.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use http::Uri;
use hudsucker::rustls::{ClientConfig, pki_types::ServerName};
use hyper::rt::{Read, ReadBufCursor, Write};
use hyper_util::{
    client::legacy::connect::{Connected, Connection, HttpConnector, dns::Name},
    rt::TokioIo,
};
use std::{
    future::Future,
    io,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI64, Ordering},
    },
    task::{Context, Poll},
    time::Instant,
};
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};
use tower_service::Service as _;

/// One connection to a server, in every response that came over it.
#[derive(Debug)]
pub struct Upstream {
    /// Counted from 1 in each run.
    pub id: i64,
    /// When it began, in microseconds since 1970.
    pub started_us: i64,
    /// When it could carry a request.
    pub ready_us: i64,
    /// `None` for an address given as such.
    pub dns_us: Option<i64>,
    pub connect_us: i64,
    pub tls_us: Option<i64>,
    pub remote: SocketAddr,
    pub tls: Option<TlsFacts>,
    /// Its facts are written once, with the first flow to come back over it.
    told: AtomicBool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsFacts {
    pub version: String,
    pub cipher: String,
    pub alpn: Option<String>,
    /// The server's certificates in DER, its own first.
    pub certificates: Vec<Vec<u8>>,
}

impl Upstream {
    /// The first to ask gets what to write; the rest refer to it by `id`.
    pub fn tell_once(&self) -> Option<Vec<u8>> {
        (!self.told.swap(true, Ordering::Relaxed)).then(|| self.describe())
    }

    /// As JSON, which the inspector reads back.
    pub fn describe(&self) -> Vec<u8> {
        let mut out = serde_json::json!({ "remote": self.remote.to_string() });
        if let Some(tls) = &self.tls {
            out["tls"] = serde_json::json!({
                "version": tls.version,
                "cipher": tls.cipher,
                "alpn": tls.alpn,
                "certificates": tls
                    .certificates
                    .iter()
                    .map(|der| STANDARD.encode(der))
                    .collect::<Vec<_>>(),
            });
        }
        serde_json::to_vec(&out).unwrap_or_default()
    }
}

/// Makes the connections the proxy sends requests over.
#[derive(Clone)]
pub struct TimedConnector {
    tls: TlsConnector,
    next_id: Arc<AtomicI64>,
}

impl TimedConnector {
    pub fn new(mut config: ClientConfig) -> Self {
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Self {
            tls: TlsConnector::from(Arc::new(config)),
            next_id: Arc::new(AtomicI64::new(1)),
        }
    }
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

impl tower_service::Service<Uri> for TimedConnector {
    type Response = Stream;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Stream, BoxError>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), BoxError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let tls = self.tls.clone();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        Box::pin(async move { connect(uri, tls, id).await })
    }
}

async fn connect(uri: Uri, tls: TlsConnector, id: i64) -> Result<Stream, BoxError> {
    let secure = match uri.scheme_str() {
        Some("https") => true,
        Some("http") | None => false,
        Some(other) => return Err(format!("cannot connect for {other}://").into()),
    };
    let host = uri.host().ok_or("no host to connect to")?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let port = uri.port_u16().unwrap_or(if secure { 443 } else { 80 });
    let started_us = unix_micros();
    let began = Instant::now();

    // Looked up here to be timed; hyper's own connector then tries the
    // addresses, falling back from IPv6 to IPv4 when the first is slow.
    let (addresses, dns_us) = match host.parse::<IpAddr>() {
        Ok(ip) => (vec![SocketAddr::new(ip, port)], None),
        Err(_) => {
            let found: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
                .await
                .map_err(|e| format!("look up {host}: {e}"))?
                .collect();
            if found.is_empty() {
                return Err(format!("look up {host}: no addresses").into());
            }
            (found, Some(micros(began)))
        }
    };

    let connecting = Instant::now();
    let mut http = HttpConnector::new_with_resolver(Resolved(addresses));
    http.enforce_http(false);
    http.set_nodelay(true);
    let tcp = http
        .call(uri.clone())
        .await
        .map_err(|e| format!("connect to {host}:{port}: {}", error_text(&e)))?
        .into_inner();
    let remote = tcp.peer_addr()?;
    let connect_us = micros(connecting);

    let (stream, tls_us, facts) = if secure {
        let shaking = Instant::now();
        let name = ServerName::try_from(host.to_owned())
            .map_err(|_| format!("\"{host}\" is not a name TLS can check"))?;
        let stream = tls
            .connect(name, tcp)
            .await
            .map_err(|e| format!("TLS with {host}: {e}"))?;
        let tls_us = micros(shaking);
        let (_, session) = stream.get_ref();
        let facts = TlsFacts {
            version: session
                .protocol_version()
                .and_then(|v| v.as_str())
                .unwrap_or("TLS")
                .replace('v', " ")
                .replace('_', "."),
            cipher: session
                .negotiated_cipher_suite()
                .and_then(|s| s.suite().as_str())
                .unwrap_or_default()
                .to_owned(),
            alpn: session
                .alpn_protocol()
                .map(|p| String::from_utf8_lossy(p).into_owned()),
            certificates: session
                .peer_certificates()
                .unwrap_or_default()
                .iter()
                .map(|der| der.to_vec())
                .collect(),
        };
        (
            Inner::Tls(TokioIo::new(Box::new(stream))),
            Some(tls_us),
            Some(facts),
        )
    } else {
        (Inner::Plain(TokioIo::new(tcp)), None, None)
    };

    Ok(Stream {
        inner: stream,
        upstream: Arc::new(Upstream {
            id,
            started_us,
            ready_us: started_us + micros(began),
            dns_us,
            connect_us,
            tls_us,
            remote,
            tls: facts,
            told: AtomicBool::new(false),
        }),
    })
}

/// Addresses already looked up, handed to hyper's connector.
#[derive(Clone)]
struct Resolved(Vec<SocketAddr>);

impl tower_service::Service<Name> for Resolved {
    type Response = std::vec::IntoIter<SocketAddr>;
    type Error = io::Error;
    type Future = std::future::Ready<io::Result<Self::Response>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: Name) -> Self::Future {
        std::future::ready(Ok(self.0.clone().into_iter()))
    }
}

/// An error and what caused it, on one line.
fn error_text(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

fn micros(since: Instant) -> i64 {
    since.elapsed().as_micros() as i64
}

fn unix_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_micros() as i64)
}

enum Inner {
    Plain(TokioIo<TcpStream>),
    Tls(TokioIo<Box<TlsStream<TcpStream>>>),
}

pub struct Stream {
    inner: Inner,
    upstream: Arc<Upstream>,
}

impl Connection for Stream {
    fn connected(&self) -> Connected {
        let connected = Connected::new().extra(Arc::clone(&self.upstream));
        let h2 = self
            .upstream
            .tls
            .as_ref()
            .is_some_and(|tls| tls.alpn.as_deref() == Some("h2"));
        if h2 {
            connected.negotiated_h2()
        } else {
            connected
        }
    }
}

impl Read for Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        match &mut self.get_mut().inner {
            Inner::Plain(s) => Pin::new(s).poll_read(cx, buf),
            Inner::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl Write for Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match &mut self.get_mut().inner {
            Inner::Plain(s) => Pin::new(s).poll_write(cx, buf),
            Inner::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().inner {
            Inner::Plain(s) => Pin::new(s).poll_flush(cx),
            Inner::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.get_mut().inner {
            Inner::Plain(s) => Pin::new(s).poll_shutdown(cx),
            Inner::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match &self.inner {
            Inner::Plain(s) => s.is_write_vectored(),
            Inner::Tls(s) => s.is_write_vectored(),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        match &mut self.get_mut().inner {
            Inner::Plain(s) => Pin::new(s).poll_write_vectored(cx, bufs),
            Inner::Tls(s) => Pin::new(s).poll_write_vectored(cx, bufs),
        }
    }
}
