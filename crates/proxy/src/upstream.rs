//! Connections to servers, made step by step so that each step is timed: the
//! name looked up, TCP, then TLS. What the server showed of itself, its address,
//! TLS version and certificates, goes with every response over the connection.
//! When TLS fails, the certificates the server showed go with the error.

use crate::client_certs::ClientCerts;
use crate::relay::{Kind as RelayKind, Relay, Route};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use http::Uri;
use hudsucker::rustls::{
    self, ClientConfig, DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
    sign::{CertifiedKey, SingleCertAndKey},
};
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
        Arc, Mutex,
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

/// A TLS handshake that failed, with what the server showed before it did.
#[derive(Debug)]
pub struct TlsFailure {
    pub id: i64,
    pub host: String,
    pub remote: SocketAddr,
    pub message: String,
    /// In DER, the server's own first. Empty if it showed none.
    pub certificates: Vec<Vec<u8>>,
}

impl std::fmt::Display for TlsFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TLS with {}: {}", self.host, self.message)
    }
}

impl std::error::Error for TlsFailure {}

impl TlsFailure {
    /// As JSON, the way [`Upstream::describe`] writes a connection.
    pub fn describe(&self) -> Vec<u8> {
        let out = serde_json::json!({
            "remote": self.remote.to_string(),
            "tls": {
                "error": self.message,
                "certificates": self
                    .certificates
                    .iter()
                    .map(|der| STANDARD.encode(der))
                    .collect::<Vec<_>>(),
            },
        });
        serde_json::to_vec(&out).unwrap_or_default()
    }
}

/// Checks a server's certificates as `inner` does, and keeps them, so that
/// ones refused can still be shown.
#[derive(Debug)]
struct Witness {
    inner: Arc<dyn ServerCertVerifier>,
    seen: Mutex<Vec<Vec<u8>>>,
}

impl Witness {
    fn take(&self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.seen.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

impl ServerCertVerifier for Witness {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        *self.seen.lock().unwrap_or_else(|e| e.into_inner()) = std::iter::once(end_entity)
            .chain(intermediates)
            .map(|der| der.to_vec())
            .collect();
        self.inner
            .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

/// Makes the connections the proxy sends requests over.
#[derive(Clone)]
pub struct TimedConnector {
    config: Arc<ClientConfig>,
    verifier: Arc<dyn ServerCertVerifier>,
    next_id: Arc<AtomicI64>,
    /// Another proxy to go through, as a company's.
    relay: Arc<Relay>,
    /// Certificates for servers that ask the client for one.
    client_certs: Arc<ClientCerts>,
}

impl TimedConnector {
    /// `verifier` must be the one `config` checks certificates with.
    pub fn new(
        mut config: ClientConfig,
        verifier: Arc<dyn ServerCertVerifier>,
        relay: Arc<Relay>,
        client_certs: Arc<ClientCerts>,
    ) -> Self {
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Self {
            config: Arc::new(config),
            verifier,
            next_id: Arc::new(AtomicI64::new(1)),
            relay,
            client_certs,
        }
    }

    /// The certificate to show the server `uri` points to, if it asks.
    fn client_cert(&self, uri: &Uri) -> Option<Arc<CertifiedKey>> {
        let host = uri.host()?;
        self.client_certs
            .for_host(host.trim_start_matches('[').trim_end_matches(']'))
    }
}

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

impl TimedConnector {
    /// A connection to where `uri` points, as any other, a relay included, that
    /// speaks HTTP/1.1 alone: a WebSocket's upgrade is HTTP/1.1's. Through an
    /// HTTP relay it is always a tunnel, as browsers have it: a relay passes
    /// requests on, not upgrades.
    pub async fn connect_http1(&self, uri: Uri) -> Result<Stream, BoxError> {
        let mut config = (*self.config).clone();
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let route = uri.host().and_then(|host| {
            self.relay
                .route_for(host.trim_start_matches('[').trim_end_matches(']'))
        });
        let client_cert = self.client_cert(&uri);
        connect(
            uri,
            Arc::new(config),
            Arc::clone(&self.verifier),
            id,
            route,
            true,
            client_cert,
        )
        .await
    }
}

impl tower_service::Service<Uri> for TimedConnector {
    type Response = Stream;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Stream, BoxError>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), BoxError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let (config, verifier) = (Arc::clone(&self.config), Arc::clone(&self.verifier));
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let route = uri.host().and_then(|host| {
            self.relay
                .route_for(host.trim_start_matches('[').trim_end_matches(']'))
        });
        let client_cert = self.client_cert(&uri);
        Box::pin(async move { connect(uri, config, verifier, id, route, false, client_cert).await })
    }
}

async fn connect(
    uri: Uri,
    config: Arc<ClientConfig>,
    verifier: Arc<dyn ServerCertVerifier>,
    id: i64,
    route: Option<Arc<Route>>,
    tunnel: bool,
    client_cert: Option<Arc<CertifiedKey>>,
) -> Result<Stream, BoxError> {
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

    // Through a relay, the name goes to it as it is, for it to look up. Plain
    // HTTP goes to an HTTP relay as a request to pass on, which is what such
    // a relay allows; anything else goes through a tunnel.
    let (tcp, remote, dns_us, connect_us, forwarded) = match &route {
        Some(route) => {
            let connecting = Instant::now();
            let forwarded = !secure && !tunnel && route.kind == RelayKind::Http;
            let tcp = if forwarded {
                route.connect().await
            } else {
                route.tunnel(host, port).await
            }?;
            let remote = tcp.peer_addr()?;
            (tcp, remote, None, micros(connecting), forwarded)
        }
        None => {
            let (tcp, dns_us, connect_us) = connect_straight(&uri, host, port, began).await?;
            let remote = tcp.peer_addr()?;
            (tcp, remote, dns_us, connect_us, false)
        }
    };

    let (stream, tls_us, facts) = if secure {
        let shaking = Instant::now();
        let name = ServerName::try_from(host.to_owned())
            .map_err(|_| format!("\"{host}\" is not a name TLS can check"))?;
        // A config of its own, to keep what this server shows. It shares the
        // rest, sessions to resume among them.
        let witness = Arc::new(Witness {
            inner: verifier,
            seen: Mutex::new(Vec::new()),
        });
        let mut config = (*config).clone();
        config
            .dangerous()
            .set_certificate_verifier(Arc::clone(&witness) as Arc<dyn ServerCertVerifier>);
        // Shown only if the server asks for one.
        if let Some(cert) = client_cert {
            config.client_auth_cert_resolver = Arc::new(SingleCertAndKey::from(cert));
        }
        let stream = TlsConnector::from(Arc::new(config))
            .connect(name, tcp)
            .await
            .map_err(|e| TlsFailure {
                id,
                host: host.to_owned(),
                remote,
                message: e.to_string(),
                certificates: witness.take(),
            })?;
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
        forwarded,
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

/// A connection to the server itself: the name looked up, then TCP.
async fn connect_straight(
    uri: &Uri,
    host: &str,
    port: u16,
    began: Instant,
) -> Result<(TcpStream, Option<i64>, i64), BoxError> {
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
    Ok((tcp, dns_us, micros(connecting)))
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
    /// To an HTTP relay, which takes requests whole, their URL in full.
    forwarded: bool,
}

impl Stream {
    /// The connection it is, with what the server showed of itself.
    pub fn upstream(&self) -> &Arc<Upstream> {
        &self.upstream
    }
}

impl Connection for Stream {
    fn connected(&self) -> Connected {
        let connected = Connected::new()
            .extra(Arc::clone(&self.upstream))
            .proxy(self.forwarded);
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
