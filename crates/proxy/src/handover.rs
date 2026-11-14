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
//! A client that says nothing in time is neither of those, and is given a
//! tunnel: a protocol whose server greets first gets one that way. Inside it,
//! hudsucker waits for the client's first bytes before it decides what to do
//! with them, so such a connection is passed along rather than served — which
//! is what raw TCP is until there is a mode for it.
//!
//! A flow handed over as an address is **named by its hello**, where the
//! client writes one: the first record is read here on its way past, the
//! `server_name` out of it becomes the [`Target::Named`] the proxy is asked
//! for, and `140.82.121.4:443` is called `github.com:443` instead. See
//! [`named`] and [`crate::name`]. A source that named the flow itself keeps
//! its own name, and a client that names nobody keeps the address.
//!
//! What is still missing is written up in
//! [capture-modes.md](../../../docs/design/capture-modes.md): the name
//! *replaces* the address rather than standing beside it, so the proxy
//! resolves it again on its way out and may reach a different address than
//! the client was going to. Only a plain `GET` or `POST` goes unnamed now —
//! it carries a `Host` header, which `as_absolute` deliberately does not
//! trust in place of what the source knew.

use crate::{
    name::{self, RECORD_HEAD, TLS_START},
    source::{Flow, Target},
};
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

/// How much of the client's first word is read to tell TLS from the rest: a
/// TLS record starts [`TLS_START`], and anything else is a request to be
/// read at the proxy port as it stands. Where it *is* TLS, the read goes on
/// to the end of that record, because the name the client was going to is in
/// it — see [`name`](crate::name).
const PEEK: usize = TLS_START.len();

/// How long the client is given to say that much. An HTTP request and a TLS
/// ClientHello both come at once, so something still silent after this is
/// neither, and is given a tunnel rather than held here for ever: a protocol
/// whose server greets first, or a connection somebody opened and thought
/// better of.
///
/// It is paid **once per connection**, not once. Connections opened at the
/// same time wait at the same time, but a client that opens one, waits for a
/// greeting, uses it and opens the next pays it again each time — ten of
/// those is twenty seconds of a database client looking broken. The way out,
/// when a source that deals in packets needs one, is to look at where the
/// connection was going: a destination port of 25, 3306 or 5432 is not HTTP
/// and could go straight to a tunnel with no wait at all.
const FIRST_WORD_WITHIN: Duration = Duration::from_secs(2);

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
    // The client's first word comes first, before the proxy is told
    // anything: it is what says whether this is TLS, and a TLS flow handed
    // over as an address carries in that same record the name it was going
    // to. Learning it here rather than after `remember` is what lets the
    // flow be remembered under the name — the handler reads `Handed` once,
    // and an entry corrected afterwards would be a race against it.
    let mut first = Vec::with_capacity(PEEK);
    let waited = tokio::time::timeout(FIRST_WORD_WITHIN, opening(&mut stream, &mut first)).await;
    let silent = waited.is_err();
    if let Ok(read) = waited {
        read?;
    }
    let to = named(to, &first);

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
    let result = tunnel(&mut stream, &mut to_proxy, &to, &first, silent).await;
    handed.forget(from);
    result.with_context(|| format!("a connection from {client} to {}", to.authority()))
}

/// Where the flow was going, by name if its first record names it.
///
/// A source that reads packets knows an address and nothing else, so this is
/// the one chance to learn the name: `140.82.121.4:443` becomes
/// `github.com:443`, which is what the flow is called, what goes in the
/// `Host` header, and what the proxy asks the server for — a server with
/// several names on one address answers the wrong one otherwise.
///
/// What it buys is the connection out. [`upstream`](crate::upstream) takes
/// the host from the URI, so an address there becomes a rustls
/// `ServerName::IpAddress`: no SNI is sent, and the certificate is checked
/// against the address, which a public certificate does not carry. A TLS
/// flow taken as an address therefore failed on its way out, not merely read
/// badly. The client's own side was never the problem — hudsucker issues its
/// leaf from the SNI the client sent.
///
/// **The address is replaced, not kept alongside**, and that costs
/// something. The proxy looks the name up again on its way out, so DNS here
/// can answer with a different address than the client was going to — a
/// CDN's nearer edge, a split-horizon answer — or with nothing at all, for a
/// name only the client's own resolver knows. That last one is a connection
/// that used to be made and now fails, which is the real price of this and
/// not a theoretical one. It is the trade `socks5h://` has always made, for
/// a handshake that works instead of one that cannot. Keeping both — the
/// name to show and ask with, the address to connect to — needs the
/// connector to take a destination of its own, written up in
/// [capture-modes.md](../../../docs/design/capture-modes.md) and still to
/// come.
///
/// A source that named the flow itself is left alone: it read the name from
/// the same place or better, and may have one no record carries.
fn named(to: Target, first: &[u8]) -> Target {
    let Target::Addr(addr) = to else {
        return to;
    };
    match name::in_client_hello(first) {
        Some(host) => Target::Named {
            host,
            port: addr.port(),
        },
        None => Target::Addr(addr),
    }
}

/// Two ways in, told apart by what the client says first.
async fn tunnel(
    stream: &mut Box<dyn crate::source::Duplex>,
    to_proxy: &mut TcpStream,
    to: &Target,
    first: &[u8],
    silent: bool,
) -> Result<()> {
    if silent || first.starts_with(TLS_START) {
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
    to_proxy.write_all(first).await?;
    tokio::io::copy_bidirectional(stream, to_proxy).await?;
    Ok(())
}

/// The client's first bytes, read into `first` to be passed on with the rest:
/// there is nothing to peek with on a `dyn Duplex`. A client that says
/// nothing and goes gives none. It reads into a buffer the caller owns
/// because the caller gives up on it after a while, and the bytes that did
/// arrive still have to be forwarded.
///
/// Two bytes decide the shape. Anything but TLS stops there, because a
/// request is read at the proxy port as it stands and reading further would
/// wait on a client that has already said its piece — the next thing an HTTP
/// client sends may be a whole response's worth of silence away.
///
/// TLS reads on to the end of the first record, which is where the name is.
/// That costs nothing in the usual case, because a `ClientHello` arrives
/// whole and in one go; a client that stalls half-way through one pays the
/// same [`FIRST_WORD_WITHIN`] a silent client pays, and is then handed on
/// with whatever arrived, named by its address. Slower than before for that
/// one client, and no different for the rest.
async fn opening(stream: &mut Box<dyn crate::source::Duplex>, first: &mut Vec<u8>) -> Result<()> {
    read_upto(stream, first, PEEK).await?;
    if !first.starts_with(TLS_START) {
        return Ok(());
    }
    read_upto(stream, first, RECORD_HEAD).await?;
    let Some(record) = name::record_len(first) else {
        return Ok(()); // The header never arrived whole.
    };
    read_upto(stream, first, record).await
}

/// Reads until `first` holds `want` bytes, or the client stops. Never asks
/// for more than that, so nothing is read that belongs to the conversation
/// rather than to its opening.
async fn read_upto(
    stream: &mut Box<dyn crate::source::Duplex>,
    first: &mut Vec<u8>,
    want: usize,
) -> Result<()> {
    let mut chunk = [0u8; 1024];
    while first.len() < want {
        let room = (want - first.len()).min(chunk.len());
        // `read` loses nothing when it is cancelled, which is what the
        // deadline above does to it: what it has already put in `first`
        // stays there, and the caller forwards it either way.
        let read = stream.read(&mut chunk[..room]).await?;
        if read == 0 {
            break;
        }
        first.extend_from_slice(&chunk[..read]);
    }
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

    /// A client that says nothing is not a plain HTTP client, so it is given
    /// a tunnel instead of being held for ever — and then a server that
    /// greets first can reach it, which is the case this exists for.
    #[tokio::test]
    async fn a_client_that_says_nothing_is_given_a_tunnel_and_hears_the_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = listener.local_addr().unwrap();
        let greeting = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if sock.read(&mut byte).await.unwrap() == 0 {
                    break;
                }
                head.push(byte[0]);
            }
            sock.write_all(OK.as_bytes()).await.unwrap();
            // As a mail or database server does: it speaks first.
            sock.write_all(b"220 hello").await.unwrap();
            sock.shutdown().await.unwrap();
            String::from_utf8_lossy(&head).into_owned()
        });
        let (mut mine, theirs) = tokio::io::duplex(1024);
        let handed = Handed::default();
        let heard = async {
            let mut said = String::new();
            mine.read_to_string(&mut said).await.unwrap();
            assert_eq!(said, "220 hello", "the server's greeting reached it");
            drop(mine);
        };
        let (giving, ()) = tokio::join!(give(flow(theirs, None), proxy, &handed), heard);
        giving.unwrap();
        let head = greeting.await.unwrap();
        assert!(
            head.starts_with("CONNECT github.com:443 HTTP/1.1\r\n"),
            "{head:?}"
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

    /// A flow as a source that reads packets hands one over: an address, and
    /// nothing else to go on.
    fn flow_to(stream: tokio::io::DuplexStream, to: Target) -> Flow {
        Flow {
            stream: Box::new(stream),
            client: "192.168.1.13:51234".parse().unwrap(),
            to,
            who: None,
        }
    }

    /// Writes its opening, hears the server out, and goes.
    async fn opens_with(mut client: tokio::io::DuplexStream, what: Vec<u8>) {
        client.write_all(&what).await.unwrap();
        client.shutdown().await.unwrap();
        let mut said = String::new();
        client.read_to_string(&mut said).await.unwrap();
        assert_eq!(said, "from the server");
    }

    /// The whole point of reading the hello: a flow that arrived as
    /// `140.82.121.4:443` is asked for as `github.com:443`, so the server
    /// answers for the name it holds a certificate for rather than for
    /// whichever of its names happens to share that address.
    #[tokio::test]
    async fn a_flow_taken_as_an_address_is_asked_for_by_the_name_in_its_hello() {
        let (proxy, served) = fake_proxy(OK).await;
        let (mine, theirs) = tokio::io::duplex(4096);
        let handed = Handed::default();
        let hello = name::hello(&[name::sni("github.com")]);
        let to = Target::Addr("140.82.121.4:443".parse().unwrap());
        let (giving, ()) = tokio::join!(
            give(flow_to(theirs, to), proxy, &handed),
            opens_with(mine, hello.clone())
        );
        giving.unwrap();

        let got = served.await.unwrap();
        let head = String::from_utf8_lossy(&got[..got.len() - hello.len()]).to_string();
        assert_eq!(
            head, "CONNECT github.com:443 HTTP/1.1\r\nHost: github.com:443\r\n\r\n",
            "asked for by name, where the address was all the source knew"
        );
        assert!(
            got.ends_with(&hello),
            "and the hello itself was passed on whole, not eaten by the look at it"
        );
    }

    /// Reading it is allowed to fail. A client that names nobody — an old
    /// one, or one talking to an address on purpose — leaves the flow
    /// exactly as it arrived, which is how it was before any of this.
    #[tokio::test]
    async fn a_hello_that_names_nobody_leaves_the_flow_its_address() {
        let (proxy, served) = fake_proxy(OK).await;
        let (mine, theirs) = tokio::io::duplex(4096);
        let handed = Handed::default();
        let hello = name::hello(&[]);
        let to = Target::Addr("140.82.121.4:443".parse().unwrap());
        let (giving, ()) = tokio::join!(
            give(flow_to(theirs, to), proxy, &handed),
            opens_with(mine, hello.clone())
        );
        giving.unwrap();

        let got = served.await.unwrap();
        assert!(
            String::from_utf8_lossy(&got).starts_with("CONNECT 140.82.121.4:443 "),
            "{:?}",
            String::from_utf8_lossy(&got[..got.len() - hello.len()])
        );
    }

    /// A source that named the flow itself is not second-guessed. It read
    /// the name from the same record or from a SOCKS request, and may hold
    /// one the record cannot carry; a hello saying otherwise is the client
    /// talking, not the OS.
    #[tokio::test]
    async fn a_name_the_source_knew_is_not_replaced_by_the_hello() {
        let (proxy, served) = fake_proxy(OK).await;
        let (mine, theirs) = tokio::io::duplex(4096);
        let handed = Handed::default();
        let hello = name::hello(&[name::sni("somewhere.else")]);
        let (giving, ()) = tokio::join!(
            give(flow(theirs, None), proxy, &handed),
            opens_with(mine, hello.clone())
        );
        giving.unwrap();

        let got = served.await.unwrap();
        assert!(
            String::from_utf8_lossy(&got).starts_with("CONNECT github.com:443 "),
            "the source's own name stands"
        );
    }

    /// A hello split across two segments, which is what a `ClientHello` with
    /// a key share in it looks like on a real network. The name is only
    /// there to be read if the read goes on until the record is whole, so
    /// this is the test that the loop is a loop.
    #[tokio::test]
    async fn a_hello_that_arrives_in_two_pieces_is_still_read() {
        let (proxy, served) = fake_proxy(OK).await;
        let (mut mine, theirs) = tokio::io::duplex(4096);
        let handed = Handed::default();
        let hello = name::hello(&[name::sni("github.com"), (0x0033, vec![7; 600])]);
        let (head, tail) = hello.split_at(8);
        let to = Target::Addr("140.82.121.4:443".parse().unwrap());

        let client = async {
            mine.write_all(head).await.unwrap();
            mine.flush().await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
            mine.write_all(tail).await.unwrap();
            mine.shutdown().await.unwrap();
            let mut said = String::new();
            mine.read_to_string(&mut said).await.unwrap();
        };
        let (giving, ()) = tokio::join!(give(flow_to(theirs, to), proxy, &handed), client);
        giving.unwrap();

        let got = served.await.unwrap();
        assert!(
            String::from_utf8_lossy(&got).starts_with("CONNECT github.com:443 "),
            "the record was waited for, not read once and given up on"
        );
        assert!(got.ends_with(&hello), "and all of it was passed on");
    }
}
