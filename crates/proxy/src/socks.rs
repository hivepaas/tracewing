//! SOCKS5 in: tracewing as a proxy a client was told to use, for the programs
//! that speak SOCKS and not HTTP proxying — anything reading `ALL_PROXY`,
//! Firefox's own SOCKS setting, `ssh -D`, proxychains.
//!
//! It is a source of taken connections rather than a second proxy port,
//! because once the handshake is over a SOCKS client says nothing further
//! about where it is going: what is left on the socket is the conversation
//! itself, which is exactly a [`Flow`]. The destination comes out of the
//! handshake, and with a name in it when the client sent one — SOCKS5 carries
//! a domain name, so [`Target::Named`] comes free here, where a source
//! reading packets has to find the name in a ClientHello.
//!
//! RFC 1928, and the parts of it tracewing does: no authentication, and
//! `CONNECT`. `BIND` and `UDP ASSOCIATE` are refused by their own reply codes
//! rather than ignored, so a client is told rather than left waiting.
//!
//! Two things to know:
//!
//! - **It answers before it knows.** RFC 1928 has the server connect and then
//!   report, but here the connecting is the proxy core's, further on, so the
//!   reply is sent as soon as the request is understood. A destination that
//!   turns out to be unreachable is a failure inside the conversation rather
//!   than a SOCKS reply, which is what every lazy SOCKS proxy does.
//! - **The flow has no program against it yet.** [`crate::apps`] finds the
//!   program behind a connection by looking for a socket whose far end is the
//!   proxy port, and a SOCKS client's far end is this port. Teaching it the
//!   port each connection came in on is a change across three operating
//!   systems, and the next piece of work here.

use crate::source::{Flow, Found, Source, Stop, Target, stopped};
use anyhow::{Context, Result, bail};
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const VERSION: u8 = 5;
/// The way in that needs no password, and the answer for having none to offer.
const NO_AUTH: u8 = 0;
const NO_METHOD: u8 = 0xFF;
const CONNECT: u8 = 1;
/// Address kinds: an IPv4, a name, an IPv6.
const V4: u8 = 1;
const NAME: u8 = 3;
const V6: u8 = 4;
/// Replies: it worked, it did not, the command is not one tracewing does, the
/// address is not a kind it does.
const OK: u8 = 0;
const FAILED: u8 = 1;
const NO_COMMAND: u8 = 7;
const NO_ADDRESS: u8 = 8;

/// How long a client is given to finish the handshake. It is a few bytes sent
/// at once by anything that means it.
const HANDSHAKE_WITHIN: Duration = Duration::from_secs(10);

/// A SOCKS5 port of tracewing's own.
pub struct Socks(TcpListener);

impl Socks {
    pub fn new(listener: TcpListener) -> Self {
        Self(listener)
    }
}

impl Source for Socks {
    fn name(&self) -> String {
        match self.0.local_addr() {
            Ok(at) => format!("the SOCKS5 port at {at}"),
            Err(_) => "the SOCKS5 port".to_owned(),
        }
    }

    fn start(self: Box<Self>, proxy: SocketAddr, found: Found, stop: Stop) -> Result<()> {
        let ours = self.0.local_addr()?;
        tokio::spawn(take_them(self.0, Ours { proxy, socks: ours }, found, stop));
        Ok(())
    }
}

/// The addresses a client must not be sent to, because they are tracewing's
/// own and it would be handing traffic back to itself.
#[derive(Clone, Copy)]
struct Ours {
    proxy: SocketAddr,
    socks: SocketAddr,
}

impl Ours {
    fn is_itself(&self, to: &Target) -> bool {
        match to {
            // A name that resolves to one of these is not caught: that would
            // mean looking it up here, which is the proxy's job and not this
            // one's. The loop it makes is one hop and ends in an error.
            Target::Named { .. } => false,
            Target::Addr(addr) => addr == &self.proxy || addr == &self.socks,
        }
    }
}

async fn take_them(listener: TcpListener, ours: Ours, found: Found, stop: Stop) {
    loop {
        tokio::select! {
            taken = listener.accept() => {
                let (stream, client) = match taken {
                    Ok(taken) => taken,
                    Err(e) => {
                        tracing::warn!("taking a SOCKS5 connection: {e}");
                        continue;
                    }
                };
                let found = found.clone();
                tokio::spawn(async move {
                    match take_one(stream, client, ours, &found).await {
                        Ok(()) => {}
                        // A client that speaks something else, or gives up
                        // in the middle: its own business, not a warning.
                        Err(e) => tracing::debug!("a SOCKS5 client at {client}: {e:#}"),
                    }
                });
            }
            _ = stopped(stop.clone()) => break,
        }
    }
}

async fn take_one(
    mut stream: TcpStream,
    client: SocketAddr,
    ours: Ours,
    found: &Found,
) -> Result<()> {
    stream.set_nodelay(true)?;
    let to = tokio::time::timeout(HANDSHAKE_WITHIN, greet(&mut stream))
        .await
        .map_err(|_| anyhow::anyhow!("it did not finish asking within {HANDSHAKE_WITHIN:?}"))??;
    if ours.is_itself(&to) {
        refuse(&mut stream, FAILED).await?;
        bail!(
            "it asked to be sent to tracewing itself, at {}",
            to.authority()
        );
    }
    // Everything after this is the conversation, for the proxy to read.
    stream
        .write_all(&reply(OK))
        .await
        .context("saying it may go ahead")?;
    found
        .flow(Flow {
            stream: Box::new(stream),
            client,
            to,
            who: None,
        })
        .await;
    Ok(())
}

/// The RFC 1928 handshake, this end of it: agree on no authentication, then
/// read where the client wants to go.
async fn greet(stream: &mut TcpStream) -> Result<Target> {
    let mut hello = [0u8; 2];
    stream
        .read_exact(&mut hello)
        .await
        .context("reading its greeting")?;
    if hello[0] != VERSION {
        bail!("it speaks version {}, not SOCKS5", hello[0]);
    }
    let mut ways = vec![0u8; hello[1] as usize];
    stream
        .read_exact(&mut ways)
        .await
        .context("reading the ways in it offers")?;
    if !ways.contains(&NO_AUTH) {
        stream.write_all(&[VERSION, NO_METHOD]).await?;
        bail!("it offers no way in that tracewing has: {ways:02x?}");
    }
    stream.write_all(&[VERSION, NO_AUTH]).await?;

    let mut asked = [0u8; 4];
    stream
        .read_exact(&mut asked)
        .await
        .context("reading what it asks for")?;
    if asked[0] != VERSION {
        bail!("its request says version {}, not SOCKS5", asked[0]);
    }
    if asked[1] != CONNECT {
        refuse(stream, NO_COMMAND).await?;
        bail!(
            "it asks for command {}, and tracewing does CONNECT",
            asked[1]
        );
    }
    match asked[3] {
        V4 => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).await.context("an address")?;
            let port = port(stream).await?;
            Ok(Target::Addr(SocketAddr::new(
                Ipv4Addr::from(addr).into(),
                port,
            )))
        }
        V6 => {
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr).await.context("an address")?;
            let port = port(stream).await?;
            Ok(Target::Addr(SocketAddr::new(
                Ipv6Addr::from(addr).into(),
                port,
            )))
        }
        NAME => {
            let mut len = [0u8; 1];
            stream
                .read_exact(&mut len)
                .await
                .context("a name's length")?;
            let mut name = vec![0u8; len[0] as usize];
            stream.read_exact(&mut name).await.context("a name")?;
            let host = String::from_utf8(name).context("a name that is not text")?;
            let port = port(stream).await?;
            if host.is_empty() {
                refuse(stream, FAILED).await?;
                bail!("it asks for a name of no characters");
            }
            Ok(Target::Named { host, port })
        }
        other => {
            refuse(stream, NO_ADDRESS).await?;
            bail!("it asks for address kind {other}");
        }
    }
}

async fn port(stream: &mut TcpStream) -> Result<u16> {
    let mut port = [0u8; 2];
    stream.read_exact(&mut port).await.context("a port")?;
    Ok(u16::from_be_bytes(port))
}

async fn refuse(stream: &mut TcpStream, why: u8) -> Result<()> {
    stream.write_all(&reply(why)).await?;
    Ok(())
}

/// A reply, with the address this end bound as zeros. RFC 1928 wants the
/// address there, and nothing has been bound: the connecting happens further
/// on, in the proxy. Clients do not read it — curl, ssh, Firefox and Chromium
/// all carry on — and sending zeros is what other proxies that connect late
/// send too.
fn reply(why: u8) -> [u8; 10] {
    [VERSION, why, 0, V4, 0, 0, 0, 0, 0, 0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Took;
    use tokio::sync::watch;

    /// A SOCKS5 port with nothing behind it but a channel, and a client
    /// talking to it over a real socket.
    async fn port_and_client() -> (
        TcpStream,
        tokio::sync::mpsc::Receiver<Took>,
        watch::Sender<bool>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let at = listener.local_addr().unwrap();
        let (found, took) = Found::channel();
        let (stop_tx, stop) = watch::channel(false);
        Box::new(Socks::new(listener))
            .start("127.0.0.1:9090".parse().unwrap(), found, stop)
            .unwrap();
        let client = TcpStream::connect(at).await.unwrap();
        (client, took, stop_tx)
    }

    /// The greeting and its answer, which every one of these starts with.
    async fn no_password(client: &mut TcpStream) {
        client.write_all(&[VERSION, 1, NO_AUTH]).await.unwrap();
        let mut answer = [0u8; 2];
        client.read_exact(&mut answer).await.unwrap();
        assert_eq!(answer, [VERSION, NO_AUTH]);
    }

    async fn asked_for(client: &mut TcpStream, request: &[u8]) -> [u8; 10] {
        client.write_all(request).await.unwrap();
        let mut answer = [0u8; 10];
        client.read_exact(&mut answer).await.unwrap();
        answer
    }

    fn flow_of(took: Option<Took>) -> Flow {
        match took {
            Some(Took::Flow(flow)) => flow,
            _ => panic!("a flow, and nothing else"),
        }
    }

    /// The reason this source is worth having first: the client says the name
    /// itself, so the flow is named without reading anyone's ClientHello.
    #[tokio::test]
    async fn a_name_comes_through_as_a_name() {
        let (mut client, mut took, _stop) = port_and_client().await;
        no_password(&mut client).await;
        let mut request = vec![VERSION, CONNECT, 0, NAME, 10];
        request.extend_from_slice(b"github.com");
        request.extend_from_slice(&443u16.to_be_bytes());
        assert_eq!(asked_for(&mut client, &request).await, reply(OK));
        let flow = flow_of(took.recv().await);
        assert_eq!(
            flow.to,
            Target::Named {
                host: "github.com".to_owned(),
                port: 443
            }
        );
        assert_eq!(flow.client, client.local_addr().unwrap());
        assert!(flow.who.is_none(), "the program is not known here yet");
    }

    #[tokio::test]
    async fn an_address_comes_through_as_an_address() {
        let (mut client, mut took, _stop) = port_and_client().await;
        no_password(&mut client).await;
        let request = [VERSION, CONNECT, 0, V4, 140, 82, 121, 4, 0x01, 0xbb];
        assert_eq!(asked_for(&mut client, &request).await, reply(OK));
        assert_eq!(
            flow_of(took.recv().await).to,
            Target::Addr("140.82.121.4:443".parse().unwrap())
        );
    }

    #[tokio::test]
    async fn an_ipv6_address_too() {
        let (mut client, mut took, _stop) = port_and_client().await;
        no_password(&mut client).await;
        let mut request = vec![VERSION, CONNECT, 0, V6];
        request.extend_from_slice(&Ipv6Addr::LOCALHOST.octets());
        request.extend_from_slice(&8080u16.to_be_bytes());
        assert_eq!(asked_for(&mut client, &request).await, reply(OK));
        assert_eq!(
            flow_of(took.recv().await).to,
            Target::Addr("[::1]:8080".parse().unwrap())
        );
    }

    /// What is left on the socket after the handshake is the conversation,
    /// whole: a byte counted wrong in the handshake would eat the start of it.
    #[tokio::test]
    async fn the_handshake_eats_nothing_of_what_follows() {
        let (mut client, mut took, _stop) = port_and_client().await;
        no_password(&mut client).await;
        let mut request = vec![VERSION, CONNECT, 0, NAME, 8];
        request.extend_from_slice(b"api.test");
        request.extend_from_slice(&80u16.to_be_bytes());
        assert_eq!(asked_for(&mut client, &request).await, reply(OK));
        client.write_all(b"GET /orders HTTP/1.1\r\n").await.unwrap();
        let mut flow = flow_of(took.recv().await);
        let mut said = [0u8; 22];
        flow.stream.read_exact(&mut said).await.unwrap();
        assert_eq!(&said, b"GET /orders HTTP/1.1\r\n");
    }

    #[tokio::test]
    async fn a_client_with_only_a_password_to_offer_is_turned_away() {
        let (mut client, mut took, _stop) = port_and_client().await;
        // Method 2 is user name and password, which tracewing does not do.
        client.write_all(&[VERSION, 1, 2]).await.unwrap();
        let mut answer = [0u8; 2];
        client.read_exact(&mut answer).await.unwrap();
        assert_eq!(answer, [VERSION, NO_METHOD]);
        assert!(took.try_recv().is_err(), "and no flow");
    }

    #[tokio::test]
    async fn a_command_that_is_not_connect_is_refused_by_its_own_code() {
        let (mut client, mut took, _stop) = port_and_client().await;
        no_password(&mut client).await;
        // 3 is UDP ASSOCIATE.
        let request = [VERSION, 3, 0, V4, 127, 0, 0, 1, 0, 53];
        assert_eq!(asked_for(&mut client, &request).await, reply(NO_COMMAND));
        assert!(took.try_recv().is_err(), "and no flow");
    }

    /// Sending a client to tracewing's own port would be handing traffic back
    /// to itself.
    #[tokio::test]
    async fn it_will_not_send_a_client_to_tracewing() {
        let (mut client, mut took, _stop) = port_and_client().await;
        no_password(&mut client).await;
        let request = [VERSION, CONNECT, 0, V4, 127, 0, 0, 1, 0x23, 0x82];
        assert_eq!(asked_for(&mut client, &request).await, reply(FAILED));
        assert!(took.try_recv().is_err(), "and no flow");
    }
}
