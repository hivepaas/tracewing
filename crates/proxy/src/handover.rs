//! A connection taken on its way, made to look like a client that asked.
//!
//! The proxy core is [hudsucker](https://docs.rs/hudsucker), which owns its
//! accept loop and takes a `TcpListener`: the type that serves a single
//! connection is private to it. So a connection a source took from the OS
//! reaches the core the way any other client does, by connecting to
//! tracewing's own proxy port — and then one of two ways, told apart by what
//! the client says first.
//!
//! **TLS** gets a `CONNECT` asked for on its behalf, since a tunnel is the
//! only place the proxy will terminate TLS.
//!
//! **Everything else** is passed on as it stands, and read at the proxy port
//! as any client's request is. It must not be wrapped in a tunnel: inside
//! one, hudsucker reads TLS and a plain `GET` and tunnels the rest blind, so
//! a `POST` would go through unseen. What such a client writes is a path and
//! a `Host` header, because it believes it is talking to the server, and
//! filling the server into the URI is [`crate::capture`]'s `as_absolute`,
//! which reads it from what the source knew. Keeping a whole HTTP/1 parser
//! here to rewrite request lines would be the other way to do it, and this
//! leaves the framing to hyper, which already has one.
//!
//! What the source knew is kept here while the handover lasts, under the
//! address the handover arrives from: the program that really made the
//! connection, the address it came from rather than `127.0.0.1`, and where it
//! was going.
//!
//! The cost is one loopback hop per taken connection. Clients that asked for
//! the proxy do not come through here at all, so the path that has to carry a
//! million flows is untouched.
//!
//! One limit is left, written up in
//! [capture-modes.md](../../../docs/design/capture-modes.md): a flow handed
//! over as an address is named by that address, and asks the server with it.
//! A source that can read the ClientHello's `server_name`, or a SOCKS
//! request's host, should hand over [`Target::Named`].

use crate::source::{Flow, Target};
use anyhow::{Context, Result, bail};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracewing_store::app::{App, AppAnswer};

/// How long tracewing's own proxy port is given to answer the `CONNECT`.
/// It is this machine answering itself, so this is a bug's timeout, not a
/// network's.
const ANSWER_WITHIN: Duration = Duration::from_secs(10);

/// The most a `200` and its headers may be. Past this something is answering
/// that is not the proxy.
const ANSWER_MAX: usize = 8 * 1024;

/// How much of the client's first word is read to tell TLS from the rest. A
/// TLS record starts `0x16 0x03`: a handshake, of SSL 3.x or of any TLS.
const PEEK: usize = 2;

/// What a source knew about a connection it handed over.
#[derive(Clone)]
pub struct Known {
    /// The client as the source saw it, which is not the loopback address the
    /// handover arrives from.
    pub client: SocketAddr,
    /// Where it was going. A client taken on its way asks for a path and
    /// names a host in a header, if it names one at all; this is what the
    /// source knew, and it has the port, which a `Host` header may not.
    pub to: Target,
    /// The program behind it, answered already: a source that took the
    /// connection from the OS was told, so nothing has to be looked up.
    pub app: Option<Arc<AppAnswer>>,
}

/// The handovers going on now, by the address each arrives from. A handful at
/// a time, and each entry lives as long as its connection.
#[derive(Default)]
pub struct Handed(Mutex<HashMap<SocketAddr, Known>>);

impl Handed {
    /// What is known of the connection arriving from `from`, if it is a
    /// handover at all.
    pub fn of(&self, from: SocketAddr) -> Option<Known> {
        self.lock().get(&from).cloned()
    }

    fn remember(&self, from: SocketAddr, known: Known) {
        self.lock().insert(from, known);
    }

    fn forget(&self, from: SocketAddr) {
        self.lock().remove(&from);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<SocketAddr, Known>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// An answer that is already in, for a program the OS named.
fn told(app: App) -> Arc<AppAnswer> {
    let answer = Arc::new(AppAnswer::default());
    answer.give(Some(app));
    answer
}

/// Hands `flow` to the proxy listening at `proxy`, and stays until the
/// connection ends.
pub async fn give(flow: Flow, proxy: SocketAddr, handed: &Handed) -> Result<()> {
    let Flow {
        mut stream,
        client,
        to,
        who,
    } = flow;
    let mut to_proxy = TcpStream::connect(proxy)
        .await
        .with_context(|| format!("hand a connection from {client} to the proxy at {proxy}"))?;
    to_proxy.set_nodelay(true)?;
    let from = to_proxy.local_addr()?;
    handed.remember(
        from,
        Known {
            client,
            to: to.clone(),
            app: who.map(told),
        },
    );
    let result = tunnel(&mut stream, &mut to_proxy, &to).await;
    handed.forget(from);
    result.with_context(|| format!("a connection from {client} to {}", to.authority()))
}

/// Two ways in, told apart by what the client says first.
async fn tunnel(
    stream: &mut Box<dyn crate::source::Duplex>,
    to_proxy: &mut TcpStream,
    to: &Target,
) -> Result<()> {
    let first = first_bytes(stream).await?;
    if first.starts_with(b"\x16\x03") {
        // TLS. The proxy terminates it, but only inside a tunnel, so ask for
        // one on the client's behalf — it has no idea a proxy is involved.
        let authority = to.authority();
        to_proxy
            .write_all(
                format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n\r\n").as_bytes(),
            )
            .await?;
        tokio::time::timeout(ANSWER_WITHIN, answered(to_proxy))
            .await
            .map_err(|_| {
                anyhow::anyhow!("the proxy did not answer CONNECT within {ANSWER_WITHIN:?}")
            })?
            .context("the proxy's answer to CONNECT")?;
    }
    // Anything else goes to the proxy port as it is, and is read there as any
    // client's request is. A client taken on its way writes `POST /orders`
    // and names the host in a header, because it believes it is talking to
    // the server; `Handed` is how the handler learns which server that was.
    // A tunnel is no use for that: inside one, only TLS and a plain `GET`
    // are read, and a `POST` would pass through unseen.
    to_proxy.write_all(&first).await?;
    tokio::io::copy_bidirectional(stream, to_proxy).await?;
    Ok(())
}

/// The client's first bytes, read and kept to be passed on: there is nothing
/// to peek with on a `dyn Duplex`, so this reads them and hands them over
/// with the rest. A client that says nothing and goes gives none.
async fn first_bytes(stream: &mut Box<dyn crate::source::Duplex>) -> Result<Vec<u8>> {
    let mut first = Vec::with_capacity(PEEK);
    let mut byte = [0u8; 1];
    while first.len() < PEEK {
        if stream.read(&mut byte).await? == 0 {
            break;
        }
        first.push(byte[0]);
    }
    Ok(first)
}

/// Reads the answer to `CONNECT` and no further. A byte at a time, because
/// whatever follows the blank line is the client's own first bytes: reading
/// into them would swallow the start of the conversation. It is some forty
/// bytes, once per connection.
async fn answered(to_proxy: &mut TcpStream) -> Result<()> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= ANSWER_MAX {
            bail!("no end to it in {ANSWER_MAX} bytes");
        }
        if to_proxy.read(&mut byte).await? == 0 {
            bail!("the proxy closed before answering");
        }
        head.push(byte[0]);
    }
    let status = String::from_utf8_lossy(&head);
    let first = status.lines().next().unwrap_or_default();
    let code = first.split_whitespace().nth(1).unwrap_or_default();
    if code != "200" {
        bail!("{first}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Flow;
    use tokio::net::TcpListener;

    const OK: &str = "HTTP/1.1 200 Connection established\r\n\r\n";
    /// Only the first two bytes have to be a TLS record's; nothing in these
    /// tests takes it any further than the splice.
    const TLS_HELLO: &[u8] = b"\x16\x03\x01\x00\x05hello";
    const PLAIN_POST: &[u8] =
        b"POST /orders HTTP/1.1\r\nHost: api.test\r\nContent-Length: 2\r\n\r\nhi";

    /// A stand-in for tracewing's own proxy port. It keeps everything it is
    /// sent; if a `CONNECT` arrives it answers with `answer`; and then it
    /// says one thing back, so that both directions of the splice are seen
    /// to work. It gives back all it was sent.
    async fn fake_proxy(answer: &'static str) -> (SocketAddr, tokio::task::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let served = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut got = Vec::new();
            let mut byte = [0u8; 1];
            // To the end of the first head, whatever kind of head it is.
            while !got.ends_with(b"\r\n\r\n") {
                if sock.read(&mut byte).await.unwrap() == 0 {
                    break;
                }
                got.push(byte[0]);
            }
            if got.starts_with(b"CONNECT ") {
                sock.write_all(answer.as_bytes()).await.unwrap();
                if !answer.contains(" 200 ") {
                    return got;
                }
            }
            // Whatever else the client had to say: a body, or the TLS that
            // followed the tunnel.
            let mut rest = Vec::new();
            sock.read_to_end(&mut rest).await.unwrap();
            got.extend_from_slice(&rest);
            sock.write_all(b"from the server").await.ok();
            sock.shutdown().await.ok();
            got
        });
        (addr, served)
    }

    fn flow(stream: tokio::io::DuplexStream, who: Option<App>) -> Flow {
        Flow {
            stream: Box::new(stream),
            client: "192.168.1.13:51234".parse().unwrap(),
            to: Target::Named {
                host: "github.com".to_owned(),
                port: 443,
            },
            who,
        }
    }

    /// Says its piece, hears the server, and goes: a client that was taken on
    /// its way and never knew a proxy was there.
    async fn talk(mut client: tokio::io::DuplexStream, what: &'static [u8]) {
        client.write_all(what).await.unwrap();
        client.shutdown().await.unwrap();
        let mut said = String::new();
        client.read_to_string(&mut said).await.unwrap();
        assert_eq!(said, "from the server");
    }

    /// The point of the two ways in. A `POST` wrapped in `CONNECT` would be
    /// tunnelled blind — inside a tunnel the proxy reads TLS and a plain
    /// `GET` and nothing else — so a plain request must arrive as itself.
    #[tokio::test]
    async fn a_plain_request_is_not_wrapped_in_a_tunnel() {
        let (proxy, served) = fake_proxy(OK).await;
        let (mine, theirs) = tokio::io::duplex(1024);
        let handed = Handed::default();
        let (giving, ()) = tokio::join!(
            give(flow(theirs, None), proxy, &handed),
            talk(mine, PLAIN_POST)
        );
        giving.unwrap();
        let got = served.await.unwrap();
        assert_eq!(
            got, PLAIN_POST,
            "the request reached the proxy port exactly as the client wrote it"
        );
        assert!(
            !got.starts_with(b"CONNECT "),
            "and nothing was put in front"
        );
    }

    #[tokio::test]
    async fn tls_asks_for_a_tunnel_on_the_clients_behalf() {
        let (proxy, served) = fake_proxy(OK).await;
        let (mine, theirs) = tokio::io::duplex(1024);
        let handed = Handed::default();
        let (giving, ()) = tokio::join!(
            give(flow(theirs, None), proxy, &handed),
            talk(mine, TLS_HELLO)
        );
        giving.unwrap();
        let got = served.await.unwrap();
        let text = String::from_utf8_lossy(&got);
        assert!(
            text.starts_with("CONNECT github.com:443 HTTP/1.1\r\n"),
            "{text:?}"
        );
        assert!(text.contains("Host: github.com:443\r\n"), "{text:?}");
        assert!(
            got.ends_with(TLS_HELLO),
            "the bytes read to tell TLS apart were passed on, all of them"
        );
    }

    #[tokio::test]
    async fn the_client_its_program_and_where_it_went_are_known_while_it_lasts() {
        let (proxy, served) = fake_proxy(OK).await;
        let (mine, theirs) = tokio::io::duplex(1024);
        let handed = Arc::new(Handed::default());
        let app = App {
            pid: 4123,
            name: "node".to_owned(),
            path: Some("/usr/bin/node".to_owned()),
        };
        let giving = {
            let handed = Arc::clone(&handed);
            tokio::spawn(async move { give(flow(theirs, Some(app)), proxy, &handed).await })
        };
        // On record under the address the handover arrives from, which is the
        // address the proxy's handler sees as the client.
        let known = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(one) = handed.lock().values().next().cloned() {
                    return one;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("a handover on record");
        assert_eq!(known.client.to_string(), "192.168.1.13:51234");
        assert_eq!(
            known.to,
            Target::Named {
                host: "github.com".to_owned(),
                port: 443
            },
            "where it was going, for the handler to put in the URI"
        );
        let app = known.app.expect("told, not looked up").now().flatten();
        assert_eq!(
            app.map(|a| (a.pid, a.name)),
            Some((4123, "node".to_owned()))
        );
        talk(mine, PLAIN_POST).await;
        giving.await.unwrap().unwrap();
        served.await.unwrap();
        assert!(
            handed.lock().is_empty(),
            "forgotten once the connection ends"
        );
    }

    #[tokio::test]
    async fn a_proxy_that_refuses_a_tunnel_is_said_and_nothing_is_spliced() {
        let (proxy, served) = fake_proxy("HTTP/1.1 403 Forbidden\r\n\r\n").await;
        let (mut mine, theirs) = tokio::io::duplex(1024);
        // It fits the buffer, so this needs nobody at the other end yet.
        mine.write_all(TLS_HELLO).await.unwrap();
        let handed = Handed::default();
        let problem = give(flow(theirs, None), proxy, &handed)
            .await
            .expect_err("403 is not a tunnel");
        let said = format!("{problem:#}");
        assert!(said.contains("403"), "{said}");
        assert!(said.contains("github.com:443"), "{said}");
        assert!(handed.lock().is_empty(), "forgotten even when it failed");
        served.await.unwrap();
    }
}
