//! The QUIC front door: HTTP/3 in, and the same core behind it.
//!
//! The proxy's other front door is [hudsucker](https://docs.rs/hudsucker),
//! which owns its accept loop and takes a `TcpListener`. HTTP/3 arrives on a
//! UDP socket instead, so it needs a door of its own -- and then hands what
//! arrives to exactly the same place, [`FlowHandler::request`], so that rules,
//! scripts, breakpoints, the slow network and the flow a request is written
//! down as are one implementation and not two. See `docs/design/http3.md`.
//!
//! What is here: the endpoint, the accept loop, a certificate chosen per
//! handshake (QUIC carries the name inside its first packet, so there is no
//! `CONNECT` to learn it from -- see [`LocalCa::quic_config`]), and the two
//! bodies of a request, carried by [`crate::quic_body`].
//!
//! The way to a server is [`crate::quic_upstream`], handed to [`Door::serve`]
//! rather than built here: the door and the way out are two things, and a
//! test of one should not have to stand up the other.
//!
//! **Nothing opens this door yet.** No way in points at it and
//! `datagrams.rs` still refuses UDP 443, so no client's traffic reaches it
//! outside the tests. Giving a chosen program's UDP 443 to it is the next
//! change; see `docs/design/http3.md`.

use crate::{
    ca::LocalCa,
    capture::FlowHandler,
    quic_body::{carried, poured},
    quic_upstream::Way,
    source::{Stop, stopped},
};
use anyhow::{Context as _, Result};
use bytes::Bytes;
use hudsucker::{Body, RequestOrResponse};
use std::{net::SocketAddr, sync::Arc};

type Served = h3::server::RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>;
type Answering = h3::server::RequestStream<h3_quinn::SendStream<Bytes>, Bytes>;
type Reading = h3::server::RequestStream<h3_quinn::RecvStream, Bytes>;

/// A QUIC endpoint serving HTTP/3.
pub struct Door {
    endpoint: quinn::Endpoint,
}

impl Door {
    /// Binds an endpoint at `at`, with certificates minted by `ca` as each
    /// client asks for a name.
    pub fn open(at: SocketAddr, ca: &Arc<LocalCa>) -> Result<Self> {
        let quic = quinn::crypto::rustls::QuicServerConfig::try_from(ca.quic_config()?)
            .context("the certificate for QUIC")?;
        let endpoint =
            quinn::Endpoint::server(quinn::ServerConfig::with_crypto(Arc::new(quic)), at)
                .with_context(|| format!("listen for QUIC on {at}"))?;
        Ok(Self { endpoint })
    }

    /// Where it listens, which a test asks for and a way in will.
    pub fn at(&self) -> Result<SocketAddr> {
        Ok(self.endpoint.local_addr()?)
    }

    /// Serves until `stop`, each connection on its own task, sending what
    /// is meant for a server over `out`.
    pub async fn serve(self, handler: FlowHandler, out: Arc<Way>, stop: Stop) {
        loop {
            let waiting = tokio::select! {
                waiting = self.endpoint.accept() => waiting,
                () = stopped(stop.clone()) => break,
            };
            let Some(incoming) = waiting else { break };
            let handler = handler.clone();
            let out = Arc::clone(&out);
            tokio::spawn(async move {
                let client = incoming.remote_address();
                if let Err(e) = connected(incoming, handler, out).await {
                    tracing::debug!("the QUIC connection from {client} ended: {e:#}");
                }
            });
        }
        // Each connection is told, rather than cut off: a client that is
        // mid-request gets an error it can read instead of a timeout.
        self.endpoint.close(0u32.into(), b"tracewing is stopping");
        self.endpoint.wait_idle().await;
    }
}

/// One connection, for as long as it brings requests.
async fn connected(incoming: quinn::Incoming, handler: FlowHandler, out: Arc<Way>) -> Result<()> {
    let conn = incoming.await?;
    let client = conn.remote_address();
    let mut h3 = h3::server::Connection::new(h3_quinn::Connection::new(conn)).await?;
    while let Some(resolver) = h3.accept().await? {
        // One handler per request, as hudsucker clones one per request: the
        // fields it fills in on the way through belong to that request.
        let mut handler = handler.clone();
        let out = Arc::clone(&out);
        tokio::spawn(async move {
            if let Err(e) = served(resolver, &mut handler, &out, client).await {
                tracing::debug!("an HTTP/3 request from {client} ended: {e:#}");
            }
        });
    }
    Ok(())
}

/// One request: through the core, and back over the stream it came on.
async fn served(
    resolver: h3::server::RequestResolver<h3_quinn::Connection, Bytes>,
    handler: &mut FlowHandler,
    out: &Way,
    client: SocketAddr,
) -> Result<()> {
    let (head, stream) = resolver.resolve_request().await?;
    let (mut answering, reading) = split(stream);
    let req = head.map(|()| carried(reading));
    let answer = match handler.request(client, req).await {
        RequestOrResponse::Response(res) => res,
        // Out over HTTP/3, and whatever went wrong on the way is the flow's
        // and the client's to be told -- which is what `failed` does with it.
        RequestOrResponse::Request(req) => match out.send(req).await {
            Ok(res) => res,
            Err(e) => handler.failed(&*e),
        },
    };
    let answer = handler.response(answer).await;
    answered(&mut answering, answer).await
}

/// The two halves of the stream a request came on: one to read its body, one
/// to answer on. Apart so that a body still arriving does not hold up the
/// answer -- which is what a `100 Continue` or an early refusal needs.
fn split(stream: Served) -> (Answering, Reading) {
    stream.split()
}

/// The answer, on its way back over HTTP/3: the head, then the body as
/// [`poured`] sends one.
async fn answered(answering: &mut Answering, answer: http::Response<Body>) -> Result<()> {
    let (head, body) = answer.into_parts();
    answering
        .send_response(http::Response::from_parts(head, ()))
        .await?;
    poured(answering, body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ca::LocalCa,
        ca_key::KeyStore,
        capture::{CA_PATH, CHECK_HOST, harness::alone},
        quic_upstream::testing::{SAID, Server, way},
        relay::RelaySettings,
        traffic_rules::TrafficRules,
    };
    use bytes::Buf as _;
    use hudsucker::rustls;

    /// A door, a handler behind it, a way out that trusts `ca`, and where it
    /// listens. The stop channel and the writer threads are given back to be
    /// held: dropped, they take the door down with them.
    fn open(dir: &std::path::Path, ca: &Arc<LocalCa>) -> (SocketAddr, impl Sized) {
        let (handler, threads) = alone(dir);
        let out = Arc::new(way(ca, TrafficRules::default(), RelaySettings::default()));
        let door = Door::open(SocketAddr::from(([127, 0, 0, 1], 0)), ca).unwrap();
        let at = door.at().unwrap();
        let (tell, stop) = tokio::sync::watch::channel(false);
        tokio::spawn(door.serve(handler, out, stop));
        (at, (threads, tell))
    }

    /// tracewing's CA, which signs what the door shows and what the server
    /// behind it shows: one CA is enough for a test, and a client trusting
    /// it is a client trusting both.
    fn ca(dir: &std::path::Path) -> Arc<LocalCa> {
        Arc::new(LocalCa::load_or_create(dir, crate::crypto_provider(), KeyStore::File).unwrap())
    }

    /// A client that trusts `roots` and asks for `host`, as a browser does.
    async fn asking(
        at: SocketAddr,
        host: &str,
        roots: rustls::RootCertStore,
    ) -> Result<(
        h3::client::Connection<h3_quinn::Connection, Bytes>,
        h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>,
    )> {
        let mut tls =
            rustls::ClientConfig::builder_with_provider(Arc::new(crate::crypto_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])?
                .with_root_certificates(roots)
                .with_no_client_auth();
        tls.alpn_protocols = vec![b"h3".to_vec()];
        let mut endpoint = quinn::Endpoint::client(SocketAddr::from(([127, 0, 0, 1], 0)))?;
        endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
        )));
        let conn = endpoint.connect(at, host)?.await?;
        Ok(h3::client::new(h3_quinn::Connection::new(conn)).await?)
    }

    /// Everything a client gets back for one request over HTTP/3, asking
    /// `at` for the certificate of `host` and then for `url`.
    async fn over_http3(
        at: SocketAddr,
        host: &str,
        url: &str,
        roots: rustls::RootCertStore,
    ) -> Result<(http::StatusCode, Vec<u8>)> {
        let (mut driver, mut sender) = asking(at, host, roots).await?;
        let driving = tokio::spawn(async move { driver.wait_idle().await });
        let mut stream = sender
            .send_request(http::Request::get(url).body(()).unwrap())
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

    /// A client's trust in this CA. Its own certificate, not `chain()`:
    /// that one holds what leads to a company's root and is empty for a CA
    /// tracewing signed itself.
    fn trusting(ca: &LocalCa) -> rustls::RootCertStore {
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(rustls::pki_types::CertificateDer::from(
                ca.cert_der().unwrap(),
            ))
            .unwrap();
        roots
    }

    fn dir(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("tracewing-quic-door-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// A page the proxy answers itself, asked for over HTTP/3 by a client
    /// that trusts tracewing's CA and verifies the name it asked for. That
    /// the handshake succeeds at all is the certificate being minted for
    /// **that name** as the client asked: rustls checks the name against the
    /// certificate, and the stand-in would fail there.
    #[tokio::test]
    async fn the_door_serves_what_the_proxy_answers_itself() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("own-pages");
        let ca = ca(&dir);
        let (at, _held) = open(&dir, &ca);
        let (status, body) = over_http3(
            at,
            CHECK_HOST,
            &format!("https://{CHECK_HOST}{CA_PATH}"),
            trusting(&ca),
        )
        .await
        .expect("an answer over HTTP/3");
        assert_eq!(status, http::StatusCode::OK);
        // What the page offers is what the handler was given to offer, which
        // in a test is a stand-in: the CA that signs the certificates of this
        // endpoint and the bytes that page hands out are two different things
        // in `alone()`, and only the first is real.
        assert_eq!(body, b"a certificate authority");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Another name over the same endpoint, which is what says the
    /// certificate is minted per handshake rather than fixed when the door
    /// opened: one endpoint, one config, two names, both verified by the
    /// client against what it asked for.
    #[tokio::test]
    async fn one_endpoint_answers_for_more_than_one_name() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("two-names");
        let ca = ca(&dir);
        let (at, _held) = open(&dir, &ca);
        for host in [CHECK_HOST, "orders.example.com"] {
            let asked = asking(at, host, trusting(&ca)).await;
            assert!(asked.is_ok(), "a certificate for {host}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A client that does not trust the CA gets nowhere, which is what keeps
    /// the tests above from passing on a door that accepts anything.
    #[tokio::test]
    async fn a_client_that_trusts_nothing_gets_no_connection() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("untrusted");
        let ca = ca(&dir);
        let (at, _held) = open(&dir, &ca);
        let asked = asking(at, CHECK_HOST, rustls::RootCertStore::empty()).await;
        assert!(asked.is_err(), "no trust, no connection");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A request meant for a server, all the way: HTTP/3 in at the door,
    /// through the core, HTTP/3 out to a server, and what the server said
    /// back to the client over the stream it asked on.
    #[tokio::test]
    async fn a_request_for_a_server_goes_out_over_http3_and_back() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("to-a-server");
        let ca = ca(&dir);
        let server = Server::start(&ca, "localhost");
        let (at, _held) = open(&dir, &ca);
        let url = format!("https://localhost:{}/orders", server.port);
        let (status, body) = over_http3(at, "localhost", &url, trusting(&ca))
            .await
            .expect("an answer from the server behind the door");
        assert_eq!(status, http::StatusCode::OK);
        assert_eq!(body, SAID);
        assert_eq!(
            server.saw().first().map(|arrived| arrived.path.clone()),
            Some("/orders".to_owned()),
            "and the server is where it arrived",
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A request meant for a server that is not there: the client is told,
    /// rather than left hanging, and the flow is marked failed with the same
    /// words. The way out is given a name nothing resolves, which fails
    /// before any packet is sent.
    #[tokio::test]
    async fn a_request_that_reaches_no_server_is_answered_with_why() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("no-server");
        let ca = ca(&dir);
        let (at, _held) = open(&dir, &ca);
        let (status, body) = over_http3(
            at,
            "orders.example.com",
            "https://nothing.invalid/orders",
            trusting(&ca),
        )
        .await
        .expect("an answer, not a hang");
        assert_eq!(status, http::StatusCode::BAD_GATEWAY);
        let said = String::from_utf8_lossy(&body);
        assert!(said.contains("look up nothing.invalid"), "{said}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
