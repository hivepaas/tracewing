//! The way out over QUIC: HTTP/3 to a server, for what arrived over HTTP/3.
//!
//! [`crate::upstream`] is the other way out, and the older one: a name looked
//! up, TCP, then TLS, with each step timed and what the server showed kept.
//! This one does the same job over a QUIC connection, and keeps the same
//! facts in the same [`Upstream`], so a flow that went this way reads in the
//! inspector as any other flow does.
//!
//! **Why it is a way out of its own rather than a branch of that one.** There
//! is no `TcpStream` in it to hand to hyper, and nothing above it to hand one
//! to: hyper's client pools connections and sends requests over them, and
//! that is what `h3` does here instead. What the two ways out do share --
//! which certificates to trust, which to show, what a rule says about a host,
//! and the numbers connections are known by -- is passed in rather than
//! written twice.
//!
//! What is deliberately not here, because it cannot be done rather than
//! because it is unwritten, is in [`Way::send`]: a company proxy cannot carry
//! QUIC, and a rule that says to ask a host with HTTP/1.1 or HTTP/2 cannot be
//! obeyed by a way out that speaks HTTP/3. Both are refused in words rather
//! than silently done differently. See `docs/design/http3.md`.

use crate::{
    client_certs::ClientCerts,
    quic_body::{carried, poured},
    relay::Relay,
    traffic_rules::TrafficRules,
    upstream::{
        BoxError, Ids, TlsFacts, TlsFailure, Unanswered, Upstream, Witness, datagrams_out, micros,
        unix_micros,
    },
};
use bytes::Bytes;
use http::{HeaderName, HeaderValue, Request, Response, header};
use hudsucker::{
    Body,
    rustls::{
        self, ClientConfig,
        client::{ResolvesClientCert, danger::ServerCertVerifier},
        sign::SingleCertAndKey,
    },
};
use quinn::crypto::rustls::{HandshakeData, QuicClientConfig};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex, RwLock, atomic::AtomicBool},
    time::Instant,
};

/// The one application protocol this way out offers, and so the only one it
/// can agree: a server that will not speak HTTP/3 fails here rather than
/// quietly being asked with something else.
const ALPN: &[u8] = b"h3";

/// What `h3` hands a request to, cloned per request.
type Sender = h3::client::SendRequest<h3_quinn::OpenStreams, Bytes>;

/// Headers HTTP/3 does not carry (RFC 9114 section 4.2). None of them arrive
/// over HTTP/3 in the first place, so this is about what happens in between:
/// a rule or a script can add one, and `h3` would refuse the whole request
/// over a header a user asked for. They go out instead.
const NOT_IN_HTTP3: [HeaderName; 3] = [
    header::CONNECTION,
    header::TRANSFER_ENCODING,
    header::UPGRADE,
];

/// The way out, held by whoever opens a QUIC front door.
pub struct Way {
    /// TLS 1.3 -- QUIC has no other (RFC 9001) -- offering `h3` alone, with
    /// the roots to check a server against carried by the verifier. Each
    /// connection clones it to add a witness and its client certificate.
    tls: Arc<ClientConfig>,
    verifier: Arc<dyn ServerCertVerifier>,
    /// Looked at per connection, to refuse rather than go around it.
    relay: Arc<Relay>,
    client_certs: Arc<ClientCerts>,
    traffic_rules: Arc<RwLock<TrafficRules>>,
    ids: Ids,
    /// One connection per server, for as long as it is good: HTTP/3
    /// multiplexes, so a page's requests belong on one connection rather
    /// than on one each.
    held: Mutex<HashMap<String, Held>>,
}

/// A connection to one server, kept for the next request to it.
#[derive(Clone)]
struct Held {
    /// Whether it is still open, which `h3` does not say and `quinn` does.
    quic: quinn::Connection,
    sender: Sender,
    upstream: Arc<Upstream>,
    /// Held so that the endpoint is not dropped while the connection it
    /// carries is still in use.
    _endpoint: quinn::Endpoint,
}

impl Way {
    /// `verifier` carries which roots a server is checked against, as it
    /// does for [`crate::upstream::TimedConnector`]; `ids` are that
    /// connector's, so the two ways out do not number connections over each
    /// other. See [`Ids`].
    pub fn new(
        verifier: Arc<dyn ServerCertVerifier>,
        key_log: Arc<dyn rustls::KeyLog>,
        relay: Arc<Relay>,
        client_certs: Arc<ClientCerts>,
        traffic_rules: Arc<RwLock<TrafficRules>>,
        ids: Ids,
    ) -> Result<Self, BoxError> {
        let mut tls = ClientConfig::builder_with_provider(Arc::new(crate::crypto_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .dangerous()
            .with_custom_certificate_verifier(Arc::clone(&verifier))
            .with_no_client_auth();
        tls.alpn_protocols = vec![ALPN.to_vec()];
        // **No early data on the way out either, said rather than
        // inherited.** This is rustls's default, so the line changes
        // nothing today -- and it is worth writing because quinn's own
        // convenience constructors set the opposite
        // (`QuicClientConfig::inner`, behind
        // `ClientConfig::with_root_certificates`, sets it true), so a later
        // move to the shorter call would turn it on with nothing failing.
        // What it would turn on: a request of the client's sent as early
        // data, which a server may answer twice. The door refuses it in the
        // other direction for the same reason -- see `LocalCa::quic_config`.
        tls.enable_early_data = false;
        // The same key log as everything else tracewing agrees, so what went
        // over HTTP/3 can be read in Wireshark like the rest.
        tls.key_log = key_log;
        Ok(Self {
            tls: Arc::new(tls),
            verifier,
            relay,
            client_certs,
            traffic_rules,
            ids,
            held: Mutex::new(HashMap::new()),
        })
    }

    /// `req` to the server it is addressed to, over HTTP/3, and what the
    /// server said -- with the connection it went over in the response's
    /// extensions, which is where the core reads it from.
    ///
    /// The two refusals are here rather than further in because they are
    /// about the request, not about any connection:
    ///
    /// - **A company proxy.** An HTTP proxy carries TCP; QUIC over one needs
    ///   `CONNECT-UDP` (RFC 9298), which tracewing does not speak and most
    ///   company proxies do not either.
    /// - **A rule naming a version.** `Ask server with HTTP/2` cannot be
    ///   obeyed over QUIC, and quietly ignoring the rule would be worse than
    ///   saying so: the rule exists to make a server answer a certain way.
    pub async fn send(&self, req: Request<Body>) -> Result<Response<Body>, BoxError> {
        let (mut head, body) = req.into_parts();
        let host = head
            .uri
            .host()
            .ok_or("an HTTP/3 request with no host to send it to")?
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        if let Some(scheme) = head.uri.scheme_str()
            && scheme != "https"
        {
            return Err(format!("{scheme}:// is not something HTTP/3 carries").into());
        }
        let port = head.uri.port_u16().unwrap_or(443);
        if self.relay.route_for(&host).is_some() {
            return Err(format!(
                "a proxy is set for {host} and a proxy cannot carry QUIC, so this HTTP/3 \
                 request was not sent on"
            )
            .into());
        }
        if let Some(asked) = self
            .traffic_rules
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .ask_host_with(&host, port)
        {
            return Err(format!(
                "a rule says to ask {host} with {}, and this request arrived over HTTP/3, \
                 which tracewing does not turn into another version",
                asked.text()
            )
            .into());
        }
        for name in NOT_IN_HTTP3 {
            head.headers.remove(name);
        }
        head.headers.remove(HeaderName::from_static("keep-alive"));
        head.headers
            .remove(HeaderName::from_static("proxy-connection"));
        // `te` may go, and only with that one value (RFC 9114 section 4.2).
        if head
            .headers
            .get(header::TE)
            .is_some_and(|value| value != HeaderValue::from_static("trailers"))
        {
            head.headers.remove(header::TE);
        }

        let key = format!("{host}:{port}");
        let asking = Request::from_parts(head, ());
        // A connection kept from an earlier request can have been closed by
        // the server in between, and nothing says so until it is used. The
        // request's headers go first and its body has not been touched yet,
        // so one that fails there can be sent again on a new connection --
        // which is why this is here rather than inside `connection`.
        let (held, fresh) = self.connection(&key, &host, port).await?;
        let sent = match held.sender.clone().send_request(asking.clone()).await {
            Ok(stream) => stream,
            Err(_) if !fresh => {
                self.forget(&key);
                let (held, _) = self.connection(&key, &host, port).await?;
                held.sender
                    .clone()
                    .send_request(asking)
                    .await
                    .map_err(|e| format!("send a request to {key} over HTTP/3: {e}"))?
            }
            Err(e) => return Err(format!("send a request to {key} over HTTP/3: {e}").into()),
        };

        // Apart, so that a body still going out does not hold up an answer:
        // a 401 or a 413 to a long upload comes before the upload ends.
        let (mut sending, mut reading) = sent.split();
        tokio::spawn(async move {
            if let Err(e) = poured(&mut sending, body).await {
                tracing::debug!("the body of an HTTP/3 request to {key}: {e:#}");
            }
        });
        let answer = reading
            .recv_response()
            .await
            .map_err(|e| format!("the answer from {host} over HTTP/3: {e}"))?;
        let (mut parts, ()) = answer.into_parts();
        // Where the core reads which connection a response came over, as it
        // does for one hyper made.
        parts.extensions.insert(Arc::clone(&held.upstream));
        Ok(Response::from_parts(parts, carried(reading)))
    }

    /// The connection to `key`, kept or made, and whether it was just made.
    async fn connection(&self, key: &str, host: &str, port: u16) -> Result<(Held, bool), BoxError> {
        if let Some(held) = self.pooled(key) {
            return Ok((held, false));
        }
        let held = self.connect(host, port).await?;
        // Two requests to a server nothing is open to yet make two
        // connections, and one of them is kept: the other serves the request
        // it was made for and goes when that ends. Not worth a lock held
        // across a handshake.
        self.held
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.to_owned(), held.clone());
        Ok((held, true))
    }

    /// The connection kept for `key`, if one is and it is still open.
    fn pooled(&self, key: &str) -> Option<Held> {
        let mut held = self.held.lock().unwrap_or_else(|e| e.into_inner());
        let kept = held.get(key)?;
        match kept.quic.close_reason() {
            None => Some(kept.clone()),
            Some(_) => {
                held.remove(key);
                None
            }
        }
    }

    fn forget(&self, key: &str) {
        self.held
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key);
    }

    /// A QUIC connection to `host`, with every step timed the way the TCP
    /// way out times its own.
    async fn connect(&self, host: &str, port: u16) -> Result<Held, BoxError> {
        let id = self.ids.next();
        let started_us = unix_micros();
        let began = Instant::now();
        let (addresses, dns_us) = looked_up(host, port, began).await?;

        // A config of this connection's own: the rest is shared, and these
        // two are not. The witness keeps what this server showed so that
        // certificates refused can still be looked at; the resolver answers
        // this server's ask for a client certificate.
        let witness = Arc::new(Witness::watching(Arc::clone(&self.verifier)));
        let unanswered = Arc::new(Unanswered::default());
        let mut tls = (*self.tls).clone();
        tls.dangerous()
            .set_certificate_verifier(Arc::clone(&witness) as Arc<dyn ServerCertVerifier>);
        tls.client_auth_cert_resolver = match self.client_certs.for_host(host) {
            Some(cert) => Arc::new(SingleCertAndKey::from(cert)),
            None => Arc::clone(&unanswered) as Arc<dyn ResolvesClientCert>,
        };
        let quic = QuicClientConfig::try_from(Arc::new(tls))
            .map_err(|e| format!("the TLS for QUIC: {e}"))?;
        let config = quinn::ClientConfig::new(Arc::new(quic));

        let shaking = Instant::now();
        let mut trouble: Option<BoxError> = None;
        // In the order the resolver gave them, one after another. The TCP
        // way out gets happy eyeballs from hyper's connector, which races
        // the families; this has none to borrow, and inventing one here is
        // not what this change is about.
        for addr in addresses {
            let endpoint = match sending_from(addr) {
                Ok(endpoint) => endpoint,
                Err(e) => {
                    trouble = Some(format!("a socket to reach {addr} from: {e}").into());
                    continue;
                }
            };
            let connecting = match endpoint.connect_with(config.clone(), addr, host) {
                Ok(connecting) => connecting,
                Err(e) => {
                    trouble = Some(format!("connect to {addr} over QUIC: {e}").into());
                    continue;
                }
            };
            match connecting.await {
                Ok(conn) => {
                    let remote = conn.remote_address();
                    let (mut driver, sender) =
                        h3::client::new(h3_quinn::Connection::new(conn.clone()))
                            .await
                            .map_err(|e| format!("HTTP/3 over QUIC to {remote}: {e}"))?;
                    // The driver is what moves the connection along; it ends
                    // when the last sender for it is dropped.
                    tokio::spawn(async move {
                        let _ = driver.wait_idle().await;
                    });
                    return Ok(Held {
                        upstream: Arc::new(Upstream {
                            id,
                            started_us,
                            ready_us: started_us + micros(began),
                            dns_us,
                            // **The whole handshake.** QUIC agrees the
                            // connection and its TLS in one, so there is no
                            // TLS step of its own to time -- what the TLS
                            // was is in the facts below. See
                            // `docs/design/http3.md`.
                            connect_us: micros(shaking),
                            tls_us: None,
                            remote,
                            tls: Some(facts_of(&conn)),
                            client_cert_missing: unanswered.was_asked(),
                            told: AtomicBool::new(false),
                            ahead: AtomicBool::new(false),
                        }),
                        quic: conn,
                        sender,
                        _endpoint: endpoint,
                    });
                }
                Err(e) => trouble = Some(refusal(id, host, addr, &e, &witness)),
            }
        }
        Err(trouble
            .unwrap_or_else(|| format!("{host}:{port} has no address to reach over QUIC").into()))
    }
}

/// What a handshake that failed is reported as: a [`TlsFailure`] where the
/// server showed certificates, so the inspector can show what was refused,
/// and plain words where it showed none and TLS never got that far.
fn refusal(
    id: i64,
    host: &str,
    addr: SocketAddr,
    e: &quinn::ConnectionError,
    witness: &Witness,
) -> BoxError {
    let certificates = witness.take();
    if certificates.is_empty() {
        return format!("connect to {addr} over QUIC: {e}").into();
    }
    Box::new(TlsFailure {
        id,
        host: host.to_owned(),
        remote: addr,
        message: e.to_string(),
        certificates,
        not_tls: false,
    })
}

/// What the server showed of itself, as the TCP way out keeps it.
///
/// Two differences, both `quinn`'s: the version is TLS 1.3 because QUIC has
/// no other rather than because it was read back, and the cipher suite is
/// not said at all -- `quinn` hands over the protocol and the name, and
/// nothing about the suite.
fn facts_of(conn: &quinn::Connection) -> TlsFacts {
    let alpn = conn
        .handshake_data()
        .and_then(|data| data.downcast::<HandshakeData>().ok())
        .and_then(|data| data.protocol.clone())
        .map(|protocol| String::from_utf8_lossy(&protocol).into_owned());
    let certificates = conn
        .peer_identity()
        .and_then(|identity| {
            identity
                .downcast::<Vec<rustls::pki_types::CertificateDer>>()
                .ok()
        })
        .map(|certificates| certificates.iter().map(|der| der.to_vec()).collect())
        .unwrap_or_default();
    TlsFacts {
        version: "TLS 1.3".to_owned(),
        cipher: String::new(),
        alpn,
        certificates,
    }
}

/// An endpoint to send from: a socket of its own per connection, because the
/// interface it must leave by can change between one connection and the
/// next. See [`datagrams_out`].
fn sending_from(to: SocketAddr) -> Result<quinn::Endpoint, BoxError> {
    let socket = datagrams_out(to)?;
    Ok(quinn::Endpoint::new(
        quinn::EndpointConfig::default(),
        None,
        socket,
        Arc::new(quinn::TokioRuntime),
    )?)
}

/// The name looked up, timed, in the same words as the TCP way out uses for
/// the same failures -- a user reading a flow should not be able to tell
/// which way out looked a name up.
async fn looked_up(
    host: &str,
    port: u16,
    began: Instant,
) -> Result<(Vec<SocketAddr>, Option<i64>), BoxError> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok((vec![SocketAddr::new(ip, port)], None));
    }
    let found: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| format!("look up {host}: {e}"))?
        .collect();
    if found.is_empty() {
        return Err(format!("look up {host}: no addresses").into());
    }
    Ok((found, Some(micros(began))))
}

/// A server to send to, and a way out that trusts it: for the tests of this
/// module and of the door in front of it. Nothing in here is built outside
/// tests.
///
/// The server holds a leaf from tracewing's own CA, as a real server holds
/// one from a real CA, so the handshake under test is the real one rather
/// than one with the checking turned off.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::{
        ca::{LocalCa, Over},
        relay::{Relay, RelaySettings},
    };
    use bytes::Buf as _;
    use http::{HeaderMap, HeaderValue};
    use hudsucker::rustls::{
        RootCertStore, client::WebPkiServerVerifier, pki_types::CertificateDer,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A way out that trusts `ca` and nothing else, where the real one
    /// trusts the machine's roots: the verifier is what carries that, here
    /// as there.
    pub(crate) fn way(ca: &LocalCa, rules: TrafficRules, relay: RelaySettings) -> Way {
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(ca.cert_der().unwrap()))
            .unwrap();
        let verifier = WebPkiServerVerifier::builder_with_provider(
            Arc::new(roots),
            Arc::new(crate::crypto_provider()),
        )
        .build()
        .unwrap();
        Way::new(
            verifier,
            Arc::new(rustls::NoKeyLog),
            Arc::new(Relay::new(&relay).unwrap()),
            Default::default(),
            Arc::new(RwLock::new(rules)),
            Ids::default(),
        )
        .unwrap()
    }

    /// What it answers where the request brought no body of its own.
    pub(crate) const SAID: &[u8] = b"over HTTP/3 from a server of our own";
    /// On the end of every answer, to say trailers came through.
    pub(crate) const TRAILER: &str = "x-done";
    /// Asked for this, the server goes away without answering -- what one
    /// does when its idle timer ran out or it was restarted between one
    /// request and the next.
    pub(crate) const GOES: &str = "/gone";

    /// One request, as the server saw it.
    pub(crate) struct Arrived {
        pub(crate) method: String,
        pub(crate) path: String,
        pub(crate) headers: HeaderMap,
        pub(crate) body: Vec<u8>,
    }

    pub(crate) struct Server {
        /// The port both families answer on, so that a name looked up to
        /// either of them is answered. A test machine resolves `localhost`
        /// to one or the other and which is not ours to choose.
        pub(crate) port: u16,
        pub(crate) seen: Arc<Mutex<Vec<Arrived>>>,
        /// How many connections were accepted, which is what says a second
        /// request went over the first's connection.
        pub(crate) connections: Arc<AtomicUsize>,
        /// Dropped with the server, which closes what it is listening on.
        _endpoints: Vec<quinn::Endpoint>,
    }

    impl Server {
        /// Listens on loopback, with a certificate for `host` minted by `ca`.
        pub(crate) fn start(ca: &LocalCa, host: &str) -> Self {
            let quic =
                quinn::crypto::rustls::QuicServerConfig::try_from(ca.config_for(host, Over::Quic))
                    .expect("the certificate for QUIC");
            let config = quinn::ServerConfig::with_crypto(Arc::new(quic));
            let v4 = quinn::Endpoint::server(config.clone(), SocketAddr::from(([127, 0, 0, 1], 0)))
                .expect("listen on 127.0.0.1");
            let port = v4.local_addr().expect("an address").port();
            // The same port for IPv6, which is free in all but a race. A
            // machine without IPv6 has none to bind and none to resolve to.
            let v6 =
                quinn::Endpoint::server(config, SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port)))
                    .ok();

            let seen = Arc::new(Mutex::new(Vec::new()));
            let connections = Arc::new(AtomicUsize::new(0));
            let mut endpoints = vec![v4];
            endpoints.extend(v6);
            for endpoint in &endpoints {
                tokio::spawn(accepting(
                    endpoint.clone(),
                    Arc::clone(&seen),
                    Arc::clone(&connections),
                ));
            }
            Self {
                port,
                seen,
                connections,
                _endpoints: endpoints,
            }
        }

        /// What arrived, as this test wants to read it.
        pub(crate) fn saw(&self) -> std::sync::MutexGuard<'_, Vec<Arrived>> {
            self.seen.lock().unwrap_or_else(|e| e.into_inner())
        }
    }

    async fn accepting(
        endpoint: quinn::Endpoint,
        seen: Arc<Mutex<Vec<Arrived>>>,
        connections: Arc<AtomicUsize>,
    ) {
        while let Some(incoming) = endpoint.accept().await {
            let (seen, connections) = (Arc::clone(&seen), Arc::clone(&connections));
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                connections.fetch_add(1, Ordering::Relaxed);
                // Kept so that a request for `GOES` can close it.
                let going = conn.clone();
                let Ok(mut h3) = h3::server::Connection::new(h3_quinn::Connection::new(conn)).await
                else {
                    return;
                };
                while let Ok(Some(resolver)) = h3.accept().await {
                    let (seen, conn) = (Arc::clone(&seen), going.clone());
                    tokio::spawn(answered(resolver, seen, conn));
                }
            });
        }
    }

    /// One request: written down, then answered with what it brought -- so a
    /// test can see its own body having gone out and come back.
    async fn answered(
        resolver: h3::server::RequestResolver<h3_quinn::Connection, Bytes>,
        seen: Arc<Mutex<Vec<Arrived>>>,
        conn: quinn::Connection,
    ) {
        let Ok((head, stream)) = resolver.resolve_request().await else {
            return;
        };
        // Before answering rather than after, so that nothing is half-sent:
        // the point is a connection that is gone, not a truncated answer.
        if head.uri().path() == GOES {
            return conn.close(0u32.into(), b"the server went away");
        }
        let (mut answering, mut reading) = stream.split();
        let mut body = Vec::new();
        while let Ok(Some(mut chunk)) = reading.recv_data().await {
            body.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
        }
        seen.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Arrived {
                method: head.method().to_string(),
                path: head.uri().path().to_owned(),
                headers: head.headers().clone(),
                body: body.clone(),
            });
        let answer = http::Response::builder()
            .status(200)
            .header("content-type", "text/plain")
            .body(())
            .expect("an answer of our own");
        if answering.send_response(answer).await.is_err() {
            return;
        }
        let said = match body.is_empty() {
            true => Bytes::from_static(SAID),
            false => Bytes::from(body),
        };
        let _ = answering.send_data(said).await;
        let mut trailers = HeaderMap::new();
        trailers.insert(TRAILER, HeaderValue::from_static("yes"));
        let _ = answering.send_trailers(trailers).await;
        let _ = answering.finish().await;
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{SAID, Server, TRAILER, way};
    use super::*;
    use crate::{
        ca::LocalCa,
        ca_key::KeyStore,
        relay::{Relay, RelaySettings},
        traffic_rules::{Action, AskWith, TrafficRule},
    };
    use http_body_util::BodyExt as _;

    fn dir(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("tracewing-quic-out-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn ca(dir: &std::path::Path) -> Arc<LocalCa> {
        Arc::new(LocalCa::load_or_create(dir, crate::crypto_provider(), KeyStore::File).unwrap())
    }

    /// A request out over HTTP/3 and the answer back: the body each way, the
    /// trailers after it, and the connection it went over on the response
    /// for the core to read.
    #[tokio::test]
    async fn a_request_goes_out_over_http3_and_the_answer_comes_back() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("there-and-back");
        let ca = ca(&dir);
        let server = Server::start(&ca, "localhost");
        let out = way(&ca, TrafficRules::default(), RelaySettings::default());

        let sent = "what the client sent";
        let answer = out
            .send(
                Request::post(format!("https://localhost:{}/orders", server.port))
                    // Not something HTTP/3 carries: it goes out rather than
                    // the whole request being refused over it.
                    .header("connection", "keep-alive")
                    .header("x-mine", "kept")
                    .body(Body::from(sent))
                    .unwrap(),
            )
            .await
            .expect("an answer over HTTP/3");

        assert_eq!(answer.status(), http::StatusCode::OK);
        assert_eq!(answer.version(), http::Version::HTTP_3);
        let upstream = Arc::clone(
            answer
                .extensions()
                .get::<Arc<Upstream>>()
                .expect("the connection it went over"),
        );
        let body = answer.into_body().collect().await.unwrap();
        let trailers = body.trailers().cloned();
        assert_eq!(
            body.to_bytes(),
            sent.as_bytes(),
            "the server echoes what arrived, so this is the body having gone out"
        );
        assert!(
            trailers.is_some_and(|t| t.contains_key(TRAILER)),
            "the trailers after the body"
        );

        let saw = server.saw();
        let arrived = saw.first().expect("the server saw the request");
        assert_eq!(
            (arrived.method.as_str(), arrived.path.as_str()),
            ("POST", "/orders")
        );
        assert_eq!(arrived.body, sent.as_bytes());
        assert_eq!(
            arrived.headers.get("x-mine").map(|v| v.as_bytes()),
            Some(b"kept".as_slice()),
        );
        assert!(
            !arrived.headers.contains_key("connection"),
            "connection is not a header HTTP/3 carries (RFC 9114 section 4.2)",
        );

        assert_eq!(upstream.remote.port(), server.port);
        assert_eq!(
            upstream.tls.as_ref().and_then(|tls| tls.alpn.as_deref()),
            Some("h3")
        );
        assert!(
            upstream
                .tls
                .as_ref()
                .is_some_and(|tls| !tls.certificates.is_empty()),
            "what the server showed, kept as the TCP way out keeps it",
        );
        // One handshake, so the whole of it is the connecting and there is no
        // TLS step of its own.
        assert!(upstream.connect_us > 0);
        assert_eq!(upstream.tls_us, None);
        drop(saw);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The second request goes over the connection the first made: HTTP/3
    /// multiplexes, and a page's requests belong on one connection.
    #[tokio::test]
    async fn a_second_request_goes_over_the_first_s_connection() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("one-connection");
        let ca = ca(&dir);
        let server = Server::start(&ca, "localhost");
        let out = way(&ca, TrafficRules::default(), RelaySettings::default());

        let mut ids = Vec::new();
        for _ in 0..2 {
            let answer = out
                .send(
                    Request::get(format!("https://localhost:{}/", server.port))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("an answer over HTTP/3");
            ids.push(
                answer
                    .extensions()
                    .get::<Arc<Upstream>>()
                    .expect("the connection")
                    .id,
            );
            let body = answer.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(body, SAID);
        }
        assert_eq!(ids[0], ids[1], "one connection, so one number");
        assert_eq!(
            server
                .connections
                .load(std::sync::atomic::Ordering::Relaxed),
            1,
            "and the server accepted it once",
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A connection the server has closed is not handed to the next
    /// request. Two things see to that and which of them does depends on
    /// timing -- the pool drops a connection `quinn` has been told is closed,
    /// and a request whose headers will not go out on a kept connection is
    /// sent again on a new one -- so what this asks is the part that is not
    /// about timing: the request after a server went away is answered.
    ///
    /// Measured, by taking each away on its own: either one alone keeps this
    /// green, and taking both away turns it red. That is why both are here.
    #[tokio::test]
    async fn a_connection_the_server_closed_is_not_handed_on() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("the-server-went");
        let ca = ca(&dir);
        let server = Server::start(&ca, "localhost");
        let out = way(&ca, TrafficRules::default(), RelaySettings::default());
        let to = |path: &str| {
            Request::get(format!("https://localhost:{}{path}", server.port))
                .body(Body::empty())
                .unwrap()
        };

        let first = out.send(to("/")).await.expect("an answer");
        assert_eq!(first.status(), http::StatusCode::OK);
        // The server goes away over this one, so it is not answered.
        let _ = out.send(to(super::testing::GOES)).await;
        let again = out
            .send(to("/"))
            .await
            .expect("an answer after the server went away");
        assert_eq!(again.status(), http::StatusCode::OK);
        assert_eq!(
            again.into_body().collect().await.unwrap().to_bytes(),
            SAID,
            "the whole answer, over a connection made for it",
        );
        assert_eq!(
            server
                .connections
                .load(std::sync::atomic::Ordering::Relaxed),
            2,
            "the one that went, and the one made after it",
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A rule that says which version to ask a host with cannot be obeyed
    /// over QUIC, and saying so is the point: a rule quietly ignored is
    /// worse than a request refused, because the rule is there to make the
    /// server answer a certain way.
    #[tokio::test]
    async fn a_rule_naming_another_version_is_refused_in_words() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("a-rule-says-h2");
        let ca = ca(&dir);
        let rules = TrafficRules {
            rules: vec![TrafficRule {
                name: "orders in HTTP/2".to_owned(),
                enabled: true,
                url: "orders.example.com".to_owned(),
                method: String::new(),
                action: Action::AskServerWith {
                    version: AskWith::Http2,
                },
            }],
        };
        let out = way(&ca, rules, RelaySettings::default());
        let refused = out
            .send(
                Request::get("https://orders.example.com/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect_err("a rule this way out cannot obey");
        let said = refused.to_string();
        assert!(said.contains("HTTP/2"), "{said}");
        assert!(said.contains("orders.example.com"), "{said}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A company proxy carries TCP, and a QUIC connection through one needs
    /// `CONNECT-UDP`, which neither tracewing nor most such proxies speak.
    /// Refused here rather than going around the proxy, which is what a
    /// machine behind one would notice.
    #[tokio::test]
    async fn a_proxy_in_the_way_is_refused_rather_than_gone_around() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("a-proxy-is-set");
        let ca = ca(&dir);
        let out = way(
            &ca,
            TrafficRules::default(),
            RelaySettings {
                proxy: "http://proxy.invalid:8080".to_owned(),
                bypass: String::new(),
            },
        );
        let refused = out
            .send(
                Request::get("https://orders.example.com/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect_err("a proxy cannot carry QUIC");
        assert!(refused.to_string().contains("proxy"), "{refused}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Nothing to send to, said rather than guessed at.
    #[tokio::test]
    async fn a_request_with_no_host_goes_nowhere() {
        let _ = crate::crypto_provider().install_default();
        let dir = dir("no-host");
        let ca = ca(&dir);
        let out = way(&ca, TrafficRules::default(), RelaySettings::default());
        let refused = out
            .send(Request::get("/orders").body(Body::empty()).unwrap())
            .await
            .expect_err("no host, nowhere to send it");
        assert!(refused.to_string().contains("no host"), "{refused}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **The numbers connections are known by are one counter.** A flow row
    /// points at its connection by this number, so a way out that started
    /// its own count at 1 would put two connections under one number in the
    /// inspector. See [`Ids`].
    #[test]
    fn the_two_ways_out_number_connections_over_one_counter() {
        let (tls, verifier) = crate::upstream_tls(&[], false).unwrap();
        let relay = Arc::new(Relay::new(&Default::default()).unwrap());
        let connector = crate::upstream::TimedConnector::new(
            tls,
            verifier,
            relay,
            Default::default(),
            Default::default(),
        );
        let ids = connector.ids();
        assert_eq!(ids.next(), 1);
        assert_eq!(connector.ids().next(), 2, "the same counter, not a copy");
        assert_eq!(ids.next(), 3);
    }

    /// **The way out asks for no early data.** rustls defaults it off, so
    /// this asserts a default -- on purpose: quinn's convenience
    /// constructor sets it on, and the day somebody moves this to that call
    /// the decision has to come with it. The door's half of the same
    /// decision is measured from the outside in `quic_server`.
    #[test]
    fn the_way_out_does_not_offer_early_data() {
        let dir = dir("no-early-data");
        let ca = ca(&dir);
        let out = testing::way(&ca, TrafficRules::default(), RelaySettings::default());
        assert!(
            !out.tls.enable_early_data,
            "early data on the way out is a request the server may answer twice"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
