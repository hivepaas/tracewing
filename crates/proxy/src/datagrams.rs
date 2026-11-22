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
//! **53** does not come through here at all: it is answered by
//! [`crate::dns`], because a dropped query looks like the network has gone
//! rather than like a DNS fault.
//!
//! **443** has three answers, and which one a datagram gets depends on the
//! program that sent it.
//!
//! - Passed on, as everything else is, when it is **nobody's business of
//!   ours**: that is this file's bargain and QUIC is no exception to it.
//!   This is what a tunnel a phone joined does with every datagram.
//! - Sent to **tracewing's own HTTP/3 door** when the program behind it is
//!   one somebody chose and HTTP/3 capture is on. The door terminates QUIC,
//!   reads the HTTP/3 in it, and [`crate::quic_upstream`] asks the real
//!   server over HTTP/3 of its own. What the door needs to know about the
//!   client -- who it is, which program, where it was really going -- goes
//!   in [`crate::handover::Handed`] under the address the datagrams reach
//!   the door from, which is the same table a taken connection's facts go
//!   in and the same [`crate::capture`] reads them from. See [`Catching`].
//! - **Refused**, as it always was, when nobody chose the program: the
//!   datagram goes back to the stack, which has no socket for port 443 and
//!   answers an ICMP port unreachable, and the client gives up on HTTP/3 and
//!   tries TCP where tracewing can see it. Refusing is still the answer when
//!   HTTP/3 capture is off, which is how it ships.
//!
//! The cost of the third is named rather than hidden: a program nobody chose
//! loses QUIC and gets a slower connection instead of a broken one. The cost
//! of telling the second from the third is a wait -- the operating system is
//! asked which program holds the socket, which is one look at a table of the
//! kernel's and usually a millisecond or two -- paid once per pair of ends
//! while capture is on, and paid by every program's QUIC, not only by the
//! chosen ones. The datagrams that arrive during it are held rather than
//! dropped, because a QUIC client that loses its first flight waits out a
//! timeout of its own before sending it again.

use crate::{
    handover::{Handed, Keeping, Known},
    source::{Stop, stopped},
};
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
pub(crate) const UDP: u8 = 17;

/// A UDP header is always these eight bytes: two ports, a length, a checksum.
const UDP_HEAD: usize = 8;

/// Answered here, not passed on. See [`crate::dns`].
const DNS_PORT: u16 = 53;

/// Where QUIC is, and so where HTTP/3 is: either refused so that a client
/// falls back to TCP, or captured. See the module's own documentation.
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

/// How many of one client's datagrams are held while the operating system is
/// asked which program sent them.
///
/// A QUIC client's first flight is one or two datagrams, and this is for
/// those: holding them is what keeps a captured connection from starting
/// with a retransmission timeout. It is not a queue for a flood -- a client
/// that keeps sending while the answer is still coming has lost nothing that
/// matters, since none of it was going anywhere until the answer arrives.
const HELD_MAX: usize = 4;

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

/// Whether a datagram going to `port` is one tracewing may read instead of
/// refusing, when the program behind it is one somebody chose. See
/// [`Catching`].
pub fn ours_to_catch(port: u16) -> bool {
    port == QUIC_PORT
}

/// What capturing a program's HTTP/3 takes from outside this file.
///
/// Injected for the same reasons [`Bind`] is, and one more. The answer needs
/// the window's list of chosen programs, the live settings and a way to ask
/// the operating system which program holds a socket, none of which a test
/// here has or wants; and **the door is inside the proxy while this is
/// outside it**, so where it listens is something this is told rather than
/// something it knows.
pub struct Catching {
    /// Whether to look at UDP 443 at all, asked of each datagram and
    /// answered without waiting on anything.
    ///
    /// **It is why the setting being off costs nothing.** Off, a datagram to
    /// port 443 is refused by the stack exactly as it was before any of
    /// this: no entry in the table below, no task spawned, no hop through
    /// this file. That matters because QUIC is most of the UDP a machine
    /// sends, and a table filling with refusals would crowd out the
    /// datagrams that have to be passed on -- somebody's VPN, clock and
    /// voice call.
    pub on: On,
    pub whether: Whether,
    /// Where what the door needs to know about each captured client goes.
    /// The same table a taken connection's facts go in: see
    /// [`crate::handover`].
    pub handed: Arc<Handed>,
}

pub type On = Arc<dyn Fn() -> bool + Send + Sync>;

/// Whether this datagram's client is one whose HTTP/3 tracewing reads, and
/// if it is, everything needed to carry it.
///
/// The answer comes later, as [`crate::sieve::Keeps`]'s does and for the
/// same reason: which program holds a socket is a question for a thread of
/// its own, and nothing here may block while it is asked.
pub type Whether = Arc<dyn Fn(Datagram) -> Asking + Send + Sync>;

pub type Asking = Pin<Box<dyn Future<Output = Option<Caught>> + Send>>;

/// A client whose HTTP/3 is to be read: where its datagrams really go, and
/// what the door is to know about it.
pub struct Caught {
    /// tracewing's own HTTP/3 door, on this machine.
    pub door: SocketAddr,
    /// Who the client is, which program is behind it, and where it was
    /// really going -- which the door cannot see, because every captured
    /// client reaches it over the loopback from a socket of ours.
    pub known: Known,
}

/// Whether a datagram is for the link it was sent on rather than for
/// anywhere beyond it: a multicast group, or a broadcast. Those are dropped,
/// not passed on.
///
/// **Passed on, they came straight back, and without end.** The machine
/// announces itself on every link it has, ours included -- LLMNR, mDNS,
/// SSDP -- and the link behind the adapter has nobody on it but us. A socket
/// of ours sending one on was pinned for unicast alone (`IP_UNICAST_IF`), so
/// the group's datagram left by the adapter again, from that socket's new
/// port, which made a new pair of ends to pass, which made another socket.
/// Measured in the VM by `local_live`'s S8 tests: 512 pairs, the table's
/// whole room, 70 ms after the adapter came up, and from then on every new
/// pair was refused room -- a chosen program's QUIC among them. The machine
/// sends the same announcements on its real interfaces itself, so nothing is
/// lost by leaving ours unanswered.
fn for_this_link(to: IpAddr) -> bool {
    match to {
        IpAddr::V4(v4) => v4.is_multicast() || v4.is_broadcast(),
        IpAddr::V6(v6) => v6.is_multicast(),
    }
}

/// Whether a datagram going to `port` is refused so that its client tries
/// TCP instead. One that is gets looked at first: refusing it silently is
/// what made a program insisting on HTTP/3 invisible, and [`crate::quic`] is
/// what makes it a row. Nothing of it is decrypted beyond the hello the
/// client sent in the clear.
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
///
/// `catching`, where it is given, is what makes UDP 443 something other than
/// refused; see [`Catching`]. `again` is where a datagram nobody chose goes
/// back to, to be refused by the stack as it was before any of this -- the
/// stack holds the one socket-less port 443 whose ICMP is what sends a
/// client back to TCP, and refusing in two places would be two answers to
/// get out of step.
pub fn passing(
    out: mpsc::Sender<Vec<u8>>,
    bind: Bind,
    catching: Option<Catching>,
    again: mpsc::Sender<Vec<u8>>,
    stop: Stop,
) -> mpsc::Sender<Datagram> {
    waiting(out, bind, catching, again, stop, IDLE_FOR, SWEEP_EVERY)
}

/// The same, with its own idea of how long a pair lives.
///
/// Thirty seconds and five are the right answers and the wrong ones to wait
/// for in a test, and `tokio`'s clock cannot be wound forward past a
/// `std::time::Instant`. So the two durations are arguments rather than
/// constants read in place, and a test gives milliseconds -- which is what
/// makes the sweep, and the reader stopping with its pair, reachable at all.
#[allow(clippy::too_many_arguments)]
fn waiting(
    out: mpsc::Sender<Vec<u8>>,
    bind: Bind,
    catching: Option<Catching>,
    again: mpsc::Sender<Vec<u8>>,
    stop: Stop,
    idle_for: Duration,
    sweep_every: Duration,
) -> mpsc::Sender<Datagram> {
    let (taken, taking) = mpsc::channel(QUEUED_MAX);
    tokio::spawn(pass(
        taking,
        out,
        bind,
        catching,
        again,
        stop,
        idle_for,
        sweep_every,
    ));
    taken
}

/// One pair of ends, by the two addresses the client's datagram carried.
type Ends = (SocketAddr, SocketAddr);

/// What is being done with one pair of ends, and when it last had anything
/// to do.
struct Doing {
    last: Instant,
    what: What,
}

impl Doing {
    fn now(what: What) -> Self {
        Self {
            last: Instant::now(),
            what,
        }
    }
}

enum What {
    /// A socket of ours is carrying it: to where it was going, or -- for a
    /// client whose HTTP/3 is being read -- to tracewing's own door.
    Through(Pair),
    /// The operating system is being asked which program sent it, and what
    /// has arrived meanwhile is waiting for the answer. See [`HELD_MAX`].
    Asking(Vec<Vec<u8>>),
    /// Nobody chose the program behind it, so every datagram of it goes back
    /// to the stack to be refused.
    Refused,
}

/// One pair of ends, and the socket that carries it.
struct Pair {
    socket: Arc<UdpSocket>,
    /// Dropping this is what ends the task reading answers: the pair going
    /// out of the table has to take its reader with it, or a socket nobody
    /// looks at any more would still be listened to.
    _until: tokio::sync::oneshot::Sender<()>,
    /// What the door is to know about a captured client, held so that the
    /// entry lives exactly as long as the pair does and no longer: the
    /// address it is kept under is one the operating system will hand out
    /// again.
    _kept: Option<Keeping>,
}

#[allow(clippy::too_many_arguments)]
async fn pass(
    mut taking: mpsc::Receiver<Datagram>,
    out: mpsc::Sender<Vec<u8>>,
    bind: Bind,
    catching: Option<Catching>,
    again: mpsc::Sender<Vec<u8>>,
    stop: Stop,
    idle_for: Duration,
    sweep_every: Duration,
) {
    let mut by: HashMap<Ends, Doing> = HashMap::new();
    // Where an answer about one client's program comes back to. Held here
    // for as long as the loop runs, so the receiver never closes while
    // something may still be asking.
    let (asked, mut answers) = mpsc::channel::<(Ends, Option<Caught>)>(QUEUED_MAX);
    loop {
        tokio::select! {
            taken = taking.recv() => match taken {
                Some(datagram) => {
                    send(&mut by, datagram, &out, &bind, catching.as_ref(), &again, &asked, idle_for).await;
                }
                // The stack has let go: nothing more will arrive.
                None => break,
            },
            answer = answers.recv() => {
                if let Some((ends, caught)) = answer {
                    told(&mut by, ends, caught, &out, catching.as_ref(), &again).await;
                }
            }
            // A pair going out of the table takes its reader with it,
            // because dropping `Pair` drops the sender the reader waits on.
            _ = tokio::time::sleep(sweep_every) => {
                by.retain(|_, doing| doing.last.elapsed() < idle_for);
            }
            _ = stopped(stop.clone()) => break,
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn send(
    by: &mut HashMap<Ends, Doing>,
    datagram: Datagram,
    out: &mpsc::Sender<Vec<u8>>,
    bind: &Bind,
    catching: Option<&Catching>,
    again: &mpsc::Sender<Vec<u8>>,
    asked: &mpsc::Sender<(Ends, Option<Caught>)>,
    idle_for: Duration,
) {
    if for_this_link(datagram.to.ip()) {
        return;
    }
    let ends = (datagram.from, datagram.to);
    if let Some(doing) = by.get_mut(&ends) {
        doing.last = Instant::now();
        match &mut doing.what {
            What::Through(pair) => {
                if let Err(e) = pair.socket.send(&datagram.body).await {
                    tracing::debug!("a datagram to {} did not go: {e}", datagram.to);
                }
            }
            What::Asking(held) => {
                if held.len() < HELD_MAX {
                    held.push(datagram.body);
                } else {
                    tracing::debug!(
                        "a datagram to {} was dropped while its program was asked for",
                        datagram.to
                    );
                }
            }
            What::Refused => refuse(again, datagram.from, datagram.to, &datagram.body),
        }
        return;
    }
    if by.len() >= PASSING_MAX {
        // Oldest first, so that a flood of new pairs cannot push out the
        // one a reply is coming back to.
        by.retain(|_, doing| doing.last.elapsed() < idle_for);
        if by.len() >= PASSING_MAX {
            tracing::debug!("no room to pass a datagram to {}", datagram.to);
            return;
        }
    }
    // A port that may be captured asks first, off this task, and what it
    // decides is what makes the pair -- to the door or back to be refused.
    if let Some(catching) = catching.filter(|_| ours_to_catch(datagram.to.port())) {
        let whether = Arc::clone(&catching.whether);
        let asked = asked.clone();
        let about = datagram.clone();
        tokio::spawn(async move {
            let caught = whether(about).await;
            // The loop has stopped, which is the only way this fails.
            let _ = asked.send((ends, caught)).await;
        });
        by.insert(ends, Doing::now(What::Asking(vec![datagram.body])));
        return;
    }
    let socket = match bind(datagram.to.is_ipv6()).await {
        Ok(socket) => socket,
        Err(e) => {
            tracing::warn!("no socket to pass a datagram to {}: {e}", datagram.to);
            return;
        }
    };
    let pair = match through(socket, datagram.to, ends, out, None).await {
        Ok(pair) => pair,
        Err(e) => {
            tracing::warn!("cannot reach {} to pass a datagram: {e}", datagram.to);
            return;
        }
    };
    if let Err(e) = pair.socket.send(&datagram.body).await {
        tracing::debug!("a datagram to {} did not go: {e}", datagram.to);
    }
    by.insert(ends, Doing::now(What::Through(pair)));
}

/// What the answer about one client's program says to do with it: a pair to
/// the door, or every datagram of it back to be refused.
async fn told(
    by: &mut HashMap<Ends, Doing>,
    ends: Ends,
    caught: Option<Caught>,
    out: &mpsc::Sender<Vec<u8>>,
    catching: Option<&Catching>,
    again: &mpsc::Sender<Vec<u8>>,
) {
    // Swept while the answer was coming, which a client that asked once and
    // went away leaves behind. Nothing of it is left to carry.
    let Some(doing) = by.get_mut(&ends) else {
        return;
    };
    let What::Asking(waiting) = &mut doing.what else {
        return;
    };
    let waiting = std::mem::take(waiting);
    let (client, server) = ends;
    let door = match (caught, catching) {
        (Some(caught), Some(catching)) => match to_the_door(caught, ends, out, catching).await {
            Ok(pair) => Some(pair),
            // The door is on this machine, so this is a fault rather than a
            // network: say so, and let the client fall back to TCP as it
            // would have before.
            Err(e) => {
                tracing::warn!("no way to tracewing's own HTTP/3 door for {client}: {e}");
                None
            }
        },
        _ => None,
    };
    match door {
        Some(pair) => {
            // In the order they came, which is the one thing holding them
            // was for.
            for body in waiting {
                if let Err(e) = pair.socket.send(&body).await {
                    tracing::debug!("a datagram to the HTTP/3 door did not go: {e}");
                }
            }
            doing.what = What::Through(pair);
        }
        None => {
            for body in waiting {
                refuse(again, client, server, &body);
            }
            doing.what = What::Refused;
        }
    }
}

/// A pair whose socket goes to tracewing's own door, with what the door is
/// to know about the client kept under the address it will see.
///
/// Kept **before** the pair exists, because the first datagram goes as soon
/// as it does: an entry written afterwards would be a race against the
/// handshake it is there to explain.
async fn to_the_door(
    caught: Caught,
    ends: Ends,
    out: &mpsc::Sender<Vec<u8>>,
    catching: &Catching,
) -> io::Result<Pair> {
    // Unspecified rather than the loopback, so that a door bound anywhere is
    // reachable; `local_addr` after `connect` is the address the door will
    // see either way.
    let socket = UdpSocket::bind(crate::upstream::any_port(caught.door.is_ipv6())).await?;
    socket.connect(caught.door).await?;
    let kept = catching.handed.keep(socket.local_addr()?, caught.known);
    through(socket, caught.door, ends, out, Some(kept)).await
}

/// A pair carrying this client's datagrams to `to`, with a task putting what
/// comes back to the client as packets from the server it was going to.
///
/// `to` is where the socket is connected, which is the server for a datagram
/// passed on and tracewing's own door for one being read. The client is told
/// the server either way: that is the only address it will take an answer
/// from.
async fn through(
    socket: UdpSocket,
    to: SocketAddr,
    ends: Ends,
    out: &mpsc::Sender<Vec<u8>>,
    kept: Option<Keeping>,
) -> io::Result<Pair> {
    // Connected, so that an answer from anywhere else is not taken for one
    // from the server. It is also what lets the reader use `recv`.
    socket.connect(to).await?;
    let socket = Arc::new(socket);
    let (until, ended) = tokio::sync::oneshot::channel();
    let (client, server) = ends;
    tokio::spawn(answers(
        Arc::clone(&socket),
        client,
        server,
        out.clone(),
        ended,
    ));
    Ok(Pair {
        socket,
        _until: until,
        _kept: kept,
    })
}

/// A datagram back to the stack, to be refused there as every datagram to
/// port 443 was before HTTP/3 capture existed.
fn refuse(again: &mpsc::Sender<Vec<u8>>, client: SocketAddr, server: SocketAddr, body: &[u8]) {
    let Some(packet) = packet_for(client, server, body) else {
        tracing::debug!("a datagram to {server} is too long to put back");
        return;
    };
    // `try_send` and not `send`: the stack is what feeds this task, so
    // waiting on the stack here is how the two would hold each other up. A
    // full queue drops, and a client whose attempt is dropped tries again.
    if again.try_send(packet).is_err() {
        tracing::debug!("a datagram to {server} was dropped instead of refused");
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
    // One byte more than can be put back, so that an answer too long to
    // carry is seen to be too long rather than cut short and sent on as a
    // datagram that was never sent.
    let mut buffer = vec![0u8; BODY_MAX + 1];
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
    use crate::source::{By, Target};
    use smoltcp::wire::{Ipv4Packet, Ipv6Packet};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn client() -> SocketAddr {
        "10.0.0.7:51000".parse().unwrap()
    }

    fn server() -> SocketAddr {
        "203.0.113.9:123".parse().unwrap()
    }

    /// Where a client's QUIC says it is going, which is never where it is
    /// sent once it is captured.
    fn quic_server() -> SocketAddr {
        "203.0.113.9:443".parse().unwrap()
    }

    /// A way out that must not be used: a captured datagram goes to the door
    /// through a socket of its own, and a test that reached for this one
    /// would be testing the wrong path.
    fn no_way_out() -> Bind {
        Arc::new(|_v6| {
            Box::pin(async { Err(io::Error::other("no way out of this test")) }) as Binding
        })
    }

    /// A catcher that keeps every client it is asked about and sends them to
    /// `door`, or keeps none where there is no door; `waits` is how long the
    /// operating system takes to answer. The count is how often it was
    /// asked, which is what says a pair of ends asks once however many
    /// datagrams go through it.
    fn catching(
        door: Option<SocketAddr>,
        handed: &Arc<Handed>,
        waits: Duration,
    ) -> (Catching, Arc<AtomicUsize>) {
        let asks = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&asks);
        let catching = Catching {
            on: Arc::new(|| true),
            handed: Arc::clone(handed),
            whether: Arc::new(move |datagram| {
                let asks = Arc::clone(&counted);
                Box::pin(async move {
                    asks.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(waits).await;
                    Some(Caught {
                        door: door?,
                        known: Known {
                            by: By::Tun,
                            client: datagram.from,
                            to: Target::Addr(datagram.to),
                            app: None,
                        },
                    })
                })
            }),
        };
        (catching, asks)
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
        let (again, _back) = mpsc::channel(8);
        let bind: Bind = Arc::new(|_v6| Box::pin(UdpSocket::bind("127.0.0.1:0")));
        let taken = passing(out, bind, None, again, stop);

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
        let (again, _back) = mpsc::channel(8);
        let bind: Bind = Arc::new(|_v6| Box::pin(UdpSocket::bind("127.0.0.1:0")));
        let taken = passing(out, bind, None, again, stop);

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
        let (again, _back) = mpsc::channel(8);
        let bind: Bind = Arc::new(|_v6| Box::pin(UdpSocket::bind("127.0.0.1:0")));
        let taken = waiting(
            out,
            bind,
            None,
            again,
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

    /// A datagram for a group or for everyone on the link is the link's
    /// own, and is not passed on: no socket is made for it. Passed on, it
    /// came back into the adapter from the new socket and filled the table
    /// (see `for_this_link`). The unicast one after them shows the way out
    /// is asked for at all, so that silence means something.
    #[tokio::test]
    async fn what_is_for_this_link_alone_is_not_passed_on() {
        let far = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = far.local_addr().unwrap();
        let (out, _coming) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let (again, _back) = mpsc::channel(8);
        let binds = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&binds);
        let bind: Bind = Arc::new(move |_v6| {
            counted.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { UdpSocket::bind("127.0.0.1:0").await })
        });
        let taken = passing(out, bind, None, again, stop);
        for to in [
            "224.0.0.252:5355",
            "239.255.255.250:1900",
            "255.255.255.255:67",
            "[ff02::1:3]:5355",
            "[ff02::fb]:5353",
        ] {
            let from = match to.starts_with('[') {
                true => "[fe80::1]:51000".parse().unwrap(),
                false => client(),
            };
            taken
                .send(Datagram {
                    from,
                    to: to.parse().unwrap(),
                    body: b"who is there".to_vec(),
                })
                .await
                .unwrap();
        }
        taken
            .send(Datagram {
                from: client(),
                to: server,
                body: b"to somewhere".to_vec(),
            })
            .await
            .unwrap();
        let mut buffer = [0u8; 64];
        let (len, _) = tokio::time::timeout(Duration::from_secs(5), far.recv_from(&mut buffer))
            .await
            .expect("the unicast one goes")
            .unwrap();
        assert_eq!(&buffer[..len], b"to somewhere");
        assert_eq!(
            binds.load(Ordering::SeqCst),
            1,
            "a socket for the unicast datagram and for nothing else"
        );
    }

    /// A socket that cannot be had is said and nothing else: no panic, and
    /// the next datagram is still passed.
    #[tokio::test]
    async fn a_socket_that_cannot_be_made_loses_one_datagram_and_no_more() {
        let far = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = far.local_addr().unwrap();
        let (out, _coming) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let (again, _back) = mpsc::channel(8);
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
        let taken = passing(out, bind, None, again, stop);

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

    /// The whole of capturing one client's HTTP/3, at the only level this
    /// file can be asked about it: the datagram reaches tracewing's own
    /// door, the door can look up whose it is, and what the door answers
    /// reaches the client as a packet from the server it was going to.
    #[tokio::test]
    async fn a_chosen_client_s_datagrams_go_to_the_door_and_its_answers_come_back() {
        let door = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let at = door.local_addr().unwrap();
        let handed = Arc::new(Handed::default());
        let (out, mut coming) = mpsc::channel(8);
        let (again, mut back) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let (catching, asks) = catching(Some(at), &handed, Duration::ZERO);
        let taken = passing(out, no_way_out(), Some(catching), again, stop);

        taken
            .send(Datagram {
                from: client(),
                to: quic_server(),
                body: b"an Initial".to_vec(),
            })
            .await
            .unwrap();

        let mut buffer = [0u8; 64];
        let (len, ours) = tokio::time::timeout(Duration::from_secs(5), door.recv_from(&mut buffer))
            .await
            .expect("the datagram reaches the door instead of being refused")
            .unwrap();
        assert_eq!(&buffer[..len], b"an Initial");

        // What the door needs and cannot see: every captured client reaches
        // it over the loopback from a socket of ours.
        let known = handed.of(ours).expect("the door can look this client up");
        assert_eq!(known.client, client(), "the client, not the socket of ours");
        assert_eq!(known.to.authority(), "203.0.113.9:443");
        assert!(matches!(known.by, By::Tun), "and which way in took it");

        door.send_to(b"an answer", ours).await.unwrap();
        let packet = tokio::time::timeout(Duration::from_secs(5), coming.recv())
            .await
            .expect("the answer comes back")
            .unwrap();
        let answer = datagram(&packet).expect("a datagram");
        assert_eq!(
            answer.from,
            quic_server(),
            "from the server the client sent to, which is the only address it will take"
        );
        assert_eq!(answer.to, client());
        assert_eq!(answer.body, b"an answer");
        assert_eq!(asks.load(Ordering::Relaxed), 1);
        assert!(back.try_recv().is_err(), "and nothing was refused");
    }

    /// A program nobody chose keeps what it had before any of this: its
    /// datagram goes back to the stack, which holds no socket for port 443
    /// and answers an ICMP port unreachable.
    ///
    /// The second datagram is the other half of the test. Asking the
    /// operating system which program holds a socket is the one expensive
    /// step here, and a client that retries -- which a refused QUIC client
    /// does, several times -- must not pay it again.
    #[tokio::test]
    async fn a_client_nobody_chose_is_put_back_to_be_refused_and_asked_about_once() {
        let handed = Arc::new(Handed::default());
        let (out, mut coming) = mpsc::channel(8);
        let (again, mut back) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let (catching, asks) = catching(None, &handed, Duration::ZERO);
        let taken = passing(out, no_way_out(), Some(catching), again, stop);

        for _ in 0..2 {
            taken
                .send(Datagram {
                    from: client(),
                    to: quic_server(),
                    body: b"an Initial".to_vec(),
                })
                .await
                .unwrap();
            let packet = tokio::time::timeout(Duration::from_secs(5), back.recv())
                .await
                .expect("the datagram comes back to be refused")
                .unwrap();
            let put_back = datagram(&packet).expect("the datagram it came in as");
            assert_eq!(put_back.from, client());
            assert_eq!(put_back.to, quic_server());
            assert_eq!(put_back.body, b"an Initial");
        }
        assert_eq!(
            asks.load(Ordering::Relaxed),
            1,
            "asked once for the pair of ends, not once per datagram"
        );
        assert!(
            coming.try_recv().is_err(),
            "and nothing was sent anywhere on its behalf"
        );
    }

    /// What arrives while the operating system is being asked waits for the
    /// answer and then goes in the order it came.
    ///
    /// Dropping it instead would cost a captured connection a retransmission
    /// timeout of the client's own at the start of every connection, which is
    /// hundreds of milliseconds to save a few bytes of queue.
    #[tokio::test]
    async fn what_arrives_while_the_program_is_asked_for_waits_and_goes_in_order() {
        let door = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let at = door.local_addr().unwrap();
        let handed = Arc::new(Handed::default());
        let (out, _coming) = mpsc::channel(8);
        let (again, _back) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let (catching, asks) = catching(Some(at), &handed, Duration::from_millis(100));
        let taken = passing(out, no_way_out(), Some(catching), again, stop);

        for body in [b"first".to_vec(), b"second".to_vec(), b"third".to_vec()] {
            taken
                .send(Datagram {
                    from: client(),
                    to: quic_server(),
                    body,
                })
                .await
                .unwrap();
        }

        let mut arrived: Vec<Vec<u8>> = Vec::new();
        for _ in 0..3 {
            let mut buffer = [0u8; 64];
            let len = tokio::time::timeout(Duration::from_secs(5), door.recv(&mut buffer))
                .await
                .expect("all three reach the door")
                .unwrap();
            arrived.push(buffer[..len].to_vec());
        }
        assert_eq!(
            arrived,
            [b"first".to_vec(), b"second".to_vec(), b"third".to_vec()]
        );
        assert_eq!(asks.load(Ordering::Relaxed), 1);
    }

    /// The door stops being told about a client whose pair of ends has gone.
    ///
    /// The address it is kept under is a port of this machine's that the
    /// operating system will hand out again, so an entry left behind would
    /// one day name somebody else's program for somebody else's connection.
    #[tokio::test]
    async fn the_door_is_told_about_a_client_for_exactly_as_long_as_its_pair_lives() {
        let door = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let at = door.local_addr().unwrap();
        let handed = Arc::new(Handed::default());
        let (out, _coming) = mpsc::channel(8);
        let (again, _back) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let (catching, _asks) = catching(Some(at), &handed, Duration::ZERO);
        let taken = waiting(
            out,
            no_way_out(),
            Some(catching),
            again,
            stop,
            Duration::from_millis(50),
            Duration::from_millis(10),
        );

        taken
            .send(Datagram {
                from: client(),
                to: quic_server(),
                body: b"an Initial".to_vec(),
            })
            .await
            .unwrap();
        let mut buffer = [0u8; 64];
        let (_, ours) = tokio::time::timeout(Duration::from_secs(5), door.recv_from(&mut buffer))
            .await
            .expect("the datagram reaches the door")
            .unwrap();
        assert!(handed.of(ours).is_some(), "while the pair lives");

        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            handed.of(ours).is_none(),
            "and not after the pair has been swept"
        );
    }

    /// An answer too long to put in a packet is dropped, not cut short.
    ///
    /// The read buffer is one byte longer than a datagram may carry, which
    /// is what turns a truncating read into a drop with a reason. QUIC is
    /// why it matters: a client handed 1472 bytes of a longer datagram
    /// fails that packet's own authentication and may drop the connection,
    /// with nothing anywhere saying why -- and the full-size datagram is
    /// exactly what a QUIC endpoint sends, so this is the edge rather than
    /// a far-off case.
    #[tokio::test]
    async fn an_answer_too_long_to_carry_is_dropped_rather_than_cut_short() {
        let far = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = far.local_addr().unwrap();
        let (out, mut coming) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let (again, _back) = mpsc::channel(8);
        let bind: Bind = Arc::new(|_v6| Box::pin(UdpSocket::bind("127.0.0.1:0")));
        let taken = passing(out, bind, None, again, stop);

        taken
            .send(Datagram {
                from: client(),
                to: server,
                body: b"ask".to_vec(),
            })
            .await
            .unwrap();
        let mut buffer = [0u8; 64];
        let (_, ours) = tokio::time::timeout(Duration::from_secs(5), far.recv_from(&mut buffer))
            .await
            .expect("the datagram goes out")
            .unwrap();

        far.send_to(&vec![7u8; BODY_MAX + 1], ours).await.unwrap();
        far.send_to(&vec![9u8; BODY_MAX], ours).await.unwrap();
        let packet = tokio::time::timeout(Duration::from_secs(5), coming.recv())
            .await
            .expect("the one that fits comes back")
            .unwrap();
        let answer = datagram(&packet).expect("a datagram");
        assert_eq!(
            answer.body,
            vec![9u8; BODY_MAX],
            "the full-size one, whole -- and the longer one left behind rather than cut down to it"
        );
    }
}
