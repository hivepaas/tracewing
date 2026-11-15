//! A WireGuard endpoint: a phone joins, and its packets come out as packets.
//!
//! This is the first **device** for [`crate::stack`]. The stack turns packets
//! into flows and needs somebody to hand it packets; every other way in that
//! tracewing has is handed a stream somebody else built. A phone is the case
//! none of them reach: it has to be told the proxy by hand, and an app that
//! ignores the Wi-Fi proxy cannot be caught at all.
//!
//! What makes this the first one worth building is what it does not need. No
//! privilege on this machine, no driver, nothing installed on the phone
//! beyond the WireGuard app it can get from either store. The phone is told
//! one address and one key, and from then on its traffic arrives here as
//! UDP.
//!
//! What is here and what is not:
//!
//! - The protocol is [`boringtun`]'s, not ours. It holds the Noise
//!   handshake, the session keys, the rekeying timers, the replay window and
//!   the cookie replies that answer a flood. Writing that by hand is a day's
//!   work and a year's worry: every one of those fails **silently**, and a
//!   handshake that is wrong in a way tests cannot see still completes.
//! - The socket, the peer's address, and the bridge to the stack are ours.
//!   `boringtun`'s library is deliberately without "the network or tunnel
//!   stacks", which is the division this wanted anyway: what comes out of
//!   [`Tunn::decapsulate`] is an IP packet, and an IP packet is what
//!   [`Wire`](crate::stack::Wire) takes.
//!
//! **One peer for now.** A second would need the receiver index read out of
//! each datagram to tell whose it is — `Tunn::parse_incoming_packet` is
//! there for it — and a peer's keys have to be configured and shown as a QR
//! code before more than one is any use. One is what a phone on a desk
//! needs, and the shape here does not fight a second.
//!
//! The peer's address is learnt from whatever datagram last authenticated,
//! which is how WireGuard roams: a phone that moves from Wi-Fi to mobile
//! keeps its session and arrives from somewhere else, and answering to where
//! it used to be would lose it.
//!
//! See [capture-modes.md](../../../docs/design/capture-modes.md).

use crate::{
    source::{Found, Source, Stop, stopped},
    stack::{Stack, Wire},
};
use anyhow::{Context, Result};
use boringtun::{
    noise::{Tunn, TunnResult, errors::WireGuardError, rate_limiter::RateLimiter},
    x25519::{PublicKey, StaticSecret},
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::UdpSocket, sync::mpsc};

/// The most a datagram off the wire may be. A WireGuard datagram is the
/// packet inside it plus 32 bytes of header and tag, so this holds any
/// packet an ordinary link carries.
const DATAGRAM_MAX: usize = 2048;

/// How often the protocol's own timers are run: rekeying, keepalives, and
/// the reset of the flood counter that decides when cookies are demanded.
/// `boringtun`'s own device uses a quarter of a second, and the timers are
/// rounded to the second inside, so this is frequent enough to be exact and
/// rare enough to cost nothing.
const TICK: Duration = Duration::from_millis(250);

/// How many handshakes a second are answered before cookies are demanded
/// instead. The limiter is `boringtun`'s; this is the number it is given.
const HANDSHAKES_PER_SECOND: u64 = 100;

/// Who may join, and as what.
pub struct Peer {
    /// This end's private key. The public half of it is what the phone is
    /// told to expect.
    pub ours: StaticSecret,
    /// The phone's public key.
    pub theirs: PublicKey,
    /// An extra symmetric key both ends hold, if the user set one. Not
    /// needed, and a second lock where it is used.
    pub preshared: Option<[u8; 32]>,
    /// How often the phone should send a packet to keep a NAT open. The
    /// phone decides this itself in its own configuration; this is what we
    /// would do, and `None` is right for a server that only answers.
    pub keepalive: Option<u16>,
}

/// The public key a phone has to be given to join. Written out as the
/// standard base64 of the 32 bytes, which is what a WireGuard
/// configuration file and a QR code both carry.
pub fn public_of(ours: &StaticSecret) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(PublicKey::from(ours).as_bytes())
}

/// A key of our own, for a tracewing that has never had one. The bytes come
/// from the operating system and nowhere else: this is the secret the whole
/// endpoint rests on, so there is no fallback to anything weaker — a machine
/// that cannot give 32 random bytes does not get a WireGuard port.
pub fn new_key() -> Result<StaticSecret> {
    let mut bytes = [0u8; 32];
    // `getrandom`'s error is not one `anyhow` can take context on, so it is
    // said here instead.
    getrandom::fill(&mut bytes)
        .map_err(|e| anyhow::anyhow!("ask the operating system for a WireGuard key: {e}"))?;
    Ok(StaticSecret::from(bytes))
}

/// Reads a key as a WireGuard configuration writes one: base64 of 32 bytes.
pub fn key_from(text: &str) -> Result<[u8; 32]> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .context("a WireGuard key is 32 bytes written in base64")?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("a WireGuard key is 32 bytes, this is not"))?;
    Ok(bytes)
}

/// A way in: the endpoint, and the stack that turns what it takes into
/// flows. One type, because the two are no use apart — packets with nothing
/// to read them, or a stack with nothing to feed it.
///
/// The socket is bound by the caller, before the proxy's runtime exists, so
/// that a port already in use is said in the window rather than from inside
/// a thread. That is how [`crate::socks`] is given its listener, for the
/// same reason.
pub struct Endpoint {
    bound: std::net::UdpSocket,
    peer: Peer,
}

impl Endpoint {
    pub fn new(bound: std::net::UdpSocket, peer: Peer) -> Self {
        Self { bound, peer }
    }

    /// Where it listens, as the socket was bound.
    pub fn at(&self) -> Result<SocketAddr> {
        Ok(self.bound.local_addr()?)
    }
}

impl Source for Endpoint {
    fn name(&self) -> String {
        "wireguard".to_owned()
    }

    // `own_ports` stays empty: a phone never connects to a port of
    // tracewing's, it talks to the server it was going to, so there is no
    // socket of ours for `crate::apps` to find a program by. The stack says
    // the same, for the same reason.

    fn start(self: Box<Self>, proxy: SocketAddr, found: Found, stop: Stop) -> Result<()> {
        let Self { bound, peer } = *self;
        // Forgetting this blocks a worker on a read, which is a hang rather
        // than an error — the same trap the SOCKS5 listener has.
        bound.set_nonblocking(true)?;
        let socket = UdpSocket::from_std(bound)?;
        let (wire, into_stack, from_stack) = Wire::channels();
        // The stack first: it is what `found` hears from, and a packet that
        // arrived before it was listening would have nowhere to go.
        Box::new(Stack::new(wire)).start(proxy, found, stop.clone())?;
        tokio::spawn(run(socket, peer, into_stack, from_stack, stop));
        Ok(())
    }
}

/// Serves `peer` on `socket` until `stop`, handing the packets it takes to
/// `into_stack` and sending back whatever `from_stack` answers.
///
/// Both ends of the bridge are the channels [`Wire::channels`] hands out, so
/// what this serves is whatever the stack makes of the packets: flows, to
/// the proxy, as though they had come from a client that asked.
///
/// [`Wire::channels`]: crate::stack::Wire::channels
pub async fn run(
    socket: UdpSocket,
    peer: Peer,
    into_stack: mpsc::Sender<Vec<u8>>,
    mut from_stack: mpsc::Receiver<Vec<u8>>,
    stop: Stop,
) {
    // The limiter is given our own public key because a cookie is a MAC
    // under it: a flood is answered without the handshake being done, which
    // is the whole point of them.
    let limiter = Arc::new(RateLimiter::new(
        &PublicKey::from(&peer.ours),
        HANDSHAKES_PER_SECOND,
    ));
    let mut tunn = Tunn::new(
        peer.ours,
        peer.theirs,
        peer.preshared,
        peer.keepalive,
        // One peer, so one index. A second peer would need its own, and the
        // index is what tells their datagrams apart.
        0,
        Some(Arc::clone(&limiter)),
    );

    // Where the phone was last heard from. Nothing can be sent before it has
    // said something: a server has no idea where its peer is until it does.
    let mut phone: Option<SocketAddr> = None;
    let mut datagram = vec![0u8; DATAGRAM_MAX];
    let mut out = vec![0u8; DATAGRAM_MAX];
    let mut tick = tokio::time::interval(TICK);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            got = socket.recv_from(&mut datagram) => {
                let (len, from) = match got {
                    Ok(got) => got,
                    Err(e) => {
                        tracing::warn!("reading from the WireGuard port: {e}");
                        continue;
                    }
                };
                match tunn.decapsulate(Some(from.ip()), &datagram[..len], &mut out) {
                    TunnResult::Done => {}
                    TunnResult::Err(e) => said(e),
                    TunnResult::WriteToNetwork(answer) => {
                        // A handshake answer or a cookie. Either way the
                        // peer is where this datagram came from.
                        phone = Some(from);
                        send(&socket, answer, from).await;
                        // Whatever was queued while there was no session:
                        // the call with no datagram is how `boringtun` is
                        // asked for it, one packet at a time until it has
                        // none left.
                        loop {
                            let mut more = vec![0u8; DATAGRAM_MAX];
                            match tunn.decapsulate(None, &[], &mut more) {
                                TunnResult::WriteToNetwork(queued) => {
                                    send(&socket, queued, from).await;
                                }
                                _ => break,
                            }
                        }
                    }
                    TunnResult::WriteToTunnelV4(packet, _) | TunnResult::WriteToTunnelV6(packet, _) => {
                        // An IP packet the phone sent, which is what the
                        // stack is waiting for. Its address is learnt here
                        // too: this datagram authenticated.
                        phone = Some(from);
                        if into_stack.send(packet.to_vec()).await.is_err() {
                            return; // The stack has gone.
                        }
                    }
                }
            }
            packet = from_stack.recv() => {
                let Some(packet) = packet else {
                    return; // The stack has gone.
                };
                let Some(to) = phone else {
                    // Nothing has joined, so there is nobody to answer. The
                    // stack can only be answering something, so this would
                    // be a packet for a session that has gone.
                    continue;
                };
                match tunn.encapsulate(&packet, &mut out) {
                    TunnResult::WriteToNetwork(datagram) => send(&socket, datagram, to).await,
                    TunnResult::Err(e) => said(e),
                    _ => {}
                }
            }
            _ = tick.tick() => {
                match tunn.update_timers(&mut out) {
                    TunnResult::WriteToNetwork(datagram) => {
                        if let Some(to) = phone {
                            send(&socket, datagram, to).await;
                        }
                    }
                    TunnResult::Err(WireGuardError::ConnectionExpired) => {
                        // The session is over and the phone has to start
                        // again, which it will when it next has traffic.
                        // Not an error of ours, and not worth a line each
                        // time it happens.
                        phone = None;
                    }
                    TunnResult::Err(e) => said(e),
                    _ => {}
                }
            }
            _ = stopped(stop.clone()) => return,
        }
    }
}

async fn send(socket: &UdpSocket, datagram: &[u8], to: SocketAddr) {
    if let Err(e) = socket.send_to(datagram, to).await {
        tracing::warn!("writing to {to}: {e}");
    }
}

/// A protocol error is the other end's business, not a fault here: a
/// datagram that fails its MAC is what a scan of the port looks like, and
/// saying so at a level anybody watches would make a log out of noise.
fn said(e: WireGuardError) {
    tracing::debug!("a WireGuard datagram came to nothing: {e:?}");
}

/// The address and key a phone needs, as a WireGuard configuration file.
/// Written here rather than in the window because it is the protocol's
/// business what the file says, and because a test can read it.
pub fn config_for(ours: &StaticSecret, phone_key: &StaticSecret, at: SocketAddr) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    format!(
        "[Interface]\n\
         PrivateKey = {}\n\
         Address = 10.78.0.2/32\n\
         DNS = 10.78.0.1\n\
         \n\
         [Peer]\n\
         PublicKey = {}\n\
         Endpoint = {at}\n\
         AllowedIPs = 0.0.0.0/0, ::/0\n\
         PersistentKeepalive = 25\n",
        b64.encode(phone_key.as_bytes()),
        b64.encode(PublicKey::from(ours).as_bytes()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::watch;

    /// How long any wait in these tests is given. A test that waits with no
    /// deadline hangs the suite instead of failing it, which is worth less
    /// than no test: found by ablation, when breaking the way in made this
    /// file hang rather than go red.
    const SOON: Duration = Duration::from_secs(5);

    /// `f`, or a failure saying what did not happen in time.
    async fn soon<T>(what: &str, f: impl std::future::Future<Output = T>) -> T {
        match tokio::time::timeout(SOON, f).await {
            Ok(got) => got,
            Err(_) => panic!("{what} did not happen within {SOON:?}"),
        }
    }

    /// A phone, played by `boringtun` itself. That is the point of the test:
    /// the other end of the handshake is an implementation that knows
    /// nothing of this module, so a protocol mistake here cannot be hidden
    /// by the same mistake on both sides — which is what a hand-written peer
    /// would have risked.
    struct Phone {
        tunn: Tunn,
        socket: UdpSocket,
        server: SocketAddr,
    }

    impl Phone {
        async fn new(server: SocketAddr, ours: &StaticSecret, theirs: PublicKey) -> Self {
            let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let tunn = Tunn::new(ours.clone(), theirs, None, Some(25), 1, None);
            Self {
                tunn,
                socket,
                server,
            }
        }

        /// Sends `packet` through the tunnel, doing the handshake first if
        /// there is not one yet, and answering whatever comes back until the
        /// packet has gone.
        async fn send(&mut self, packet: &[u8]) {
            let mut out = vec![0u8; DATAGRAM_MAX];
            match self.tunn.encapsulate(packet, &mut out) {
                TunnResult::WriteToNetwork(datagram) => {
                    self.socket.send_to(datagram, self.server).await.unwrap();
                }
                other => panic!("the phone had nothing to send: {}", named(&other)),
            }
        }

        /// Reads one datagram, hands it to the tunnel, and says what it was.
        /// Answers the handshake automatically, which is what a phone does.
        async fn turn(&mut self) -> Turn {
            let mut datagram = vec![0u8; DATAGRAM_MAX];
            let (len, _) = soon("a datagram back", self.socket.recv_from(&mut datagram))
                .await
                .unwrap();
            let mut out = vec![0u8; DATAGRAM_MAX];
            match self
                .tunn
                .decapsulate(Some(self.server.ip()), &datagram[..len], &mut out)
            {
                TunnResult::WriteToNetwork(answer) => {
                    self.socket.send_to(answer, self.server).await.unwrap();
                    // Drain whatever was queued behind the handshake.
                    loop {
                        let mut more = vec![0u8; DATAGRAM_MAX];
                        match self.tunn.decapsulate(None, &[], &mut more) {
                            TunnResult::WriteToNetwork(queued) => {
                                self.socket.send_to(queued, self.server).await.unwrap();
                            }
                            _ => break,
                        }
                    }
                    Turn::Protocol
                }
                TunnResult::WriteToTunnelV4(packet, _) | TunnResult::WriteToTunnelV6(packet, _) => {
                    Turn::Packet(packet.to_vec())
                }
                TunnResult::Done => Turn::Nothing,
                TunnResult::Err(e) => panic!("the phone refused a datagram: {e:?}"),
            }
        }

        /// Turns until a packet comes through the tunnel, or gives up.
        async fn packet(&mut self) -> Vec<u8> {
            let hunt = async {
                loop {
                    if let Turn::Packet(packet) = self.turn().await {
                        return packet;
                    }
                }
            };
            soon("a packet through the tunnel", hunt).await
        }
    }

    #[derive(Debug)]
    enum Turn {
        Protocol,
        Packet(Vec<u8>),
        Nothing,
    }

    fn named(result: &TunnResult<'_>) -> &'static str {
        match result {
            TunnResult::Done => "done",
            TunnResult::Err(_) => "an error",
            TunnResult::WriteToNetwork(_) => "a datagram",
            TunnResult::WriteToTunnelV4(..) | TunnResult::WriteToTunnelV6(..) => "a packet",
        }
    }

    /// An IPv4 TCP `SYN`, as a phone opening a connection sends one. Built
    /// here by hand because what matters is that the bytes arrive
    /// unchanged, not what they mean — the stack is what reads them, and it
    /// has its own tests.
    fn syn() -> Vec<u8> {
        let mut packet = vec![0u8; 40];
        packet[0] = 0x45; // IPv4, five words of header
        packet[2..4].copy_from_slice(&40u16.to_be_bytes());
        packet[8] = 64; // hop limit
        packet[9] = 6; // TCP
        packet[12..16].copy_from_slice(&[10, 78, 0, 2]);
        packet[16..20].copy_from_slice(&[140, 82, 121, 4]);
        packet[20..22].copy_from_slice(&51000u16.to_be_bytes());
        packet[22..24].copy_from_slice(&443u16.to_be_bytes());
        packet[32] = 5 << 4; // five words of TCP header
        packet[33] = 0x02; // SYN
        packet
    }

    struct Running {
        at: SocketAddr,
        ours: StaticSecret,
        phone_key: StaticSecret,
        into_stack: mpsc::Receiver<Vec<u8>>,
        from_stack: mpsc::Sender<Vec<u8>>,
        _stop: watch::Sender<bool>,
    }

    async fn start() -> Running {
        let ours = new_key().unwrap();
        let phone_key = new_key().unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let at = socket.local_addr().unwrap();
        let (into_tx, into_stack) = mpsc::channel(16);
        let (from_stack, from_rx) = mpsc::channel(16);
        let (stop_tx, stop) = watch::channel(false);
        tokio::spawn(run(
            socket,
            Peer {
                ours: ours.clone(),
                theirs: PublicKey::from(&phone_key),
                preshared: None,
                keepalive: None,
            },
            into_tx,
            from_rx,
            stop,
        ));
        Running {
            at,
            ours,
            phone_key,
            into_stack,
            from_stack,
            _stop: stop_tx,
        }
    }

    /// The whole of it: a phone that knows only an address and a key gets a
    /// session, and the packet it sends comes out as the packet it sent.
    #[tokio::test]
    async fn a_phone_that_joins_has_its_packets_come_out_as_packets() {
        let mut running = start().await;
        let mut phone = Phone::new(
            running.at,
            &running.phone_key,
            PublicKey::from(&running.ours),
        )
        .await;

        // The first thing a phone sends is a handshake, because it has no
        // session yet; the packet goes in the queue behind it.
        phone.send(&syn()).await;
        phone.turn().await; // the handshake's answer
        phone.send(&syn()).await;

        let got = soon("the packet reaches the stack", running.into_stack.recv())
            .await
            .expect("the endpoint is still there");
        assert_eq!(got, syn(), "the packet arrives as it was sent");
    }

    /// And back: what the stack writes reaches the phone, through the
    /// session the phone opened.
    #[tokio::test]
    async fn what_the_stack_answers_reaches_the_phone() {
        let mut running = start().await;
        let mut phone = Phone::new(
            running.at,
            &running.phone_key,
            PublicKey::from(&running.ours),
        )
        .await;
        phone.send(&syn()).await;
        phone.turn().await;
        phone.send(&syn()).await;
        soon("the way in to work", running.into_stack.recv())
            .await
            .expect("the endpoint is still there");

        // A SYN-ACK, as the stack answers one. The addresses are the other
        // way round, which is the only part that has to be true here.
        let mut answer = syn();
        answer[12..16].copy_from_slice(&[140, 82, 121, 4]);
        answer[16..20].copy_from_slice(&[10, 78, 0, 2]);
        answer[33] = 0x12; // SYN | ACK
        running.from_stack.send(answer.clone()).await.unwrap();

        assert_eq!(
            phone.packet().await,
            answer,
            "the answer arrives at the phone as it was written"
        );
    }

    /// Nothing is sent to a peer that has never been heard from. A server
    /// does not know where its phone is until the phone says so, and a
    /// packet from the stack before then has nowhere to go.
    #[tokio::test]
    async fn an_answer_with_nobody_to_send_it_to_is_dropped_and_not_fatal() {
        let running = start().await;
        running.from_stack.send(syn()).await.unwrap();

        // Still serving: a phone that joins afterwards is served as usual.
        let mut phone = Phone::new(
            running.at,
            &running.phone_key,
            PublicKey::from(&running.ours),
        )
        .await;
        phone.send(&syn()).await;
        assert!(
            matches!(phone.turn().await, Turn::Protocol),
            "the endpoint answered the handshake after dropping that packet"
        );
    }

    /// A datagram from somewhere else is not a session. A port on the
    /// internet is scanned, and what a scan gets must be nothing at all —
    /// not an answer, and not a dead endpoint.
    #[tokio::test]
    async fn rubbish_on_the_port_is_ignored_and_the_endpoint_lives() {
        let running = start().await;
        let scanner = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        for rubbish in [&b"hello?"[..], &[0u8; 148][..], &[1u8; 32][..]] {
            scanner.send_to(rubbish, running.at).await.unwrap();
        }
        // Nothing comes back, and nothing reaches the stack.
        let mut back = [0u8; DATAGRAM_MAX];
        let heard =
            tokio::time::timeout(Duration::from_millis(300), scanner.recv_from(&mut back)).await;
        assert!(heard.is_err(), "a scan is answered with silence");

        let mut phone = Phone::new(
            running.at,
            &running.phone_key,
            PublicKey::from(&running.ours),
        )
        .await;
        phone.send(&syn()).await;
        assert!(matches!(phone.turn().await, Turn::Protocol));
    }

    /// The wrong key is not let in, which is the one thing this must never
    /// get wrong: the port is open to the internet.
    #[tokio::test]
    async fn a_phone_with_the_wrong_key_gets_no_session() {
        let running = start().await;
        let stranger = new_key().unwrap();
        let mut phone = Phone::new(running.at, &stranger, PublicKey::from(&running.ours)).await;
        phone.send(&syn()).await;

        let mut back = [0u8; DATAGRAM_MAX];
        let heard =
            tokio::time::timeout(Duration::from_secs(1), phone.socket.recv_from(&mut back)).await;
        assert!(
            heard.is_err(),
            "a handshake signed by a key we do not know is answered with nothing"
        );
    }

    #[test]
    fn a_key_is_written_and_read_as_a_wireguard_configuration_does() {
        let key = new_key().unwrap();
        let text = public_of(&key);
        assert_eq!(text.len(), 44, "32 bytes in base64, with its padding");
        assert_eq!(
            key_from(&text).unwrap(),
            *PublicKey::from(&key).as_bytes(),
            "what is written is what is read"
        );
        assert!(key_from("not a key").is_err());
        assert!(
            key_from(&format!("{text}\n")).is_ok(),
            "a key copied out of a file brings its newline"
        );
    }

    #[test]
    fn the_configuration_names_this_end_and_keeps_the_phones_key_secret_to_it() {
        let ours = new_key().unwrap();
        let phone = new_key().unwrap();
        let at: SocketAddr = "192.168.1.13:51820".parse().unwrap();
        let config = config_for(&ours, &phone, at);
        assert!(config.contains(&format!("PublicKey = {}", public_of(&ours))));
        assert!(config.contains("Endpoint = 192.168.1.13:51820"));
        assert!(
            config.contains("AllowedIPs = 0.0.0.0/0, ::/0"),
            "everything goes through it, which is the point"
        );
        assert!(
            !config.contains(&public_of(&phone)),
            "the phone's own public key is not in its own file: it has the private one"
        );
    }

    /// The whole way in, end to end: a phone joins the tunnel, opens a TCP
    /// connection inside it to a server it was going to, and the proxy is
    /// handed a flow. Nothing here is a stand-in — a real handshake, a real
    /// TCP handshake inside it, and `Found` is the proxy's own channel.
    ///
    /// The TCP client is [`crate::stack`]'s own test client, reached across
    /// modules on purpose: a second hand-written one would be a second
    /// chance to get the sequence numbers wrong, which is a mistake that
    /// already cost an afternoon once.
    #[tokio::test]
    async fn a_phone_in_the_tunnel_becomes_a_flow_the_proxy_is_given() {
        use crate::source::Took;
        use crate::stack::Client;
        use smoltcp::wire::TcpControl;

        let ours = new_key().unwrap();
        let phone_key = new_key().unwrap();
        let bound = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let at = bound.local_addr().unwrap();
        let (found, mut took) = crate::source::Found::channel();
        let (_stop_tx, stop) = watch::channel(false);

        let endpoint = Endpoint::new(
            bound,
            Peer {
                ours: ours.clone(),
                theirs: PublicKey::from(&phone_key),
                preshared: None,
                keepalive: None,
            },
        );
        assert_eq!(endpoint.at().unwrap(), at, "it says where it listens");
        Box::new(endpoint)
            .start("127.0.0.1:9090".parse().unwrap(), found, stop)
            .unwrap();

        let mut phone = Phone::new(at, &phone_key, PublicKey::from(&ours)).await;
        let mut tcp = Client::new("10.78.0.2:51000", "140.82.121.4:443");

        // The phone's first datagram is a handshake, so the SYN goes in the
        // queue behind it and is sent again once there is a session.
        let syn = tcp.send(TcpControl::Syn, &[]);
        phone.send(&syn).await;
        phone.turn().await;
        phone.send(&syn).await;

        // The stack answers as the server the phone was going to, and that
        // answer comes back through the tunnel.
        let answer = phone.packet().await;
        let (control, _) = tcp.heard(&answer);
        assert_eq!(control, TcpControl::Syn, "a SYN-ACK, as GitHub would send");

        // The phone finishes the handshake, which is when the flow is handed
        // over: the stack waits until it can write before giving it away.
        phone.send(&tcp.send(TcpControl::None, &[])).await;

        let flow = match soon("a flow", took.recv()).await {
            Some(Took::Flow(flow)) => flow,
            Some(Took::Port(port)) => panic!("a flow, not the listener {}", port.name),
            None => panic!("the endpoint handed nothing over and then went"),
        };
        assert_eq!(flow.client, tcp.from, "the phone, as the packet said");
        assert_eq!(
            flow.to,
            crate::source::Target::Addr(tcp.to),
            "where it was going, which is all a packet carries"
        );
        assert!(flow.who.is_none(), "a packet says nothing about a program");
    }
}
