//! UDP that is none of our business, passed to where it was going.
//!
//! The stack turns TCP into flows and answers names itself; every other
//! datagram used to meet a stack with no socket for it, and `smoltcp` answers
//! those with an ICMP port unreachable. Down a tunnel a phone joined that is
//! right: the only datagram worth refusing is QUIC, and refusing it sends an
//! app back to HTTP/2, which is the whole point.
//!
//! On **this machine's own adapter** it is not right at all. A route takes
//! every datagram the machine sends, not only the ones somebody chose to
//! look at, so a refusal there is the user's own VPN, their clock, their
//! voice call and their game all failing while tracewing is on — and none of
//! those is traffic anybody asked us to read. Filtering by program cannot
//! save them either: the refusal happens before any program is known, and a
//! datagram carries no handshake to wait for.
//!
//! So they are passed on: out through a socket of ours, and the answer put
//! back as a packet from the address it was sent to. Nothing is read, kept or
//! decrypted, which is the same bargain as [`crate::sieve`] makes for a
//! connection nobody chose — and it is what makes this mode's promise to
//! leave the rest of the machine alone true of datagrams as well as streams.
//!
//! Two ports do not come through here, and both on purpose: **53**, which is
//! answered by [`crate::dns`] because a dropped query looks like the network
//! has gone rather than like a DNS fault, and **443**, which stays refused so
//! that a client gives up on HTTP/3 and tries TCP, where tracewing can see
//! it. The cost of the second is named rather than hidden: a program nobody
//! chose also loses QUIC, and gets a slower connection instead of a broken
//! one.

use crate::source::{Stop, stopped};
use smoltcp::wire::{IpAddress, IpProtocol, IpRepr, Ipv4Repr, Ipv6Repr, UdpPacket, UdpRepr};
use std::{
    collections::HashMap,
    future::Future,
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{net::UdpSocket, sync::mpsc};

/// What carries UDP, in the `protocol` of an IPv4 header and the
/// `next_header` of an IPv6 one.
const UDP: u8 = 17;

/// A UDP header is always these eight bytes: two ports, a length, a checksum.
const UDP_HEAD: usize = 8;

/// Answered here, not passed on. See [`crate::dns`].
const DNS_PORT: u16 = 53;

/// Refused, not passed on, so that a client falls back to TCP and tracewing
/// can see it.
const QUIC_PORT: u16 = 443;

/// The most one datagram may carry, in or out. A reply longer than this would
/// have to be fragmented to reach the client, and fragmenting it is work
/// nothing yet needs: it is dropped and said instead.
const BODY_MAX: usize = 1472;

/// How many pairs of ends are passed at once. Each holds a socket of the
/// operating system's, so this is the cost of the whole arrangement at worst.
const PASSING_MAX: usize = 512;

/// How long a pair of ends lives with nothing going through it. This is a
/// NAT's own question and the usual answer: long enough for a reply to a
/// query, short enough that a machine's chatter does not fill the table.
const IDLE_FOR: Duration = Duration::from_secs(30);

/// How many datagrams wait to be passed on before the rest are dropped. A
/// full queue drops, which is what a congested network does, and anything
/// that cares about a lost datagram is already built for it.
const QUEUED_MAX: usize = 256;

/// How often the table is looked over for pairs whose time is up.
const SWEEP_EVERY: Duration = Duration::from_secs(5);

/// One datagram, as it arrived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Datagram {
    pub from: SocketAddr,
    pub to: SocketAddr,
    pub body: Vec<u8>,
}

/// How a socket to the far side is made.
///
/// Injected rather than built here, because on Windows it has to be **pinned
/// to the interface the machine's own traffic leaves by** — a socket of ours
/// that followed the ordinary routes would be routed straight back into
/// tracewing's own adapter, and the datagram would arrive here again. A test
/// gives an ordinary socket, which is the whole reason this is a seam: every
/// line below is then exercised on Linux.
///
/// The argument says which family is wanted.
pub type Bind = Arc<dyn Fn(bool) -> Binding + Send + Sync>;

pub type Binding = Pin<Box<dyn Future<Output = io::Result<UdpSocket>> + Send>>;

/// Whether a datagram going to `port` is ours to pass on.
pub fn ours_to_pass(port: u16) -> bool {
    port != DNS_PORT && port != QUIC_PORT
}

/// Whether a datagram going to `port` is refused here so that its client
/// tries TCP instead. One that is gets looked at first: refusing it silently
/// is what made a program insisting on HTTP/3 invisible, and
/// [`crate::quic`] is what makes it a row. Nothing of it is decrypted
/// beyond the hello the client sent in the clear.
pub fn ours_to_refuse(port: u16) -> bool {
    port == QUIC_PORT
}

/// The datagram an IP packet carries, where it carries one this understands.
///
/// `None` for anything that is not UDP, for an IPv4 fragment that is not the
/// first, and for an IPv6 packet with an extension header — the same two
/// packets [`crate::stack::peek`] leaves alone, for the same reason: the UDP
/// header is not where this looks, so guessing would be worse than passing it
/// to the stack to drop.
pub fn datagram(packet: &[u8]) -> Option<Datagram> {
    match *packet.first()? >> 4 {
        4 => {
            let header = usize::from(packet[0] & 0x0f) * 4;
            if header < 20 || packet.len() < header + UDP_HEAD || packet[9] != UDP {
                return None;
            }
            let fragment = u16::from_be_bytes([packet[6], packet[7]]);
            if fragment & 0x1fff != 0 || fragment & 0x2000 != 0 {
                return None;
            }
            let from = Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]);
            let to = Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]);
            ends(&packet[header..], from.into(), to.into())
        }
        6 => {
            const HEAD: usize = 40;
            if packet.len() < HEAD + UDP_HEAD || packet[6] != UDP {
                return None;
            }
            let from = Ipv6Addr::from(<[u8; 16]>::try_from(&packet[8..24]).ok()?);
            let to = Ipv6Addr::from(<[u8; 16]>::try_from(&packet[24..40]).ok()?);
            ends(&packet[HEAD..], from.into(), to.into())
        }
        _ => None,
    }
}

/// The ports and the body, out of a UDP header and what follows it.
fn ends(udp: &[u8], from: IpAddr, to: IpAddr) -> Option<Datagram> {
    let from_port = u16::from_be_bytes([udp[0], udp[1]]);
    let to_port = u16::from_be_bytes([udp[2], udp[3]]);
    // The length in the header counts itself. A header that claims more than
    // the packet holds is one to leave alone rather than to read past.
    let said = usize::from(u16::from_be_bytes([udp[4], udp[5]]));
    if said < UDP_HEAD || said > udp.len() {
        return None;
    }
    Some(Datagram {
        from: SocketAddr::new(from, from_port),
        to: SocketAddr::new(to, to_port),
        body: udp[UDP_HEAD..said].to_vec(),
    })
}

/// The packet for a datagram from `from` to `to`: an IP header, a UDP header
/// and the body, with both checksums worked out.
///
/// This is how an answer gets back to the client. It has to look as though it
/// came from the address the client sent to, because that is the only address
/// the client will accept it from.
pub fn packet_for(from: SocketAddr, to: SocketAddr, body: &[u8]) -> Option<Vec<u8>> {
    if body.len() > BODY_MAX {
        return None;
    }
    let udp = UdpRepr {
        src_port: from.port(),
        dst_port: to.port(),
    };
    let ip = match (from.ip(), to.ip()) {
        (IpAddr::V4(src), IpAddr::V4(dst)) => IpRepr::Ipv4(Ipv4Repr {
            src_addr: src,
            dst_addr: dst,
            next_header: IpProtocol::Udp,
            payload_len: UDP_HEAD + body.len(),
            hop_limit: 64,
        }),
        (IpAddr::V6(src), IpAddr::V6(dst)) => IpRepr::Ipv6(Ipv6Repr {
            src_addr: src,
            dst_addr: dst,
            next_header: IpProtocol::Udp,
            payload_len: UDP_HEAD + body.len(),
            hop_limit: 64,
        }),
        // One end of each family is no datagram anybody sent.
        _ => return None,
    };
    // `buffer_len` counts the payload as well; what is wanted is where the
    // UDP header goes, which is the end of the IP header alone. Getting this
    // wrong put the header ten bytes further on and left a hole of zeroes in
    // front of it -- a packet of exactly the right shape that nothing could
    // read, which is what the round trip below is for.
    let head = ip.header_len();
    let mut packet = vec![0u8; head + UDP_HEAD + body.len()];
    ip.emit(&mut packet[..head], &Default::default());
    udp.emit(
        &mut UdpPacket::new_unchecked(&mut packet[head..]),
        &IpAddress::from(from.ip()),
        &IpAddress::from(to.ip()),
        body.len(),
        |into| into.copy_from_slice(body),
        // Worked out rather than left to the device: this packet goes
        // straight out, and nothing downstream will fill it in.
        &Default::default(),
    );
    Some(packet)
}

/// Starts passing datagrams on, and returns where to put them.
///
/// The table lives in a task of its own: making a socket and sending on it
/// both wait on the operating system, and the stack's loop is what moves
/// every connection's packets. `out` is where answers go, as packets.
pub fn passing(out: mpsc::Sender<Vec<u8>>, bind: Bind, stop: Stop) -> mpsc::Sender<Datagram> {
    waiting(out, bind, stop, IDLE_FOR, SWEEP_EVERY)
}

/// The same, with its own idea of how long a pair lives.
///
/// Thirty seconds and five are the right answers and the wrong ones to wait
/// for in a test, and `tokio`'s clock cannot be wound forward past a
/// `std::time::Instant`. So the two durations are arguments rather than
/// constants read in place, and a test gives milliseconds -- which is what
/// makes the sweep, and the reader stopping with its pair, reachable at all.
fn waiting(
    out: mpsc::Sender<Vec<u8>>,
    bind: Bind,
    stop: Stop,
    idle_for: Duration,
    sweep_every: Duration,
) -> mpsc::Sender<Datagram> {
    let (taken, taking) = mpsc::channel(QUEUED_MAX);
    tokio::spawn(pass(taking, out, bind, stop, idle_for, sweep_every));
    taken
}

/// One pair of ends, and the socket that carries it.
struct Pair {
    socket: Arc<UdpSocket>,
    /// Dropping this is what ends the task reading answers: the pair going
    /// out of the table has to take its reader with it, or a socket nobody
    /// looks at any more would still be listened to.
    _until: tokio::sync::oneshot::Sender<()>,
    last: Instant,
}

async fn pass(
    mut taking: mpsc::Receiver<Datagram>,
    out: mpsc::Sender<Vec<u8>>,
    bind: Bind,
    stop: Stop,
    idle_for: Duration,
    sweep_every: Duration,
) {
    let mut by: HashMap<(SocketAddr, SocketAddr), Pair> = HashMap::new();
    loop {
        tokio::select! {
            taken = taking.recv() => match taken {
                Some(datagram) => send(&mut by, datagram, &out, &bind, idle_for).await,
                // The stack has let go: nothing more will arrive.
                None => break,
            },
            // A pair going out of the table takes its reader with it,
            // because dropping `Pair` drops the sender the reader waits on.
            _ = tokio::time::sleep(sweep_every) => {
                by.retain(|_, pair| pair.last.elapsed() < idle_for);
            }
            _ = stopped(stop.clone()) => break,
        }
    }
}

async fn send(
    by: &mut HashMap<(SocketAddr, SocketAddr), Pair>,
    datagram: Datagram,
    out: &mpsc::Sender<Vec<u8>>,
    bind: &Bind,
    idle_for: Duration,
) {
    let key = (datagram.from, datagram.to);
    if !by.contains_key(&key) {
        if by.len() >= PASSING_MAX {
            // Oldest first, so that a flood of new pairs cannot push out the
            // one a reply is coming back to.
            by.retain(|_, pair| pair.last.elapsed() < idle_for);
            if by.len() >= PASSING_MAX {
                tracing::debug!("no room to pass a datagram to {}", datagram.to);
                return;
            }
        }
        let socket = match bind(datagram.to.is_ipv6()).await {
            Ok(socket) => socket,
            Err(e) => {
                tracing::warn!("no socket to pass a datagram to {}: {e}", datagram.to);
                return;
            }
        };
        // Connected, so that an answer from anywhere else is not taken for
        // one from the server. It is also what lets the reader use `recv`.
        if let Err(e) = socket.connect(datagram.to).await {
            tracing::warn!("cannot reach {} to pass a datagram: {e}", datagram.to);
            return;
        }
        let socket = Arc::new(socket);
        let (until, ended) = tokio::sync::oneshot::channel();
        tokio::spawn(answers(
            Arc::clone(&socket),
            datagram.from,
            datagram.to,
            out.clone(),
            ended,
        ));
        by.insert(
            key,
            Pair {
                socket,
                _until: until,
                last: Instant::now(),
            },
        );
    }
    let pair = by.get_mut(&key).expect("just put there");
    pair.last = Instant::now();
    if let Err(e) = pair.socket.send(&datagram.body).await {
        tracing::debug!("a datagram to {} did not go: {e}", datagram.to);
    }
}

/// What comes back, as packets from the address the client sent to.
async fn answers(
    socket: Arc<UdpSocket>,
    client: SocketAddr,
    server: SocketAddr,
    out: mpsc::Sender<Vec<u8>>,
    mut ended: tokio::sync::oneshot::Receiver<()>,
) {
    let mut buffer = vec![0u8; BODY_MAX];
    loop {
        let read = tokio::select! {
            read = socket.recv(&mut buffer) => read,
            // The pair has gone out of the table, or the stack has stopped.
            _ = &mut ended => break,
        };
        let Ok(len) = read else { break };
        let Some(packet) = packet_for(server, client, &buffer[..len]) else {
            tracing::debug!("an answer from {server} is too long to pass back");
            continue;
        };
        if out.send(packet).await.is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smoltcp::wire::{Ipv4Packet, Ipv6Packet};

    fn client() -> SocketAddr {
        "10.0.0.7:51000".parse().unwrap()
    }

    fn server() -> SocketAddr {
        "203.0.113.9:123".parse().unwrap()
    }

    #[test]
    fn a_packet_and_the_datagram_in_it_are_the_same_thing_both_ways() {
        let packet = packet_for(client(), server(), b"what time is it").expect("a packet");
        let read = datagram(&packet).expect("a datagram");
        assert_eq!(read.from, client());
        assert_eq!(read.to, server());
        assert_eq!(read.body, b"what time is it");
    }

    /// The checksums are worked out here and nothing downstream fills them
    /// in, so a client would drop the packet if they were wrong. `smoltcp`'s
    /// own reader is the judge, since ours does not check them.
    #[test]
    fn the_checksums_are_right_for_both_families() {
        let packet = packet_for(client(), server(), b"hello").expect("a packet");
        let ip = Ipv4Packet::new_checked(&packet[..]).expect("an IPv4 packet");
        assert!(ip.verify_checksum(), "the IPv4 header's checksum");
        let udp = UdpPacket::new_checked(ip.payload()).expect("a UDP packet");
        assert!(
            udp.verify_checksum(&ip.src_addr().into(), &ip.dst_addr().into()),
            "the UDP checksum, which covers the addresses as well as the body"
        );

        let six = packet_for(
            "[2001:db8::7]:51000".parse().unwrap(),
            "[2001:db8::9]:123".parse().unwrap(),
            b"hello",
        )
        .expect("a packet");
        let ip = Ipv6Packet::new_checked(&six[..]).expect("an IPv6 packet");
        let udp = UdpPacket::new_checked(ip.payload()).expect("a UDP packet");
        assert!(
            udp.verify_checksum(&ip.src_addr().into(), &ip.dst_addr().into()),
            "IPv6 has no header checksum of its own, and UDP's is not optional there"
        );
    }

    #[test]
    fn what_is_not_a_datagram_this_understands() {
        // TCP.
        let mut tcp = packet_for(client(), server(), b"x").unwrap();
        tcp[9] = 6;
        assert_eq!(datagram(&tcp), None, "not UDP");
        // A fragment that is not the first.
        let mut later = packet_for(client(), server(), b"x").unwrap();
        later[6] = 0x00;
        later[7] = 0x01;
        assert_eq!(datagram(&later), None, "a fragment carries no header here");
        // The first of several.
        let mut more = packet_for(client(), server(), b"x").unwrap();
        more[6] = 0x20;
        assert_eq!(datagram(&more), None, "more fragments follow");
        // Cut short.
        let whole = packet_for(client(), server(), b"x").unwrap();
        assert_eq!(datagram(&whole[..20]), None, "no room for a UDP header");
        // A length that claims more than the packet holds.
        let mut lying = packet_for(client(), server(), b"x").unwrap();
        let head = 20;
        lying[head + 4] = 0xff;
        lying[head + 5] = 0xff;
        assert_eq!(datagram(&lying), None, "a header that claims too much");
        // Neither family.
        assert_eq!(datagram(&[0x30, 0, 0, 0]), None);
        assert_eq!(datagram(&[]), None);
    }

    #[test]
    fn an_ipv6_packet_with_an_extension_header_is_left_alone() {
        let mut six = packet_for(
            "[2001:db8::7]:51000".parse().unwrap(),
            "[2001:db8::9]:123".parse().unwrap(),
            b"hello",
        )
        .unwrap();
        // Hop-by-hop options, which put the UDP header somewhere else.
        six[6] = 0;
        assert_eq!(datagram(&six), None);
    }

    #[test]
    fn the_two_ports_that_do_not_come_through_here() {
        assert!(!ours_to_pass(53), "a name is answered, not forwarded");
        assert!(!ours_to_pass(443), "QUIC stays refused, so TCP is tried");
        assert!(ours_to_pass(123), "the clock");
        assert!(ours_to_pass(51820), "somebody's own WireGuard");
        assert!(ours_to_pass(3478), "a voice call");
    }

    #[test]
    fn a_body_too_long_to_carry_is_no_packet() {
        assert!(packet_for(client(), server(), &vec![0u8; BODY_MAX]).is_some());
        assert!(packet_for(client(), server(), &vec![0u8; BODY_MAX + 1]).is_none());
    }

    #[test]
    fn one_end_of_each_family_is_no_packet() {
        assert!(
            packet_for(client(), "[2001:db8::9]:123".parse().unwrap(), b"x").is_none(),
            "a datagram has both ends in one family"
        );
    }

    /// The whole of it against a real socket: what the client sent arrives at
    /// the far end, and what the far end answers comes back as a packet from
    /// the address the client sent to.
    #[tokio::test]
    async fn a_datagram_goes_out_and_its_answer_comes_back_as_a_packet() {
        let far = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = far.local_addr().unwrap();
        let (out, mut coming) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let bind: Bind = Arc::new(|_v6| Box::pin(UdpSocket::bind("127.0.0.1:0")));
        let taken = passing(out, bind, stop);

        taken
            .send(Datagram {
                from: client(),
                to: server,
                body: b"what time is it".to_vec(),
            })
            .await
            .unwrap();

        let mut buffer = [0u8; 64];
        let (len, ours) = tokio::time::timeout(Duration::from_secs(5), far.recv_from(&mut buffer))
            .await
            .expect("the datagram reaches the far end")
            .unwrap();
        assert_eq!(&buffer[..len], b"what time is it");

        far.send_to(b"half past four", ours).await.unwrap();
        let packet = tokio::time::timeout(Duration::from_secs(5), coming.recv())
            .await
            .expect("an answer comes back")
            .expect("a packet");
        let read = datagram(&packet).expect("a datagram");
        assert_eq!(
            read.from, server,
            "from the address the client sent to, which is the only one it will take"
        );
        assert_eq!(read.to, client());
        assert_eq!(read.body, b"half past four");
    }

    /// The same client and server twice over use the one socket, so an answer
    /// to either comes back: a pair per datagram would be a new source port
    /// each time and a NAT nothing could reply through.
    #[tokio::test]
    async fn two_datagrams_to_one_server_go_out_the_same_way() {
        let far = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = far.local_addr().unwrap();
        let (out, _coming) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let bind: Bind = Arc::new(|_v6| Box::pin(UdpSocket::bind("127.0.0.1:0")));
        let taken = passing(out, bind, stop);

        let mut ours = Vec::new();
        for body in [b"one".to_vec(), b"two".to_vec()] {
            taken
                .send(Datagram {
                    from: client(),
                    to: server,
                    body,
                })
                .await
                .unwrap();
            let mut buffer = [0u8; 64];
            let (_, from) =
                tokio::time::timeout(Duration::from_secs(5), far.recv_from(&mut buffer))
                    .await
                    .expect("both arrive")
                    .unwrap();
            ours.push(from);
        }
        assert_eq!(ours[0], ours[1], "one pair of ends, one socket");
    }

    /// A pair with nothing going through it goes out of the table, and its
    /// reader goes with it: an answer that comes after that reaches nobody.
    ///
    /// The positive control is in the same test and comes first, because "no
    /// packet arrived" is also what a passer that never worked would say.
    #[tokio::test]
    async fn a_pair_nothing_uses_goes_away_and_takes_its_reader_with_it() {
        let far = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = far.local_addr().unwrap();
        let (out, mut coming) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let bind: Bind = Arc::new(|_v6| Box::pin(UdpSocket::bind("127.0.0.1:0")));
        let taken = waiting(
            out,
            bind,
            stop,
            Duration::from_millis(50),
            Duration::from_millis(10),
        );

        taken
            .send(Datagram {
                from: client(),
                to: server,
                body: b"while it lives".to_vec(),
            })
            .await
            .unwrap();
        let mut buffer = [0u8; 64];
        let (_, ours) = tokio::time::timeout(Duration::from_secs(5), far.recv_from(&mut buffer))
            .await
            .expect("the datagram goes out")
            .unwrap();

        // The control: while the pair is there, an answer comes back.
        far.send_to(b"here", ours).await.unwrap();
        let back = tokio::time::timeout(Duration::from_secs(5), coming.recv())
            .await
            .expect("an answer while the pair lives")
            .expect("a packet");
        assert_eq!(datagram(&back).expect("a datagram").body, b"here");

        // Past its time, with a sweep or two to notice.
        tokio::time::sleep(Duration::from_millis(300)).await;
        far.send_to(b"too late", ours).await.unwrap();
        match tokio::time::timeout(Duration::from_millis(500), coming.recv()).await {
            Err(_) => {}
            Ok(late) => panic!(
                "the pair and its reader should be gone, but {:?} came back",
                late.as_deref().map(datagram)
            ),
        }
    }

    /// A socket that cannot be had is said and nothing else: no panic, and
    /// the next datagram is still passed.
    #[tokio::test]
    async fn a_socket_that_cannot_be_made_loses_one_datagram_and_no_more() {
        let far = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = far.local_addr().unwrap();
        let (out, _coming) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let refuse_once = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let bind: Bind = Arc::new(move |_v6| {
            let first = refuse_once.swap(false, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async move {
                if first {
                    return Err(io::Error::other("not this time"));
                }
                UdpSocket::bind("127.0.0.1:0").await
            })
        });
        let taken = passing(out, bind, stop);

        for body in [b"lost".to_vec(), b"kept".to_vec()] {
            taken
                .send(Datagram {
                    from: client(),
                    to: server,
                    body,
                })
                .await
                .unwrap();
        }
        let mut buffer = [0u8; 64];
        let (len, _) = tokio::time::timeout(Duration::from_secs(5), far.recv_from(&mut buffer))
            .await
            .expect("the second one still goes")
            .unwrap();
        assert_eq!(
            &buffer[..len],
            b"kept",
            "the first was lost with its socket, and only the first"
        );
    }
}
