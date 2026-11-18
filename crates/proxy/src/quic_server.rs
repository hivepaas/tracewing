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
//! `CONNECT` to learn it from -- see [`LocalCa::quic_config`]), and a body
//! carried each way between `h3`'s streams and the body the core takes.
//!
//! **What is not here is the way to a server.** The core answers some
//! requests itself -- the revocation list, the certificate, the check page --
//! and those are served over HTTP/3 as they are over TCP. A request meant for
//! a server is a request this cannot send yet: speaking HTTP/3 upstream is
//! the next change, and until it lands such a request is answered with the
//! reason rather than hung on. That is also why nothing opens this door yet:
//! no way in points at it, and `datagrams.rs` still refuses UDP 443.

use crate::{
    ca::LocalCa,
    capture::FlowHandler,
    source::{Stop, stopped},
};
use anyhow::{Context as _, Result};
use bytes::{Buf as _, Bytes};
use http_body::Frame;
use http_body_util::{BodyExt as _, combinators::BoxBody};
use hudsucker::{Body, RequestOrResponse};
use std::{
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::mpsc;

/// How much of a client's body waits here while the core reads it. Small on
/// purpose: a body is read as it arrives, and anything larger is the spool's
/// business rather than this channel's.
const WAITING: usize = 8;

/// What a request meant for a server is answered with until this door can
/// speak HTTP/3 to one.
const NO_UPSTREAM: &str = "tracewing cannot speak HTTP/3 to a server yet: this \
                           request arrived over HTTP/3 and was not sent on";

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

    /// Serves until `stop`, each connection on its own task.
    pub async fn serve(self, handler: FlowHandler, stop: Stop) {
        loop {
            let waiting = tokio::select! {
                waiting = self.endpoint.accept() => waiting,
                () = stopped(stop.clone()) => break,
            };
            let Some(incoming) = waiting else { break };
            let handler = handler.clone();
            tokio::spawn(async move {
                let client = incoming.remote_address();
                if let Err(e) = connected(incoming, handler).await {
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
async fn connected(incoming: quinn::Incoming, handler: FlowHandler) -> Result<()> {
    let conn = incoming.await?;
    let client = conn.remote_address();
    let mut h3 = h3::server::Connection::new(h3_quinn::Connection::new(conn)).await?;
    while let Some(resolver) = h3.accept().await? {
        // One handler per request, as hudsucker clones one per request: the
        // fields it fills in on the way through belong to that request.
        let mut handler = handler.clone();
        tokio::spawn(async move {
            if let Err(e) = served(resolver, &mut handler, client).await {
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
    client: SocketAddr,
) -> Result<()> {
    let (head, stream) = resolver.resolve_request().await?;
    let (mut answering, reading) = split(stream);
    let req = head.map(|()| sent(reading));
    let answer = match handler.request(client, req).await {
        RequestOrResponse::Response(res) => res,
        // Not a mistake of the client's, and not the server's either: it is
        // this door that cannot pass it on yet. The flow says so, and so does
        // the client's answer.
        RequestOrResponse::Request(_) => handler.unreachable(NO_UPSTREAM.to_owned()),
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

/// The client's body, as the core takes bodies: read off the stream by a task
/// of its own and handed over a frame at a time.
fn sent(mut reading: Reading) -> Body {
    let (tx, rx) = mpsc::channel(WAITING);
    tokio::spawn(async move {
        loop {
            match reading.recv_data().await {
                Ok(Some(mut data)) => {
                    let frame = Frame::data(data.copy_to_bytes(data.remaining()));
                    // The core let go of the body: nothing more is wanted.
                    if tx.send(Ok(frame)).await.is_err() {
                        return;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = tx.send(Err(as_error(&e))).await;
                    return;
                }
            }
        }
        // Trailers come after the body, and gRPC's status is in them, so they
        // are carried rather than dropped.
        if let Ok(Some(trailers)) = reading.recv_trailers().await {
            let _ = tx.send(Ok(Frame::trailers(trailers))).await;
        }
    });
    Body::from(BoxBody::new(Sent(rx)))
}

/// The answer, on its way back over HTTP/3.
async fn answered(answering: &mut Answering, answer: http::Response<Body>) -> Result<()> {
    let (head, mut body) = answer.into_parts();
    answering
        .send_response(http::Response::from_parts(head, ()))
        .await?;
    while let Some(frame) = body.frame().await {
        match frame?.into_data() {
            Ok(data) => answering.send_data(data).await?,
            // Anything that is not data is trailers, which end the body.
            Err(frame) => {
                if let Ok(trailers) = frame.into_trailers() {
                    answering.send_trailers(trailers).await?;
                }
                break;
            }
        }
    }
    answering.finish().await?;
    Ok(())
}

/// What the core's bodies carry as a failure. An `h3` error is none of its
/// variants, so it goes in as the io error it reads as.
fn as_error(e: &h3::error::StreamError) -> hudsucker::Error {
    hudsucker::Error::Io(std::io::Error::other(e.to_string()))
}

/// The receiving end of a client's body: see [`sent`].
struct Sent(mpsc::Receiver<Result<Frame<Bytes>, hudsucker::Error>>);

impl http_body::Body for Sent {
    type Data = Bytes;
    type Error = hudsucker::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
        self.0.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ca::LocalCa,
        ca_key::KeyStore,
        capture::{CA_PATH, CHECK_HOST, harness::alone},
    };
    use hudsucker::rustls;

    /// A door, a handler behind it, and where it listens. The stop channel
    /// and the writer threads are given back to be held: dropped, they take
    /// the door down with them.
    fn open(dir: &std::path::Path) -> (SocketAddr, Arc<LocalCa>, impl Sized) {
        let ca = Arc::new(
            LocalCa::load_or_create(dir, crate::crypto_provider(), KeyStore::File).unwrap(),
        );
        let (handler, threads) = alone(dir);
        let door = Door::open(SocketAddr::from(([127, 0, 0, 1], 0)), &ca).unwrap();
        let at = door.at().unwrap();
        let (tell, stop) = tokio::sync::watch::channel(false);
        tokio::spawn(door.serve(handler, stop));
        (at, ca, (threads, tell))
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

    /// Everything a client gets back for one request over HTTP/3.
    async fn over_http3(
        at: SocketAddr,
        host: &str,
        path: &str,
        roots: rustls::RootCertStore,
    ) -> Result<(http::StatusCode, Vec<u8>)> {
        let (mut driver, mut sender) = asking(at, host, roots).await?;
        let driving = tokio::spawn(async move { driver.wait_idle().await });
        let mut stream = sender
            .send_request(
                http::Request::get(format!("https://{host}{path}"))
                    .body(())
                    .unwrap(),
            )
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
        let (at, ca, _held) = open(&dir);
        let (status, body) = over_http3(at, CHECK_HOST, CA_PATH, trusting(&ca))
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
        let (at, ca, _held) = open(&dir);
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
        let (at, _ca, _held) = open(&dir);
        let asked = asking(at, CHECK_HOST, rustls::RootCertStore::empty()).await;
        assert!(asked.is_err(), "no trust, no connection");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A request meant for a server: this door cannot send it on yet, and
    /// says so rather than hanging on it. The flow is marked failed with the
    /// same words, which is what `unreachable` is for.
    #[tokio::test]
    async fn a_request_for_a_server_is_told_there_is_no_way_on_yet() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("no-upstream");
        let (at, ca, _held) = open(&dir);
        let (status, body) = over_http3(at, "orders.example.com", "/orders", trusting(&ca))
            .await
            .expect("an answer, not a hang");
        assert_eq!(status, http::StatusCode::BAD_GATEWAY);
        let said = String::from_utf8_lossy(&body);
        assert!(
            said.contains("cannot speak HTTP/3 to a server yet"),
            "{said}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
