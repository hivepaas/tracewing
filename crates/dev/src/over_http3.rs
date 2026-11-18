//! An HTTP/3 server and a client for the tests that need a real one, beside
//! the HTTPS origin this crate already serves.
//!
//! Here rather than in each test because there are now two kinds of test
//! that want a whole HTTP/3 exchange and no mock of one: the proxy's own,
//! which carry packets between a client and the door in memory, and
//! `local_live_http3`, which carries them through a real adapter. The
//! certificate is minted by **tracewing's own CA**, as the proxy mints the
//! ones it shows, so a client that trusts that CA trusts both ends.

use anyhow::{Context as _, Result};
use bytes::Buf as _;
use hudsucker::rustls::{self, RootCertStore};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tracewing_proxy::ca::{LocalCa, Over};

/// What the server answers, so that a test can tell its body from any other.
pub const SAID: &[u8] = b"this arrived over HTTP/3";

/// One request as the server saw it.
#[derive(Clone, Debug)]
pub struct Arrived {
    pub method: String,
    pub path: String,
    pub authority: String,
}

/// An HTTP/3 server on the loopback, holding a leaf for `host`.
pub struct Server {
    pub port: u16,
    saw: Arc<Mutex<Vec<Arrived>>>,
    /// Dropped, the endpoint stops listening.
    _endpoint: quinn::Endpoint,
}

impl Server {
    /// Starts it on a port of the operating system's choosing.
    ///
    /// **Both families on one port**, which is not a flourish: a client asked
    /// for `localhost` may go to `::1` first, and an endpoint on `0.0.0.0`
    /// never hears it -- the trap `docs/design/http3.md` records an hour to.
    pub fn start(ca: &Arc<LocalCa>, host: &str) -> Result<Self> {
        let config = ca.config_for(host, Over::Quic);
        let quic = quinn::crypto::rustls::QuicServerConfig::try_from(config)
            .context("the certificate for QUIC")?;
        let server = quinn::ServerConfig::with_crypto(Arc::new(quic));
        // Bound to the unspecified address of both families at once, so one
        // port answers `127.0.0.1` and `[::1]` alike.
        let endpoint = quinn::Endpoint::server(server, SocketAddr::from(([0u16; 8], 0)))
            .context("listen for QUIC")?;
        let port = endpoint.local_addr()?.port();
        let saw = Arc::new(Mutex::new(Vec::new()));
        tokio::spawn(accepting(endpoint.clone(), Arc::clone(&saw)));
        Ok(Self {
            port,
            saw,
            _endpoint: endpoint,
        })
    }

    /// The requests it has answered, in order.
    pub fn saw(&self) -> Vec<Arrived> {
        self.saw.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

async fn accepting(endpoint: quinn::Endpoint, saw: Arc<Mutex<Vec<Arrived>>>) {
    while let Some(incoming) = endpoint.accept().await {
        let saw = Arc::clone(&saw);
        tokio::spawn(async move {
            // Said rather than swallowed, and worded as what it usually
            // is: a client that has had its answer and gone. A way out
            // that makes an endpoint per connection drops it as soon as
            // the body is read, which this end sees as the peer going
            // away mid-connection.
            if let Err(e) = served(incoming, saw).await {
                eprintln!("the HTTP/3 server's connection ended: {e:#}");
            }
        });
    }
}

async fn served(incoming: quinn::Incoming, saw: Arc<Mutex<Vec<Arrived>>>) -> Result<()> {
    let conn = incoming.await?;
    let mut h3 = h3::server::Connection::new(h3_quinn::Connection::new(conn)).await?;
    while let Some(resolver) = h3.accept().await? {
        let saw = Arc::clone(&saw);
        tokio::spawn(async move {
            if let Err(e) = answered(resolver, saw).await {
                eprintln!("an HTTP/3 request: {e:#}");
            }
        });
    }
    Ok(())
}

async fn answered(
    resolver: h3::server::RequestResolver<h3_quinn::Connection, bytes::Bytes>,
    saw: Arc<Mutex<Vec<Arrived>>>,
) -> Result<()> {
    let (req, mut stream) = resolver.resolve_request().await?;
    saw.lock().unwrap_or_else(|e| e.into_inner()).push(Arrived {
        method: req.method().to_string(),
        path: req.uri().path().to_owned(),
        authority: req
            .uri()
            .authority()
            .map(|a| a.to_string())
            .unwrap_or_default(),
    });
    let answer = http::Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .header("content-length", SAID.len())
        .body(())
        .expect("a response of our own");
    stream.send_response(answer).await?;
    stream.send_data(bytes::Bytes::from_static(SAID)).await?;
    stream.finish().await?;
    Ok(())
}

/// A client's trust in one CA: the authority's own certificate, which is what
/// a leaf it minted leads to.
pub fn trusting(ca: &LocalCa) -> Result<RootCertStore> {
    let mut roots = RootCertStore::empty();
    roots.add(rustls::pki_types::CertificateDer::from(ca.cert_der()?))?;
    Ok(roots)
}

/// Asks `url` over HTTP/3, by sending to `at` and asking for the name `host`.
///
/// The three are given separately on purpose, because that is what a captured
/// client does: its packets go to an address that is not the server's, while
/// the name it asks for and the URL it wants are the server's own.
pub async fn ask(
    at: SocketAddr,
    host: &str,
    url: &str,
    roots: RootCertStore,
) -> Result<(http::StatusCode, Vec<u8>)> {
    let mut tls =
        rustls::ClientConfig::builder_with_provider(Arc::new(tracewing_proxy::crypto_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_root_certificates(roots)
            .with_no_client_auth();
    tls.alpn_protocols = vec![b"h3".to_vec()];
    let mut endpoint = quinn::Endpoint::client(match at.is_ipv4() {
        true => SocketAddr::from(([0, 0, 0, 0], 0)),
        false => SocketAddr::from(([0u16; 8], 0)),
    })?;
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
    )));
    let conn = endpoint
        .connect(at, host)
        .context("a QUIC connection")?
        .await
        .with_context(|| format!("the QUIC handshake with {at} as {host}"))?;
    let (mut driver, mut sender) = h3::client::new(h3_quinn::Connection::new(conn)).await?;
    let driving = tokio::spawn(async move { driver.wait_idle().await });
    let mut stream = sender
        .send_request(http::Request::get(url).body(()).context("a request")?)
        .await?;
    stream.finish().await?;
    let answer = stream.recv_response().await?;
    let mut body = Vec::new();
    while let Some(mut chunk) = stream.recv_data().await? {
        body.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
    }
    drop(sender);
    let _ = driving.await;
    Ok((answer.status(), body))
}
