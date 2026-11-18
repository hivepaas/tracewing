//! What a browser and a server will really do over QUIC, before tracewing is
//! built to speak it.
//!
//! Capturing HTTP/3 means terminating QUIC on both sides, and two questions
//! decide whether that is worth building at all. Neither can be answered by
//! reading: a browser's QUIC is governed by its own policies, and a server's
//! by what it deployed. So this asks them.
//!
//! - `serve [port] [host]` — an HTTP/3 server holding a leaf from
//!   **tracewing's own CA**, through [`LocalCa::config_for`] with
//!   [`Over::Quic`], which is the same minting the proxy would do. Nothing is
//!   proved by a certificate a browser was told to ignore, so this uses the
//!   real one, and the browser has to be one that trusts tracewing's CA.
//! - `altsvc [port] [host]` — the same, with TLS over TCP beside it on the
//!   same port offering `Alt-Svc: h3`, to watch whether a browser moves to
//!   QUIC on its own rather than being forced onto it.
//! - `fetch <url>` — the other direction: HTTP/3 to a real server, with the
//!   steps timed the way [`tracewing_proxy::upstream`] times TCP and TLS.
//!
//! What it measured, and the traps, are written up in
//! `docs/design/http3.md`. Two worth repeating here:
//!
//! - **Bind both families.** A browser asked for `localhost` goes to `::1`
//!   first, and an endpoint on `0.0.0.0` never hears it: the first answer
//!   this gave was `ERR_QUIC_HANDSHAKE_FAILED`, which looked exactly like a
//!   browser refusing a locally-trusted certificate and was nothing of the
//!   kind.
//! - **A loopback race is not a real one.** Offered both, a browser races
//!   QUIC against TCP, and over loopback TCP wins every time, so the QUIC
//!   attempt is cancelled and the offer ends up marked broken. That is the
//!   test bench, not the world.
//!
//! `cargo run -p tracewing-dev --bin quicprobe -- serve 4433 localhost`

use anyhow::{Context as _, Result};
use bytes::Buf as _;
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};
use tracewing_proxy::{
    ca::{LocalCa, Over},
    ca_key::KeyStore,
    crypto_provider, default_data_dir,
};

/// tracewing's own CA, from the folder the app keeps it in, with its key
/// wherever that is -- the keychain, for an app that has run once.
fn ca(dir: Option<PathBuf>) -> Result<LocalCa> {
    let dir = dir.unwrap_or_else(default_data_dir);
    println!("CA from {}", dir.display());
    LocalCa::load_or_create(&dir, crypto_provider(), KeyStore::System)
}

/// An HTTP/3 endpoint answering anything with one line, so that what is being
/// watched is the handshake and the request rather than a page.
async fn serve(port: u16, host: &str) -> Result<()> {
    let config = ca(None)?.config_for(host, Over::Quic);
    let quic = quinn::crypto::rustls::QuicServerConfig::try_from(config)?;
    // Both families: see the note at the top of this file.
    let addr = SocketAddr::from(([0u16; 8], port));
    let endpoint = quinn::Endpoint::server(quinn::ServerConfig::with_crypto(Arc::new(quic)), addr)?;
    println!("HTTP/3 on [::]:{port}, certificate for {host}");

    while let Some(incoming) = endpoint.accept().await {
        tokio::spawn(async move {
            let at = Instant::now();
            let conn = match incoming.await {
                Ok(conn) => conn,
                Err(e) => return println!("!! handshake failed: {e}"),
            };
            let alpn = conn
                .handshake_data()
                .and_then(|d| d.downcast::<quinn::crypto::rustls::HandshakeData>().ok())
                .and_then(|d| d.protocol.clone())
                .map(|p| String::from_utf8_lossy(&p).into_owned())
                .unwrap_or_default();
            println!(
                "** QUIC from {} in {:?}, alpn {alpn:?}",
                conn.remote_address(),
                at.elapsed()
            );
            let mut h3 = match h3::server::Connection::new(h3_quinn::Connection::new(conn)).await {
                Ok(h3) => h3,
                Err(e) => return println!("!! no HTTP/3 over it: {e}"),
            };
            loop {
                match h3.accept().await {
                    Ok(Some(resolver)) => {
                        tokio::spawn(async move {
                            let (req, mut stream) = match resolver.resolve_request().await {
                                Ok(pair) => pair,
                                Err(e) => return println!("!! request: {e}"),
                            };
                            let agent = req
                                .headers()
                                .get("user-agent")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("-")
                                .to_owned();
                            println!(">> HTTP/3 {} {} from {agent}", req.method(), req.uri());
                            let body = b"tracewing: this arrived over HTTP/3\n";
                            let answer = http::Response::builder()
                                .status(200)
                                .header("content-type", "text/plain")
                                .header("content-length", body.len())
                                .body(())
                                .expect("a response of our own");
                            if let Err(e) = stream.send_response(answer).await {
                                return println!("!! answering: {e}");
                            }
                            let _ = stream.send_data(bytes::Bytes::from_static(body)).await;
                            let _ = stream.finish().await;
                        });
                    }
                    Ok(None) => return,
                    Err(e) => return println!("-- connection ended: {e}"),
                }
            }
        });
    }
    Ok(())
}

/// The same endpoint, with TLS over TCP beside it offering HTTP/3, which is
/// how a browser learns of it in the ordinary way. Every answer says which
/// way it arrived, so the log shows where the browser went.
async fn altsvc(port: u16, host: &str) -> Result<()> {
    let quic_host = host.to_owned();
    tokio::spawn(async move {
        if let Err(e) = serve(port, &quic_host).await {
            println!("!! the QUIC half: {e:#}");
        }
    });

    let config = ca(None)?.config_for(host, Over::Tcp);
    let acceptor = tokio_rustls::TlsAcceptor::from(config);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0u16; 8], port))).await?;
    println!("TLS over TCP on [::]:{port} as well, offering h3");
    loop {
        let (socket, _) = listener.accept().await?;
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
            let Ok(mut tls) = acceptor.accept(socket).await else {
                return println!("!! a TLS handshake over TCP failed");
            };
            let mut buffer = [0u8; 4096];
            loop {
                let Ok(read) = tls.read(&mut buffer).await else {
                    return;
                };
                if read == 0 {
                    return;
                }
                let text = String::from_utf8_lossy(&buffer[..read]);
                let path = text.split_whitespace().nth(1).unwrap_or("?").to_owned();
                println!("-- over TCP: {path}");
                // Two sub-resources, so that a second request on the same
                // page can go the other way and be seen doing it.
                let body = "<html><body>over TCP\
                            <img src=\"/one.png\"><img src=\"/two.png\"></body></html>";
                let head = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\n\
                     content-length: {}\r\nalt-svc: h3=\":{port}\"; ma=86400\r\n\r\n",
                    body.len()
                );
                if tls.write_all(head.as_bytes()).await.is_err()
                    || tls.write_all(body.as_bytes()).await.is_err()
                {
                    return;
                }
            }
        });
    }
}

/// HTTP/3 to a real server, timed in the steps `upstream` keeps: the name
/// looked up, the handshake, then the first answer.
async fn fetch(url: &str) -> Result<()> {
    let uri: http::Uri = url.parse()?;
    let host = uri.host().context("a host in the url")?.to_owned();
    let port = uri.port_u16().unwrap_or(443);

    let at = Instant::now();
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.clone(), port))
        .await?
        .collect();
    let looked_up = at.elapsed();
    // The endpoint below is bound to one family, so the address has to be of
    // that family: IPv4 where there is one.
    let addr = addrs
        .iter()
        .find(|a| a.is_ipv4())
        .or(addrs.first())
        .copied()
        .context("the name has no address")?;

    let mut roots = hudsucker::rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(cert);
    }
    let mut tls =
        hudsucker::rustls::ClientConfig::builder_with_provider(Arc::new(crypto_provider()))
            .with_protocol_versions(&[&hudsucker::rustls::version::TLS13])?
            .with_root_certificates(roots)
            .with_no_client_auth();
    tls.alpn_protocols = vec![b"h3".to_vec()];

    let mut endpoint = quinn::Endpoint::client(if addr.is_ipv4() {
        SocketAddr::from(([0, 0, 0, 0], 0))
    } else {
        SocketAddr::from(([0u16; 8], 0))
    })?;
    endpoint.set_default_client_config(quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
    )));

    let at = Instant::now();
    let conn = endpoint.connect(addr, &host)?.await?;
    let handshook = at.elapsed();
    let (mut driver, mut sender) = h3::client::new(h3_quinn::Connection::new(conn)).await?;
    let driving = tokio::spawn(async move { driver.wait_idle().await });

    let at = Instant::now();
    let mut stream = sender
        .send_request(http::Request::builder().uri(url).body(())?)
        .await?;
    stream.finish().await?;
    let answer = stream.recv_response().await?;
    let first = at.elapsed();
    let mut bytes = 0usize;
    while let Some(chunk) = stream.recv_data().await? {
        bytes += chunk.remaining();
    }
    println!(
        "{url} over {addr}\n  {} {:?}, {bytes} bytes of body\n  \
         name {looked_up:?}, handshake {handshook:?}, first answer {first:?}\n  \
         alt-svc {:?}",
        answer.status(),
        answer.version(),
        answer.headers().get("alt-svc"),
    );
    drop(sender);
    let _ = driving.await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = crypto_provider().install_default();
    let mut args = std::env::args().skip(1);
    let port = |args: &mut dyn Iterator<Item = String>| -> Result<u16> {
        Ok(args.next().unwrap_or_else(|| "4433".into()).parse()?)
    };
    match args.next().as_deref() {
        Some("serve") => {
            let (port, host) = (port(&mut args)?, args.next());
            serve(port, host.as_deref().unwrap_or("localhost")).await
        }
        Some("altsvc") => {
            let (port, host) = (port(&mut args)?, args.next());
            altsvc(port, host.as_deref().unwrap_or("localhost")).await
        }
        Some("fetch") => fetch(&args.next().context("a url to fetch")?).await,
        other => {
            if let Some(what) = other {
                println!("no such probe: {what}");
            }
            println!(
                "quicprobe serve [port] [host]   an HTTP/3 server with a leaf from tracewing's CA\n\
                 quicprobe altsvc [port] [host]  the same, with TLS over TCP offering it\n\
                 quicprobe fetch <url>           HTTP/3 to a real server"
            );
            Ok(())
        }
    }
}
