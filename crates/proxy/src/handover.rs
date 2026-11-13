//! A connection taken on its way, made to look like a client that asked.
//!
//! The proxy core is [hudsucker](https://docs.rs/hudsucker), which owns its
//! accept loop and takes a `TcpListener`: the type that serves a single
//! connection is private to it. So a connection a source took from the OS
//! reaches the core the way any other client does — by connecting to
//! tracewing's own proxy port and saying `CONNECT` on the flow's behalf.
//!
//! What the source knew is kept here while the handover lasts, under the
//! address the handover arrives from, so that the flow is put down as the
//! program that really made it rather than as `127.0.0.1`.
//!
//! The cost is one loopback hop per taken connection. Clients that asked for
//! the proxy do not come through here at all, so the path that has to carry a
//! million flows is untouched.
//!
//! Two limits, both written up in
//! [capture-modes.md](../../../docs/design/capture-modes.md): inside a
//! `CONNECT` tunnel hudsucker intercepts TLS and a plain `GET`, and tunnels
//! anything else blind — so transparent plain HTTP needs its request lines
//! rewritten into absolute form instead, which is not built yet. And a flow
//! handed over as an address is named by that address; a source that can read
//! the ClientHello's `server_name` should hand over [`Target::Named`].

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

/// What a source knew about a connection it handed over.
#[derive(Clone)]
pub struct Known {
    /// The client as the source saw it, which is not the loopback address the
    /// handover arrives from.
    pub client: SocketAddr,
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
            app: who.map(told),
        },
    );
    let result = tunnel(&mut stream, &mut to_proxy, &to).await;
    handed.forget(from);
    result.with_context(|| format!("a connection from {client} to {}", to.authority()))
}

async fn tunnel(
    stream: &mut Box<dyn crate::source::Duplex>,
    to_proxy: &mut TcpStream,
    to: &Target,
) -> Result<()> {
    let authority = to.authority();
    to_proxy
        .write_all(format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n\r\n").as_bytes())
        .await?;
    tokio::time::timeout(ANSWER_WITHIN, answered(to_proxy))
        .await
        .map_err(|_| anyhow::anyhow!("the proxy did not answer CONNECT within {ANSWER_WITHIN:?}"))?
        .context("the proxy's answer to CONNECT")?;
    tokio::io::copy_bidirectional(stream, to_proxy).await?;
    Ok(())
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

    /// A stand-in for tracewing's own proxy port: it takes the `CONNECT`,
    /// says what it was told to say, and then, if that was a `200`, holds one
    /// exchange so that both directions of the splice are seen to work.
    /// It gives back the head it was sent.
    async fn fake_proxy(answer: &'static str) -> (SocketAddr, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let served = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if sock.read(&mut byte).await.unwrap() == 0 {
                    break;
                }
                head.push(byte[0]);
            }
            sock.write_all(answer.as_bytes()).await.unwrap();
            if answer.contains(" 200 ") {
                let mut asked = [0u8; 5];
                sock.read_exact(&mut asked).await.unwrap();
                assert_eq!(&asked, b"hello", "what the client said came through");
                sock.write_all(b"from the server").await.unwrap();
                sock.shutdown().await.unwrap();
            }
            String::from_utf8_lossy(&head).into_owned()
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

    /// Says `hello`, hears the server, and goes: a whole little conversation
    /// over a connection that asked for no proxy.
    async fn talk(mut client: tokio::io::DuplexStream) {
        client.write_all(b"hello").await.unwrap();
        let mut said = String::new();
        client.read_to_string(&mut said).await.unwrap();
        assert_eq!(said, "from the server");
    }

    #[tokio::test]
    async fn it_says_connect_for_a_flow_that_never_asked() {
        let (proxy, served) = fake_proxy(OK).await;
        let (mine, theirs) = tokio::io::duplex(1024);
        let handed = Handed::default();
        let (giving, ()) = tokio::join!(give(flow(theirs, None), proxy, &handed), talk(mine));
        giving.unwrap();
        let head = served.await.unwrap();
        assert!(
            head.starts_with("CONNECT github.com:443 HTTP/1.1\r\n"),
            "{head}"
        );
        assert!(head.contains("Host: github.com:443\r\n"), "{head}");
    }

    #[tokio::test]
    async fn the_client_and_its_program_are_known_while_it_lasts() {
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
                let one = handed.lock().values().next().cloned();
                if let Some(one) = one {
                    return one;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("a handover on record");
        assert_eq!(known.client.to_string(), "192.168.1.13:51234");
        let app = known.app.expect("told, not looked up").now().flatten();
        assert_eq!(
            app.map(|a| (a.pid, a.name)),
            Some((4123, "node".to_owned()))
        );
        talk(mine).await;
        giving.await.unwrap().unwrap();
        served.await.unwrap();
        assert!(
            handed.lock().is_empty(),
            "forgotten once the connection ends"
        );
    }

    #[tokio::test]
    async fn a_proxy_that_refuses_is_said_and_nothing_is_tunnelled() {
        let (proxy, served) = fake_proxy("HTTP/1.1 403 Forbidden\r\n\r\n").await;
        let (_mine, theirs) = tokio::io::duplex(1024);
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
