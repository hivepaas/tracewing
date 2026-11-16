//! Connections to servers, made step by step so that each step is timed: the
//! name looked up, TCP, then TLS. What the server showed of itself, its address,
//! TLS version and certificates, goes with every response over the connection.
//! When TLS fails, the certificates the server showed go with the error.

use crate::client_certs::ClientCerts;
use crate::relay::{Kind as RelayKind, Relay, Route};
use crate::traffic_rules::TrafficRules;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use http::Uri;
use hudsucker::rustls::{
    self, ClientConfig, DigitallySignedStruct, SignatureScheme,
    client::{
        ResolvesClientCert,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
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
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering},
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
    /// The server asked for a client certificate, and tracewing had none for it.
    pub client_cert_missing: bool,
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
    /// Looked at as a connection is made, for a rule that says which
    /// version to ask that host with.
    traffic_rules: Arc<RwLock<TrafficRules>>,
}

impl TimedConnector {
    /// `verifier` must be the one `config` checks certificates with.
    pub fn new(
        mut config: ClientConfig,
        verifier: Arc<dyn ServerCertVerifier>,
        relay: Arc<Relay>,
        client_certs: Arc<ClientCerts>,
        traffic_rules: Arc<RwLock<TrafficRules>>,
    ) -> Self {
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        Self {
            config: Arc::new(config),
            verifier,
            next_id: Arc::new(AtomicI64::new(1)),
            relay,
            client_certs,
            traffic_rules,
        }
    }

    /// The TLS to agree with where `uri` points: what a rule says to ask
    /// that host with, or both versions as usual.
    fn config_for(&self, uri: &Uri) -> Arc<ClientConfig> {
        let Some(host) = uri.host() else {
            return Arc::clone(&self.config);
        };
        let secure = uri.scheme_str() != Some("http");
        let port = uri.port_u16().unwrap_or(if secure { 443 } else { 80 });
        let asked = self
            .traffic_rules
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .ask_host_with(host, port);
        match asked {
            None => Arc::clone(&self.config),
            Some(version) => {
                let mut config = (*self.config).clone();
                config.alpn_protocols = version.alpn();
                Arc::new(config)
            }
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
        let (config, verifier) = (self.config_for(&uri), Arc::clone(&self.verifier));
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
            let forwarded =
                !secure && !tunnel && route.kind == RelayKind::Http && !route.wants_ntlm();
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

    let (stream, tls_us, facts, client_cert_missing) = if secure {
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
        // Shown only if the server asks for one. With none, whether it asked
        // is kept, to say what to do when it then turns the connection down.
        let unanswered = Arc::new(Unanswered::default());
        config.client_auth_cert_resolver = match client_cert {
            Some(cert) => Arc::new(SingleCertAndKey::from(cert)),
            None => Arc::clone(&unanswered) as Arc<dyn ResolvesClientCert>,
        };
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
            unanswered.asked.load(Ordering::Relaxed),
        )
    } else {
        (Inner::Plain(TokioIo::new(tcp)), None, None, false)
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
            client_cert_missing,
            told: AtomicBool::new(false),
        }),
    })
}

/// No client certificate to give: says none, and keeps that one was asked for.
#[derive(Debug, Default)]
struct Unanswered {
    asked: AtomicBool,
}

impl ResolvesClientCert for Unanswered {
    fn resolve(&self, _: &[&[u8]], _: &[SignatureScheme]) -> Option<Arc<CertifiedKey>> {
        self.asked.store(true, Ordering::Relaxed);
        None
    }

    fn has_certs(&self) -> bool {
        false
    }
}

/// Which interface tracewing's own connections leave by while it holds a TUN
/// adapter — one answer for each family, because Windows numbers an adapter
/// separately for IPv4 and for IPv6.
///
/// They are usually the same number and they are not the same thing, so one
/// standing for both would pin IPv6 traffic to whatever happened to share
/// the IPv4 index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutBy {
    /// `None` means this family has **no** way out that is not our own
    /// adapter, which is not the same as having no preference.
    pub v4: Option<u32>,
    pub v6: Option<u32>,
}

/// Whether tracewing holds an adapter at all, and the interface for each
/// family while it does; zero for none.
///
/// Process-wide rather than per-connection, because it is not a property of
/// a connection. It is a property of this machine's routing while tracewing
/// holds a TUN adapter: with the adapter carrying a default route, a
/// connection tracewing makes on a captured program's behalf is routed back
/// into the adapter, answered by our own stack as the server it was going
/// to, and made again — a loop with nothing bounding it.
///
/// Nothing sets them yet, and nothing but Windows reads them.
static HOLDING: AtomicBool = AtomicBool::new(false);
static OUT_BY_V4: AtomicU32 = AtomicU32::new(0);
static OUT_BY_V6: AtomicU32 = AtomicU32::new(0);

/// Says which interface tracewing's own connections must leave by while it
/// holds an adapter, or `None` once it holds none and the routing table
/// decides again as it always did.
///
/// **Both families in one call on purpose.** Setting one and forgetting the
/// other is the mistake this shape makes impossible to write, and it is the
/// expensive one: a forgotten IPv6 does not fail, it loops.
///
/// **Whoever calls this owns keeping it right.** Which interface holds the
/// default route changes as a machine moves between Wi-Fi, Ethernet and
/// tethering, so it is called again when that changes rather than once at
/// the start.
///
/// And it is not free: the option *forces* traffic out that interface,
/// including traffic that would have left by a more specific route, so while
/// it is set every connection tracewing makes goes one way. See
/// [capture-modes.md](../../../docs/design/capture-modes.md).
pub fn leave_by(out: Option<OutBy>) {
    match out {
        // The indices go in **before** the flag that makes them read, so a
        // connection that sees an adapter held never reads an index nobody
        // wrote. The other order would let one connection be routed into the
        // adapter we are holding, which is the loop this exists to stop.
        Some(out) => {
            OUT_BY_V4.store(out.v4.unwrap_or(0), Ordering::Relaxed);
            OUT_BY_V6.store(out.v6.unwrap_or(0), Ordering::Relaxed);
            HOLDING.store(true, Ordering::Release);
        }
        // And the flag goes out first on the way down, so the worst a
        // connection can see is a family with no way out — which fails,
        // rather than loops.
        None => {
            HOLDING.store(false, Ordering::Release);
            OUT_BY_V4.store(0, Ordering::Relaxed);
            OUT_BY_V6.store(0, Ordering::Relaxed);
        }
    }
}

/// What a connection of one family has to do about all that.
#[cfg(any(windows, test))]
#[derive(Debug, PartialEq, Eq)]
enum Out {
    /// No adapter is held, so the routing table decides as it always has.
    AsAlways,
    /// Out by this interface.
    By(u32),
    /// An adapter **is** held and this family has no way out that is not it,
    /// so connecting would be routed straight back into tracewing. It must
    /// not be tried.
    Nowhere,
}

/// The decision, taken apart from where the answer is kept so that it can be
/// tested without a global: tests run beside each other, and shared state
/// read by several of them is how a suite starts failing one time in ten.
#[cfg(any(windows, test))]
fn out_from(holding: bool, index: u32) -> Out {
    match (holding, index) {
        (false, _) => Out::AsAlways,
        (true, 0) => Out::Nowhere,
        (true, index) => Out::By(index),
    }
}

#[cfg(windows)]
fn out_for(v6: bool) -> Out {
    let holding = HOLDING.load(Ordering::Acquire);
    let index = match v6 {
        true => OUT_BY_V6.load(Ordering::Relaxed),
        false => OUT_BY_V4.load(Ordering::Relaxed),
    };
    out_from(holding, index)
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
    // Windows, and only once something has pinned an interface: hyper's
    // connector cannot be told which one to leave by there, so that one case
    // connects for itself. Every other platform, and Windows with nothing
    // pinned, goes through hyper exactly as before.
    #[cfg(windows)]
    if HOLDING.load(Ordering::Acquire) {
        let tcp = picking::first_to_answer(addresses.clone(), pinned)
            .await
            .map_err(|e| format!("connect to {host}:{port}: {e}"))?;
        return Ok((tcp, dns_us, micros(connecting)));
    }
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

/// Which interface to leave by, or why there is none to leave by.
///
/// Shared by everything that must not go back into the adapter: a connection
/// the proxy makes, a connection spliced past it, and a datagram passed on.
/// `about` is what is being reached, for the message.
#[cfg(windows)]
fn out_by(v6: bool, about: &dyn std::fmt::Display) -> io::Result<u32> {
    match out_for(v6) {
        Out::By(index) => Ok(index),
        // Nothing asks for a pinned socket unless an adapter is held, so
        // this is unreachable — said rather than assumed, because the thing
        // it would otherwise do silently is go out unpinned, and that is the
        // loop.
        Out::AsAlways => Err(io::Error::other("nothing is pinned")),
        Out::Nowhere => {
            let family = match v6 {
                true => "IPv6",
                false => "IPv4",
            };
            Err(io::Error::other(format!(
                "tracewing holds a TUN adapter and {family} has no way out that is not it, so \
                 reaching {about} would route back into tracewing"
            )))
        }
    }
}

/// Connects to `addr` on a socket pinned to the interface the machine's own
/// traffic leaves by.
#[cfg(windows)]
async fn pinned(addr: SocketAddr) -> io::Result<TcpStream> {
    let index = out_by(addr.is_ipv6(), &addr)?;
    let socket = match addr {
        SocketAddr::V4(_) => tokio::net::TcpSocket::new_v4()?,
        SocketAddr::V6(_) => tokio::net::TcpSocket::new_v6()?,
    };
    leave_socket_by(handle(&socket), addr.is_ipv6(), index)?;
    let stream = socket.connect(addr).await?;
    // Set on the stream rather than the socket, for the same reason the
    // connector above sets it: small writes otherwise wait for delayed ACKs.
    stream.set_nodelay(true)?;
    Ok(stream)
}

/// A socket as [`leave_socket_by`] wants it. Written once so that neither
/// caller spells the cast, and so that the two kinds of socket -- one of
/// `tokio`'s, one of `std`'s -- reach it the same way.
#[cfg(windows)]
fn handle(socket: &impl std::os::windows::io::AsRawSocket) -> usize {
    socket.as_raw_socket() as usize
}

/// A datagram socket pinned the same way, for what [`crate::datagrams`]
/// passes on.
///
/// Bound to the family's unspecified address and a port of the operating
/// system's choosing: it is only ever `connect`ed afterwards, and a NAT's
/// source port is nobody's business.
#[cfg(windows)]
async fn pinned_datagrams(v6: bool) -> io::Result<tokio::net::UdpSocket> {
    let family = match v6 {
        true => "IPv6",
        false => "IPv4",
    };
    let index = out_by(v6, &family)?;
    let any: SocketAddr = match v6 {
        true => "[::]:0".parse().expect("a literal"),
        false => "0.0.0.0:0".parse().expect("a literal"),
    };
    // Through `std`, because the option has to be set before anything is
    // sent and `tokio` has no socket of its own to set it on before binding.
    let socket = std::net::UdpSocket::bind(any)?;
    leave_socket_by(handle(&socket), v6, index)?;
    socket.set_nonblocking(true)?;
    tokio::net::UdpSocket::from_std(socket)
}

/// The way out for a connection nobody chose, for [`crate::sieve`].
#[cfg(windows)]
pub fn straight_to() -> crate::sieve::StraightTo {
    Box::new(|to| {
        Box::pin(async move {
            pinned(to)
                .await
                .map(|stream| Box::new(stream) as Box<dyn crate::source::Duplex>)
        })
    })
}

/// The way out for a datagram nobody answers here, for [`crate::datagrams`].
#[cfg(windows)]
pub fn bound_datagrams() -> crate::datagrams::Bind {
    std::sync::Arc::new(|v6| Box::pin(pinned_datagrams(v6)))
}

/// Sets `IP_UNICAST_IF`, or `IPV6_UNICAST_IF` for an IPv6 socket.
///
/// **The two take the index in different byte orders**, which is the trap
/// here and why they are one function with the difference written down.
/// Microsoft's documentation says the IPv4 option "must be an interface
/// index in network byte order", and that reading it back gives host byte
/// order; the IPv6 option takes the index as it is.
///
/// Getting the IPv4 order wrong is **usually loud**, which was worth finding
/// out by trying it: the option reads those four bytes as an IPv4 address
/// and treats them as an index only when the first octet is zero. So a
/// network-order 1 arrives as `0.0.0.1` and means interface 1, while a
/// host-order 1 arrives as the address `1.0.0.0` and Windows answers
/// `WSAEADDRNOTAVAIL`. It is quiet only for an index that is a multiple of
/// 256, where both orders still look like an index and the wrong one simply
/// names a different interface — which is the case the round trip in
/// `pinning` is there for.
#[cfg(windows)]
fn leave_socket_by(socket: usize, v6: bool, index: u32) -> io::Result<()> {
    use windows_sys::Win32::Networking::WinSock::{
        IP_UNICAST_IF, IPPROTO_IP, IPPROTO_IPV6, IPV6_UNICAST_IF, setsockopt,
    };
    let (level, name, index) = match v6 {
        true => (IPPROTO_IPV6, IPV6_UNICAST_IF, index),
        false => (IPPROTO_IP, IP_UNICAST_IF, index.to_be()),
    };
    // Bound, so that the pointer below outlives the call.
    let value = index.to_ne_bytes();
    // SAFETY: a socket this call only reads, and four bytes of ours whose
    // length is given as four.
    let set = unsafe { setsockopt(socket, level, name, value.as_ptr(), value.len() as i32) };
    match set {
        0 => Ok(()),
        _ => Err(io::Error::last_os_error()),
    }
}

/// The byte order in [`leave_socket_by`] is the whole risk there: getting it
/// wrong does not fail, it pins the wrong interface. Windows hands the index
/// back in **host** byte order however it was set, so a round trip is what
/// says which way we set it. Neither of these wants a privilege, so the
/// Windows job runs them.
///
/// A datagram socket is in here for a reason of its own. The option is the
/// same option, but it is set on a socket that is bound before it is set
/// rather than after, and on a `std` socket rather than one of `tokio`'s --
/// so "the same call works" is a thing to measure rather than to assume.
#[cfg(all(windows, test))]
mod pinning {
    use super::{handle, leave_socket_by};

    /// Loopback, which every Windows machine numbers and has.
    const SOME_INTERFACE: u32 = 1;

    fn read_back(socket: &impl std::os::windows::io::AsRawSocket, v6: bool) -> u32 {
        use windows_sys::Win32::Networking::WinSock::{
            IP_UNICAST_IF, IPPROTO_IP, IPPROTO_IPV6, IPV6_UNICAST_IF, getsockopt,
        };
        let (level, name) = match v6 {
            true => (IPPROTO_IPV6, IPV6_UNICAST_IF),
            false => (IPPROTO_IP, IP_UNICAST_IF),
        };
        let mut value = [0u8; 4];
        let mut len = value.len() as i32;
        // SAFETY: four bytes of ours, with four given as their length.
        let got = unsafe { getsockopt(handle(socket), level, name, value.as_mut_ptr(), &mut len) };
        assert_eq!(
            got,
            0,
            "reading the option back: {}",
            std::io::Error::last_os_error()
        );
        u32::from_ne_bytes(value)
    }

    #[tokio::test]
    async fn an_ipv4_index_comes_back_as_it_went_in() {
        let socket = tokio::net::TcpSocket::new_v4().expect("an IPv4 socket");
        leave_socket_by(handle(&socket), false, SOME_INTERFACE).expect("the option was set");
        assert_eq!(
            read_back(&socket, false),
            SOME_INTERFACE,
            "the IPv4 option is set in network byte order and read in host"
        );
    }

    #[tokio::test]
    async fn an_ipv6_index_comes_back_as_it_went_in() {
        let socket = tokio::net::TcpSocket::new_v6().expect("an IPv6 socket");
        leave_socket_by(handle(&socket), true, SOME_INTERFACE).expect("the option was set");
        assert_eq!(
            read_back(&socket, true),
            SOME_INTERFACE,
            "the IPv6 option is set and read in host byte order"
        );
    }

    /// A datagram socket takes the option the same way, in both families --
    /// bound first, and a `std` socket rather than one of `tokio`'s.
    #[tokio::test]
    async fn a_datagram_socket_takes_the_option_too() {
        for (v6, any) in [(false, "0.0.0.0:0"), (true, "[::]:0")] {
            let socket = std::net::UdpSocket::bind(any).expect("a datagram socket");
            leave_socket_by(handle(&socket), v6, SOME_INTERFACE).expect("the option was set");
            assert_eq!(
                read_back(&socket, v6),
                SOME_INTERFACE,
                "a datagram socket pinned, {}",
                match v6 {
                    true => "IPv6",
                    false => "IPv4",
                }
            );
        }
    }
}

/// Trying the addresses in turn, which is hyper's job everywhere but on the
/// pinned path above.
///
/// Gated on `test` as well as `windows` so that the trying is tested on
/// every platform rather than only on the one that runs it: none of it is
/// about Windows, and Linux is what sees every change.
#[cfg(any(windows, test))]
mod picking {
    use std::{future::Future, io, net::SocketAddr, time::Duration};

    /// How long each address gets before the next is started as well. The
    /// number is hyper's own default for the same thing, so that the one
    /// platform which cannot use hyper's connector behaves like the rest.
    const EACH_WITHIN: Duration = Duration::from_millis(300);

    /// What one wait came to.
    enum Waited<T> {
        Made(T),
        Failed(io::Error),
        /// This address's turn is over, or there was nothing left to wait on.
        Over,
    }

    /// Connects to the first of `addresses` that answers, starting them a
    /// turn apart so that one slow address does not hold up the rest. That
    /// is what hyper calls happy eyeballs, and it is what falls back from
    /// IPv6 to IPv4.
    ///
    /// Written over a closure rather than straight over `TcpStream` so that
    /// the trying can be tested where it can be run.
    pub async fn first_to_answer<T, F, Fut>(addresses: Vec<SocketAddr>, connect: F) -> io::Result<T>
    where
        T: Send + 'static,
        F: Fn(SocketAddr) -> Fut,
        Fut: Future<Output = io::Result<T>> + Send + 'static,
    {
        let mut trying: tokio::task::JoinSet<io::Result<T>> = tokio::task::JoinSet::new();
        let mut waiting = addresses.into_iter().peekable();
        let mut last: Option<io::Error> = None;
        while waiting.peek().is_some() || !trying.is_empty() {
            if let Some(addr) = waiting.next() {
                trying.spawn(connect(addr));
            }
            // With none left to start, whatever is running has as long as it
            // needs: there is nothing else to give a turn to.
            let turn = waiting.peek().map(|_| EACH_WITHIN);
            match waited(&mut trying, turn).await {
                Waited::Made(made) => return Ok(made),
                Waited::Failed(e) => last = Some(e),
                Waited::Over => {}
            }
        }
        Err(last.unwrap_or_else(|| io::Error::other("no address to connect to")))
    }

    async fn waited<T: Send + 'static>(
        trying: &mut tokio::task::JoinSet<io::Result<T>>,
        within: Option<Duration>,
    ) -> Waited<T> {
        let next = async {
            match trying.join_next().await {
                Some(Ok(Ok(made))) => Waited::Made(made),
                Some(Ok(Err(e))) => Waited::Failed(e),
                // A connect that panicked is not a connection; it counts as
                // one more address that did not answer.
                Some(Err(e)) => Waited::Failed(io::Error::other(e)),
                None => Waited::Over,
            }
        };
        match within {
            Some(within) => tokio::time::timeout(within, next)
                .await
                .unwrap_or(Waited::Over),
            None => next.await,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::{Arc, Mutex};

        fn addr(last: u8) -> SocketAddr {
            format!("10.0.0.{last}:443").parse().unwrap()
        }

        /// What was tried, in the order it was tried, so that a test can say
        /// not only what came back but what it cost to get there.
        #[derive(Clone, Default)]
        struct Tried(Arc<Mutex<Vec<SocketAddr>>>);

        impl Tried {
            fn so_far(&self) -> Vec<SocketAddr> {
                self.0.lock().unwrap().clone()
            }

            fn note(&self, addr: SocketAddr) {
                self.0.lock().unwrap().push(addr);
            }
        }

        #[tokio::test]
        async fn the_first_to_answer_is_the_one_taken() {
            let tried = Tried::default();
            let noting = tried.clone();
            let got = first_to_answer(vec![addr(1), addr(2)], move |a| {
                let noting = noting.clone();
                async move {
                    noting.note(a);
                    Ok(a)
                }
            })
            .await
            .expect("the first answered");
            assert_eq!(got, addr(1));
            assert_eq!(
                tried.so_far(),
                vec![addr(1)],
                "the second was never started, having not been needed"
            );
        }

        #[tokio::test]
        async fn an_address_that_will_not_answer_hands_on_to_the_next() {
            let tried = Tried::default();
            let noting = tried.clone();
            let got = first_to_answer(vec![addr(1), addr(2)], move |a| {
                let noting = noting.clone();
                async move {
                    noting.note(a);
                    match a == addr(1) {
                        true => Err(io::Error::other("no route")),
                        false => Ok(a),
                    }
                }
            })
            .await
            .expect("the second answered");
            assert_eq!(got, addr(2));
            assert_eq!(tried.so_far(), vec![addr(1), addr(2)]);
        }

        /// The whole reason this is not a loop of one connect after another.
        #[tokio::test]
        async fn a_slow_address_does_not_hold_up_the_one_behind_it() {
            let began = std::time::Instant::now();
            // Bounded, so that going back to one connect after another is a
            // failure here rather than a suite that never finishes.
            let got = tokio::time::timeout(
                Duration::from_secs(5),
                first_to_answer(vec![addr(1), addr(2)], move |a| async move {
                    if a == addr(1) {
                        // Never answers, as a filtered address does.
                        std::future::pending::<()>().await;
                    }
                    Ok(a)
                }),
            )
            .await
            .expect("the first did not hold the second up for ever")
            .expect("the second answered while the first still hung");
            assert_eq!(got, addr(2));
            assert!(
                began.elapsed() >= EACH_WITHIN,
                "the second waited its turn rather than starting at once"
            );
        }

        #[tokio::test]
        async fn the_last_thing_that_went_wrong_is_what_comes_back() {
            let failed = first_to_answer(vec![addr(1), addr(2)], move |a| async move {
                Err::<SocketAddr, _>(io::Error::other(match a == addr(1) {
                    true => "the first went wrong",
                    false => "the second went wrong",
                }))
            })
            .await
            .expect_err("neither answered");
            assert!(failed.to_string().contains("the second"), "said: {failed}");
        }

        #[tokio::test]
        async fn no_address_at_all_is_an_error_rather_than_a_wait() {
            let failed = first_to_answer(Vec::new(), move |a| async move {
                Ok::<SocketAddr, io::Error>(a)
            })
            .await
            .expect_err("there was nothing to try");
            assert!(failed.to_string().contains("no address"), "said: {failed}");
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_no_adapter_held_the_routing_table_decides() {
        assert_eq!(out_from(false, 0), Out::AsAlways);
        assert_eq!(
            out_from(false, 7),
            Out::AsAlways,
            "even with an index left over from before"
        );
    }

    #[test]
    fn an_adapter_held_and_a_way_out_is_that_way() {
        assert_eq!(out_from(true, 7), Out::By(7));
    }

    /// The case that has to fail rather than fall back. Falling back means
    /// the routing table decides, and while an adapter is held the routing
    /// table decides on the adapter — which is the loop.
    #[test]
    fn an_adapter_held_with_no_way_out_is_nowhere_and_not_as_always() {
        assert_eq!(out_from(true, 0), Out::Nowhere);
    }
}
