//! Packets in, flows out: a TCP/IP stack of tracewing's own.
//!
//! Every way in that tracewing has today is handed a stream somebody else
//! built: a client that connected to the proxy port, a SOCKS5 client after its
//! handshake, a macOS network extension that connects on the app's behalf. The
//! ways in that are left are not like that. WinDivert on Windows, a TUN device
//! on Linux and a WireGuard tunnel all hand over **packets** — IP datagrams,
//! out of order, duplicated, retransmitted, with a TCP header each and no
//! stream anywhere. Between those and [`crate::capture`], which needs
//! something that reads and writes bytes, there has to be a TCP/IP stack, and
//! this is it.
//!
//! What it does:
//!
//! - Sorts packets into connections by their four addresses, and treats a
//!   `SYN` for one it has never seen as a new connection.
//! - **Answers as the server the client was going to.** The client sent its
//!   `SYN` to `140.82.121.4:443`; nothing here forwards that `SYN` anywhere,
//!   it is answered with a `SYN-ACK` as though we were GitHub. Reaching the
//!   real server is the proxy's job, further on, and it happens over an
//!   ordinary socket.
//! - Runs the rest of TCP against that client: sequence numbers, windows,
//!   acknowledgements, retransmission, packets that arrive out of order,
//!   `FIN` and `RST`. That is [`smoltcp`]'s work, not ours.
//! - Hands each connection over as a [`Flow`] whose stream is the bytes the
//!   client sent, and writes back out as packets whatever the proxy answers.
//!
//! - Reads **IPv4 and IPv6**. Both are only parsed as far as deciding whether
//!   a packet opens a connection; the rest is `smoltcp`'s. An IPv4 fragment
//!   and an IPv6 packet carrying an extension header are both left alone, for
//!   the same reason: the TCP header is not where this looks, so no
//!   connection is made and the stack drops the packet.
//!
//! What it does not do: connect to anybody, route, speak ARP (these are IP
//! packets, with no Ethernet frame around them), or know which program sent
//! anything. A packet carries addresses and nothing else, so these flows
//! arrive as [`Target::Addr`] and with no program against them. The name is
//! then read further on, out of the client's first TLS record, by
//! [`crate::handover`] and [`crate::name`] — so a flow from here is asked
//! for by name even though nothing here knows one. The program behind the
//! four addresses is still work of its own and is not here.
//!
//! See [capture-modes.md](../../../docs/design/capture-modes.md).

use crate::source::{Flow, Found, Source, Stop, Target, stopped};
use anyhow::Result;
use smoltcp::{
    iface::{Config, Interface, SocketHandle, SocketSet},
    phy::{Checksum, Device as PhyDevice, DeviceCapabilities, Medium, RxToken, TxToken},
    socket::tcp,
    time::Instant as Moment,
    wire::{HardwareAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address},
};
use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Notify, mpsc};

/// How many packets wait to be read before the one reading them has to wait
/// with them. A full queue drops, which is what a congested network does, and
/// TCP is built for it.
const PACKETS_MAX: usize = 1024;
/// What one connection may hold in each direction. This is the TCP window, so
/// it is also what decides throughput on a link with any latency.
const WINDOW: usize = 64 * 1024;
/// How many connections at once. Each one holds two windows, so this is the
/// memory this costs at worst: a flood of `SYN`s is a flood of nothing else.
const FLOWS_MAX: usize = 512;
/// What this stack says it can carry. 1500 less a worst-case IPv4 header.
const MTU: usize = 1480;
/// How long a connection that was made for a `SYN` waits for that `SYN` to
/// arrive before it is given up on. The packet is already in hand when the
/// connection is made, so this only runs out if the stack dropped it.
const SYN_WITHIN: Duration = Duration::from_secs(10);
/// The longest the loop sleeps when the stack has asked for nothing: it still
/// wakes to sweep connections whose time is up.
const SWEEP_EVERY: Duration = Duration::from_secs(1);

/// Where packets come from and go back to: a TUN device on Linux, a WinDivert
/// handle on Windows, the plain side of a WireGuard tunnel. Each of those is a
/// task that owns a descriptor and pumps both ways; none of that work is here,
/// and this is the shape they all meet. It is also how the stack is tested
/// with no operating system in it at all — a test writes packets in and reads
/// packets out.
pub struct Wire {
    incoming: mpsc::Receiver<Vec<u8>>,
    outgoing: mpsc::Sender<Vec<u8>>,
}

impl Wire {
    /// The wire, the end packets are written into, and the end they come out
    /// of: `(wire, to_stack, from_stack)`.
    pub fn channels() -> (Self, mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
        let (into_stack, incoming) = mpsc::channel(PACKETS_MAX);
        let (outgoing, out_of_stack) = mpsc::channel(PACKETS_MAX);
        (Self { incoming, outgoing }, into_stack, out_of_stack)
    }
}

/// The stack as a source of flows.
pub struct Stack {
    wire: Wire,
    /// How many connections at once. A field rather than the constant alone,
    /// so that the cap is tested with two instead of with five hundred.
    most: usize,
}

impl Stack {
    pub fn new(wire: Wire) -> Self {
        Self {
            wire,
            most: FLOWS_MAX,
        }
    }

    #[cfg(test)]
    fn holding_at_most(wire: Wire, most: usize) -> Self {
        Self { wire, most }
    }
}

impl Source for Stack {
    fn name(&self) -> String {
        "packets".to_owned()
    }

    // `own_ports` stays empty on purpose: a client caught this way never
    // connected to a port of tracewing's, it was going to the real server, so
    // there is no socket of ours for `crate::apps` to find it by.

    fn start(self: Box<Self>, _proxy: SocketAddr, found: Found, stop: Stop) -> Result<()> {
        tokio::spawn(run(self.wire, self.most, found, stop));
        Ok(())
    }
}

/// What a packet says about where it is going, as far as this needs to know.
/// Written here rather than taken from `smoltcp` because it answers a question
/// `smoltcp` is not asked until after we have decided to make a connection for
/// it: a `SYN` needs a socket listening on its destination before the stack
/// will answer it at all.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Peek {
    pub from: SocketAddr,
    pub to: SocketAddr,
    /// A `SYN` with no `ACK`: the first packet of a connection.
    pub opening: bool,
}

/// What carries TCP, in the `protocol` of an IPv4 header and the
/// `next_header` of an IPv6 one.
const TCP: u8 = 6;

/// The least a TCP header is, which is all of it this reads.
const TCP_HEAD: usize = 20;

/// IPv4, IPv6 and TCP only. A packet this does not understand is handed to
/// the stack anyway, which drops what it does not want: this is only here to
/// decide whether to make a connection first.
pub(crate) fn peek(packet: &[u8]) -> Option<Peek> {
    match *packet.first()? >> 4 {
        4 => peek_v4(packet),
        6 => peek_v6(packet),
        _ => None,
    }
}

fn peek_v4(packet: &[u8]) -> Option<Peek> {
    let header = usize::from(packet[0] & 0x0f) * 4;
    if header < 20 || packet.len() < header + TCP_HEAD || packet[9] != TCP {
        return None;
    }
    // A fragment that is not the first carries no TCP header, and the first
    // of several is not worth the reassembly here.
    let fragment = u16::from_be_bytes([packet[6], packet[7]]);
    if fragment & 0x1fff != 0 || fragment & 0x2000 != 0 {
        return None;
    }
    let from = Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]);
    let to = Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]);
    ends(&packet[header..], from.into(), to.into())
}

/// IPv6's header is a fixed forty bytes, and what follows it is named by
/// `next_header` instead of by a protocol field at a fixed place. An
/// extension header — hop-by-hop options, routing, a fragment — therefore
/// puts the TCP header somewhere this does not look, so a packet carrying
/// one is left alone rather than guessed at: no connection is made, the
/// stack drops it, and a client on a path that needs them is a client this
/// does not catch. That is the same answer an IPv4 fragment gets, for the
/// same reason, and it is worth knowing rather than discovering.
fn peek_v6(packet: &[u8]) -> Option<Peek> {
    const HEAD: usize = 40;
    if packet.len() < HEAD + TCP_HEAD || packet[6] != TCP {
        return None;
    }
    let from = Ipv6Addr::from(<[u8; 16]>::try_from(&packet[8..24]).ok()?);
    let to = Ipv6Addr::from(<[u8; 16]>::try_from(&packet[24..40]).ok()?);
    ends(&packet[HEAD..], from.into(), to.into())
}

/// The ports and the flags, which sit in the same place whichever IP carried
/// them there.
fn ends(tcp: &[u8], from: IpAddr, to: IpAddr) -> Option<Peek> {
    const SYN: u8 = 0x02;
    const ACK: u8 = 0x10;
    let flags = tcp[13];
    Some(Peek {
        from: SocketAddr::new(from, u16::from_be_bytes([tcp[0], tcp[1]])),
        to: SocketAddr::new(to, u16::from_be_bytes([tcp[2], tcp[3]])),
        opening: flags & SYN != 0 && flags & ACK == 0,
    })
}

/// The packets waiting to be read, and where to put the ones written.
struct Device {
    waiting: VecDeque<Vec<u8>>,
    out: mpsc::Sender<Vec<u8>>,
}

struct Taking(Vec<u8>);

impl RxToken for Taking {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.0)
    }
}

struct Putting<'a>(&'a mpsc::Sender<Vec<u8>>);

impl TxToken for Putting<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut packet = vec![0u8; len];
        let answer = f(&mut packet);
        // A full queue drops the packet, and TCP sends it again. Blocking
        // here would stop the stack instead, which is worse.
        let _ = self.0.try_send(packet);
        answer
    }
}

impl PhyDevice for Device {
    type RxToken<'a> = Taking;
    type TxToken<'a> = Putting<'a>;

    fn receive(&mut self, _now: Moment) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let packet = self.waiting.pop_front()?;
        Some((Taking(packet), Putting(&self.out)))
    }

    fn transmit(&mut self, _now: Moment) -> Option<Self::TxToken<'_>> {
        Some(Putting(&self.out))
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = MTU;
        // Computed on the way out, and **not** checked on the way in: a
        // packet handed over by a kernel may have an empty or wrong checksum
        // because the hardware was going to do it. Checking would drop every
        // one of them, and the symptom is that nothing comes through at all.
        caps.checksum.ipv4 = Checksum::Tx;
        caps.checksum.tcp = Checksum::Tx;
        caps
    }
}

/// One connection the stack is holding for the proxy.
struct Conn {
    /// The flow, until the client has actually connected. A socket has to
    /// exist before its `SYN` is dealt with, or the stack has nothing to
    /// answer with -- but the `SYN` may still be refused, by a bad checksum
    /// or anything else, and a flow handed over for a connection that never
    /// happened is a flow the window shows and nobody made. So it waits here
    /// until the socket says it is established.
    pending: Option<Flow>,
    /// Bytes on their way to the proxy. `None` once the client's end is done
    /// and the proxy has been told, by this being dropped.
    to_proxy: Option<mpsc::Sender<Vec<u8>>>,
    /// Bytes the proxy sent, not yet taken by the socket.
    from_proxy: mpsc::Receiver<Vec<u8>>,
    waiting: Option<Vec<u8>>,
    /// To notice a connection whose `SYN` never arrived.
    made: Instant,
}

/// Carries bytes between one connection's socket and the stream the proxy
/// holds. The stack itself never awaits anything of the proxy's: it leaves
/// that to one of these, and sweeps when told there is something to do.
async fn pump(
    mut ours: tokio::io::DuplexStream,
    mut to_proxy: mpsc::Receiver<Vec<u8>>,
    from_proxy: mpsc::Sender<Vec<u8>>,
    woken: Arc<Notify>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buffer = vec![0u8; 16 * 1024];
    loop {
        tokio::select! {
            theirs = to_proxy.recv() => match theirs {
                Some(bytes) => {
                    if ours.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                // The client's end is done, so the proxy's read should end.
                None => {
                    let _ = ours.shutdown().await;
                    break;
                }
            },
            read = ours.read(&mut buffer) => match read {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    // Reserving first, so that a chunk is never read out of
                    // the proxy and then dropped.
                    if from_proxy.send(buffer[..n].to_vec()).await.is_err() {
                        break;
                    }
                    woken.notify_one();
                }
            },
        }
    }
    // Either way the stack should look at this connection again.
    woken.notify_one();
}

fn moment(started: Instant) -> Moment {
    Moment::from_micros(started.elapsed().as_micros() as i64)
}

async fn run(wire: Wire, most: usize, found: Found, stop: Stop) {
    let started = Instant::now();
    let Wire {
        mut incoming,
        outgoing,
    } = wire;
    let mut device = Device {
        waiting: VecDeque::new(),
        out: outgoing,
    };
    let mut config = Config::new(HardwareAddress::Ip);
    config.random_seed = seed();
    let mut iface = Interface::new(config, &mut device, moment(started));
    // Every destination is somebody else's address, so the interface has to
    // answer for all of them. `set_any_ip` is what does that, and in 0.14 it
    // does it alone: `InterfaceInner::has_ip_addr` returns `true` before
    // looking at anything when `any_ip` is set, so neither the addresses nor
    // the routes below are consulted on the way in.
    //
    // They are kept anyway, and the reason is worth writing down because no
    // test here can show it. `set_any_ip`'s own documentation says the
    // interface accepts a prefix only when a route names one of its
    // addresses as the gateway — which the implementation short-circuits.
    // Satisfying the documented contract costs four lines; relying on the
    // short-circuit would mean a `smoltcp` upgrade that honours its own
    // documentation stops every connection, with nothing failing here to say
    // so. Taking them out passes all twelve tests, which is the measure of
    // what the tests cover rather than of what the stack needs.
    iface.set_any_ip(true);
    iface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(IpAddress::v4(0, 0, 0, 0), 0));
        let _ = addrs.push(IpCidr::new(IpAddress::v6(0, 0, 0, 0, 0, 0, 0, 0), 0));
    });
    let _ = iface
        .routes_mut()
        .add_default_ipv4_route(Ipv4Address::new(0, 0, 0, 0));
    // The same for IPv6, for the reason above and not because a test needs
    // it.
    let _ = iface
        .routes_mut()
        .add_default_ipv6_route(Ipv6Address::UNSPECIFIED);
    let mut sockets = SocketSet::new(Vec::new());
    let mut conns: HashMap<SocketHandle, Conn> = HashMap::new();
    let woken = Arc::new(Notify::new());

    loop {
        iface.poll(moment(started), &mut device, &mut sockets);
        for flow in sweep(&mut sockets, &mut conns) {
            found.flow(flow).await;
        }
        let sleep = iface
            .poll_at(moment(started), &sockets)
            .map(|at| {
                at.total_micros()
                    .saturating_sub(moment(started).total_micros())
            })
            .map(|us| Duration::from_micros(us.max(0) as u64))
            .unwrap_or(SWEEP_EVERY)
            .min(SWEEP_EVERY);
        tokio::select! {
            packet = incoming.recv() => match packet {
                Some(packet) => {
                    open_for(&packet, most, &mut sockets, &mut conns, &woken);
                    device.waiting.push_back(packet);
                }
                None => break,
            },
            _ = woken.notified() => {}
            _ = tokio::time::sleep(sleep) => {}
            _ = stopped(stop.clone()) => break,
        }
    }
}

fn seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x5eed)
}

/// A `SYN` for a connection nothing is listening for yet: make one, listening
/// on the address the client was going to, and hand the flow over before the
/// packet reaches the stack. Anything else is left alone.
fn open_for(
    packet: &[u8],
    most: usize,
    sockets: &mut SocketSet<'static>,
    conns: &mut HashMap<SocketHandle, Conn>,
    woken: &Arc<Notify>,
) {
    let Some(seen) = peek(packet) else {
        return;
    };
    if !seen.opening {
        return;
    }
    if conns.len() >= most {
        tracing::warn!("{most} connections already: dropping {}", seen.from);
        return;
    }
    // One already listening for this one, as a `SYN` sent again is. The
    // whole address and not only the port: two clients going to different
    // servers on 443 are two connections, and comparing ports alone made the
    // second of them vanish whenever the first was still waiting for its own
    // `SYN` to be dealt with.
    let already = conns.keys().any(|handle| {
        let socket = sockets.get::<tcp::Socket>(*handle);
        // All four addresses. Either end alone is not enough: one client port
        // can go to two servers, and two clients can go to one.
        let open = socket
            .remote_endpoint()
            .is_some_and(|at| same(at, seen.from))
            && socket.local_endpoint().is_some_and(|at| same(at, seen.to));
        let waiting = socket.state() == tcp::State::Listen
            && socket.listen_endpoint().port == seen.to.port()
            && socket.listen_endpoint().addr == Some(address(seen.to.ip()));
        open || waiting
    });
    if already {
        return;
    }
    let mut socket = tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; WINDOW]),
        tcp::SocketBuffer::new(vec![0u8; WINDOW]),
    );
    let to = smoltcp::wire::IpEndpoint::new(address(seen.to.ip()), seen.to.port());
    if let Err(e) = socket.listen(to) {
        tracing::warn!("listening for {}: {e}", seen.to);
        return;
    }
    let handle = sockets.add(socket);
    let (ours, theirs) = tokio::io::duplex(WINDOW);
    let (to_proxy, proxy_reads) = mpsc::channel(4);
    let (proxy_wrote, from_proxy) = mpsc::channel(4);
    tokio::spawn(pump(ours, proxy_reads, proxy_wrote, Arc::clone(woken)));
    conns.insert(
        handle,
        Conn {
            pending: Some(Flow {
                stream: Box::new(theirs),
                client: seen.from,
                to: Target::Addr(seen.to),
                who: None,
            }),
            to_proxy: Some(to_proxy),
            from_proxy,
            waiting: None,
            made: Instant::now(),
        },
    );
}

/// `smoltcp` 0.14's addresses are `core::net`'s, so this only picks the arm.
fn address(ip: IpAddr) -> IpAddress {
    match ip {
        IpAddr::V4(ip) => IpAddress::Ipv4(ip),
        IpAddr::V6(ip) => IpAddress::Ipv6(ip),
    }
}

fn same(at: smoltcp::wire::IpEndpoint, addr: SocketAddr) -> bool {
    at.port == addr.port() && at.addr == address(addr.ip())
}

/// Moves bytes both ways for every connection, and lets go of the ones that
/// are over.
fn sweep(sockets: &mut SocketSet<'static>, conns: &mut HashMap<SocketHandle, Conn>) -> Vec<Flow> {
    let mut done: Vec<SocketHandle> = Vec::new();
    let mut opened: Vec<Flow> = Vec::new();
    for (handle, conn) in conns.iter_mut() {
        let socket = sockets.get_mut::<tcp::Socket>(*handle);
        // The client is through: the proxy can have the flow now.
        if conn.pending.is_some() && socket.may_send() {
            opened.extend(conn.pending.take());
        }
        // A connection made for a `SYN` that never arrived.
        if socket.state() == tcp::State::Listen && conn.made.elapsed() > SYN_WITHIN {
            done.push(*handle);
            continue;
        }
        // The client's bytes, to the proxy. Room is taken first, so that
        // nothing is read out of the socket and then dropped.
        while socket.can_recv() {
            let Some(sender) = conn.to_proxy.as_ref() else {
                break;
            };
            let Ok(room) = sender.try_reserve() else {
                break;
            };
            let mut buffer = vec![0u8; 16 * 1024];
            match socket.recv_slice(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buffer.truncate(n);
                    room.send(buffer);
                }
            }
        }
        // The proxy's bytes, to the client.
        loop {
            if conn.waiting.is_none() {
                match conn.from_proxy.try_recv() {
                    Ok(bytes) => conn.waiting = Some(bytes),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    // The proxy is done writing: the client's end closes.
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        socket.close();
                        break;
                    }
                }
            }
            let Some(bytes) = conn.waiting.take() else {
                break;
            };
            if !socket.can_send() {
                conn.waiting = Some(bytes);
                break;
            }
            match socket.send_slice(&bytes) {
                Ok(n) if n < bytes.len() => {
                    conn.waiting = Some(bytes[n..].to_vec());
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        // The client is done sending: the proxy's read should end, which
        // dropping this does. Asked of the state and not of `may_recv`,
        // which is also false before the handshake is finished -- that read
        // as "the client is done" and ended the stream before it had begun.
        let client_done = matches!(
            socket.state(),
            tcp::State::CloseWait
                | tcp::State::LastAck
                | tcp::State::Closing
                | tcp::State::TimeWait
                | tcp::State::Closed
        );
        if client_done && !socket.can_recv() {
            conn.to_proxy = None;
        }
        if !socket.is_open() && !socket.is_active() {
            done.push(*handle);
        }
    }
    for handle in done {
        conns.remove(&handle);
        sockets.remove(handle);
    }
    opened
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Took;
    use smoltcp::wire::{
        IpProtocol, Ipv4Packet, Ipv4Repr, Ipv6Packet, Ipv6Repr, TcpControl, TcpPacket, TcpRepr,
        TcpSeqNumber,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A TCP client written by hand, so that what goes into the stack is
    /// packets and nothing else. It keeps only as much state as a
    /// conversation needs: what it has sent, and what it has been told.
    struct Client {
        from: SocketAddr,
        to: SocketAddr,
        seq: TcpSeqNumber,
        ack: Option<TcpSeqNumber>,
    }

    impl Client {
        fn new(from: &str, to: &str) -> Self {
            Self {
                from: from.parse().unwrap(),
                to: to.parse().unwrap(),
                seq: TcpSeqNumber(1000),
                ack: None,
            }
        }

        /// A packet of its own making. `smoltcp`'s own writer puts the headers
        /// together, so that a mistake of mine in `peek` cannot be hidden by
        /// the same mistake in the test's hand-rolled header.
        fn packet(&self, control: TcpControl, payload: &[u8]) -> Vec<u8> {
            let tcp = TcpRepr {
                src_port: self.from.port(),
                dst_port: self.to.port(),
                control,
                seq_number: self.seq,
                ack_number: self.ack,
                window_len: 64000,
                window_scale: None,
                max_seg_size: None,
                sack_permitted: false,
                sack_ranges: [None; 3],
                timestamp: None,
                payload,
            };
            let caps = Default::default();
            // The TCP header is the same either way; only the IP header
            // around it differs, and `smoltcp` writes both, so a mistake of
            // mine in `peek` cannot be hidden by the same mistake here.
            match (self.from.ip(), self.to.ip()) {
                (IpAddr::V4(from), IpAddr::V4(to)) => {
                    let ip = Ipv4Repr {
                        src_addr: from,
                        dst_addr: to,
                        next_header: IpProtocol::Tcp,
                        payload_len: tcp.buffer_len(),
                        hop_limit: 64,
                    };
                    let mut bytes = vec![0u8; ip.buffer_len() + tcp.buffer_len()];
                    ip.emit(&mut Ipv4Packet::new_unchecked(&mut bytes), &caps);
                    tcp.emit(
                        &mut TcpPacket::new_unchecked(&mut bytes[ip.buffer_len()..]),
                        &from.into(),
                        &to.into(),
                        &caps,
                    );
                    bytes
                }
                (IpAddr::V6(from), IpAddr::V6(to)) => {
                    let ip = Ipv6Repr {
                        src_addr: from,
                        dst_addr: to,
                        next_header: IpProtocol::Tcp,
                        payload_len: tcp.buffer_len(),
                        hop_limit: 64,
                    };
                    let mut bytes = vec![0u8; ip.buffer_len() + tcp.buffer_len()];
                    // IPv6 has no header checksum, so `emit` takes none.
                    ip.emit(&mut Ipv6Packet::new_unchecked(&mut bytes));
                    tcp.emit(
                        &mut TcpPacket::new_unchecked(&mut bytes[ip.buffer_len()..]),
                        &from.into(),
                        &to.into(),
                        &caps,
                    );
                    bytes
                }
                _ => panic!("both ends of one connection are the same kind of address"),
            }
        }

        /// A packet, with the sequence number moved on by what it occupies:
        /// a byte each for the payload, and one for a `SYN` or a `FIN`. Kept
        /// here rather than in each test, because a test that counts its own
        /// sequence numbers gets them wrong -- both of the first two did, and
        /// the stack was blamed for it.
        fn send(&mut self, control: TcpControl, payload: &[u8]) -> Vec<u8> {
            let packet = self.packet(control, payload);
            let occupies = match control {
                TcpControl::Syn | TcpControl::Fin => 1,
                _ => 0,
            };
            self.seq += payload.len() + occupies;
            packet
        }

        /// Takes in what the stack sent back, so the next packet answers it.
        fn heard(&mut self, packet: &[u8]) -> (TcpControl, Vec<u8>) {
            let (control, seq, payload) = match packet.first().map(|b| b >> 4) {
                Some(4) => {
                    let ip = Ipv4Packet::new_checked(packet).expect("an IPv4 packet");
                    let tcp = TcpPacket::new_checked(ip.payload()).expect("a TCP packet");
                    let repr = TcpRepr::parse(
                        &tcp,
                        &ip.src_addr().into(),
                        &ip.dst_addr().into(),
                        &Default::default(),
                    )
                    .expect("a TCP header");
                    (repr.control, repr.seq_number, repr.payload.to_vec())
                }
                Some(6) => {
                    let ip = Ipv6Packet::new_checked(packet).expect("an IPv6 packet");
                    let tcp = TcpPacket::new_checked(ip.payload()).expect("a TCP packet");
                    let repr = TcpRepr::parse(
                        &tcp,
                        &ip.src_addr().into(),
                        &ip.dst_addr().into(),
                        &Default::default(),
                    )
                    .expect("a TCP header");
                    (repr.control, repr.seq_number, repr.payload.to_vec())
                }
                other => panic!("an IP packet, not one starting {other:?}"),
            };
            // What it sent, plus one for a SYN or a FIN, is what to acknowledge.
            let counts = usize::from(control != TcpControl::None && control != TcpControl::Psh);
            self.ack = Some(seq + payload.len() + counts);
            (control, payload)
        }
    }

    /// The stack, the end packets go into, the end they come out of, and the
    /// flows it hands over.
    struct Running {
        into: mpsc::Sender<Vec<u8>>,
        out: mpsc::Receiver<Vec<u8>>,
        took: mpsc::Receiver<Took>,
        _stop: tokio::sync::watch::Sender<bool>,
    }

    fn start(most: usize) -> Running {
        let (wire, into, out) = Wire::channels();
        let (found, took) = Found::channel();
        let (stop_tx, stop) = tokio::sync::watch::channel(false);
        Box::new(Stack::holding_at_most(wire, most))
            .start("127.0.0.1:9090".parse().unwrap(), found, stop)
            .unwrap();
        Running {
            into,
            out,
            took,
            _stop: stop_tx,
        }
    }

    impl Running {
        /// The three packets of a connection, and the flow that comes of it.
        /// The flow is handed over only once the client is through, so every
        /// test has to finish the handshake, as a client does.
        async fn connect(&mut self, client: &mut Client) -> Flow {
            let syn = client.send(TcpControl::Syn, &[]);
            self.into.send(syn).await.unwrap();
            let (control, _) = client.heard(&self.next().await);
            assert_eq!(control, TcpControl::Syn, "a SYN-ACK answers the SYN");
            let ack = client.send(TcpControl::None, &[]);
            self.into.send(ack).await.unwrap();
            self.flow().await
        }

        /// The next packet out, with a deadline: a stack that answers nothing
        /// would otherwise hang the suite rather than fail it.
        async fn next(&mut self) -> Vec<u8> {
            tokio::time::timeout(Duration::from_secs(5), self.out.recv())
                .await
                .expect("a packet back")
                .expect("the stack is still there")
        }

        async fn flow(&mut self) -> Flow {
            match tokio::time::timeout(Duration::from_secs(5), self.took.recv()).await {
                Ok(Some(Took::Flow(flow))) => flow,
                other => panic!("a flow, not {:?}", other.map(|t| t.is_some())),
            }
        }
    }

    /// The whole of it, in the order a connection happens: a `SYN` is
    /// answered, the flow says where the client was going, what the client
    /// sends arrives on the stream, and what is written to the stream goes
    /// back out as a packet.
    #[tokio::test]
    async fn a_syn_becomes_a_flow_that_carries_bytes_both_ways() {
        let mut running = start(8);
        let mut client = Client::new("10.0.0.7:51000", "140.82.121.4:443");
        let flow = running.connect(&mut client).await;
        assert_eq!(flow.client, client.from);
        assert_eq!(
            flow.to,
            Target::Addr(client.to),
            "named by where the client was going, which is all a packet says"
        );
        assert!(flow.who.is_none(), "a packet says nothing about a program");
        let mut stream = flow.stream;

        let get = client.send(TcpControl::Psh, b"GET / HTTP/1.1\r\n\r\n");
        running.into.send(get).await.unwrap();

        let mut said = [0u8; 18];
        tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut said))
            .await
            .expect("the client's bytes reach the stream")
            .unwrap();
        assert_eq!(&said, b"GET / HTTP/1.1\r\n\r\n");

        // And the other way: what the proxy writes comes out as packets.
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\n\r\n")
            .await
            .unwrap();
        let mut back = Vec::new();
        for _ in 0..8 {
            let packet = running.next().await;
            let (_, payload) = client.heard(&packet);
            back.extend_from_slice(&payload);
            if back.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        assert_eq!(
            String::from_utf8_lossy(&back),
            "HTTP/1.1 204 No Content\r\n\r\n"
        );
    }

    /// A client that says it is done ends the stream, rather than leaving the
    /// proxy waiting on a read that will never answer.
    #[tokio::test]
    async fn a_fin_ends_the_stream() {
        let mut running = start(8);
        let mut client = Client::new("10.0.0.7:51001", "93.184.216.34:80");
        let mut stream = running.connect(&mut client).await.stream;
        // Something to say, and then done saying it. The bytes before the
        // `FIN` are what makes this test fail if the stream ends too early
        // rather than at the right moment -- an earlier version of this
        // ended every stream before the handshake had finished, and a test
        // that only looked for the end passed.
        let bye = client.send(TcpControl::Psh, b"bye");
        running.into.send(bye).await.unwrap();
        let fin = client.send(TcpControl::Fin, &[]);
        running.into.send(fin).await.unwrap();

        let mut rest = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut rest))
            .await
            .expect("the read ends")
            .unwrap();
        assert_eq!(&rest, b"bye", "what was sent, and then the end of it");
    }

    /// A packet in the middle of a conversation nothing knows about makes no
    /// flow: only a `SYN` opens one. Otherwise a scan, or the leftovers of a
    /// connection from before tracewing started, would each become a flow.
    #[tokio::test]
    async fn a_packet_that_opens_nothing_is_not_a_flow() {
        let mut running = start(8);
        let mut client = Client::new("10.0.0.7:51002", "1.1.1.1:443");
        let middle = client.send(TcpControl::Psh, b"in the middle");
        running.into.send(middle).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(running.took.try_recv().is_err(), "no flow");
    }

    /// The cap is what stops a flood of `SYN`s from being a flood of windows.
    #[tokio::test]
    async fn no_more_connections_than_the_cap() {
        let mut running = start(2);
        let mut clients: Vec<Client> = (52000u16..52003)
            .map(|port| Client::new(&format!("10.0.0.7:{port}"), "140.82.121.4:443"))
            .collect();
        let _first = running.connect(&mut clients[0]).await;
        let _second = running.connect(&mut clients[1]).await;
        // The third gets no socket, so its SYN is answered by nothing.
        let syn = clients[2].send(TcpControl::Syn, &[]);
        running.into.send(syn).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            running.took.try_recv().is_err(),
            "the third is dropped, not held"
        );
    }

    /// Two clients going to different servers on the same port are two
    /// connections. Comparing only the port made the second one vanish.
    #[tokio::test]
    async fn two_servers_on_one_port_are_two_flows() {
        let mut running = start(8);
        let mut first_client = Client::new("10.0.0.7:53000", "140.82.121.4:443");
        let mut second_client = Client::new("10.0.0.7:53000", "93.184.216.34:443");
        let first = running.connect(&mut first_client).await;
        let second = running.connect(&mut second_client).await;
        let mut both = [first.to, second.to];
        both.sort_by_key(|t| t.authority());
        assert_eq!(
            both,
            [
                Target::Addr("140.82.121.4:443".parse().unwrap()),
                Target::Addr("93.184.216.34:443".parse().unwrap())
            ]
        );
    }

    /// Packets that arrive the wrong way round are put back in order before
    /// the proxy sees them. That is `smoltcp`'s work, but only if nothing
    /// here reads the socket before it is ready to be read.
    #[tokio::test]
    async fn packets_out_of_order_arrive_in_order() {
        let mut running = start(8);
        let mut client = Client::new("10.0.0.7:53100", "1.2.3.4:80");
        let mut stream = running.connect(&mut client).await.stream;

        // "second" is sent first, holding a sequence number that comes after
        // "first ", which the stack has not seen yet.
        let first = client.packet(TcpControl::Psh, b"first ");
        client.seq += 6;
        let second = client.send(TcpControl::Psh, b"second");
        running.into.send(second).await.unwrap();
        running.into.send(first).await.unwrap();

        let mut said = [0u8; 12];
        tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut said))
            .await
            .expect("both halves arrive")
            .unwrap();
        assert_eq!(&said, b"first second");
    }

    /// A connection the client resets as soon as it is up ends the proxy's
    /// stream rather than leaving it open on a connection that is gone.
    #[tokio::test]
    async fn a_reset_ends_the_stream() {
        let mut running = start(8);
        let mut client = Client::new("10.0.0.7:53300", "1.2.3.4:80");
        let mut stream = running.connect(&mut client).await.stream;
        let rst = client.send(TcpControl::Rst, &[]);
        running.into.send(rst).await.unwrap();
        let mut rest = Vec::new();
        tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut rest))
            .await
            .expect("the read ends rather than waiting for a connection that is gone")
            .unwrap();
        assert!(rest.is_empty());
    }

    /// A packet whose checksums are wrong is taken anyway, because a kernel
    /// handing one over may have left them to hardware that never ran. This
    /// is the one-line capability in `capabilities`, and without a test for
    /// it the symptom would be a device on which nothing works at all.
    #[tokio::test]
    async fn a_packet_whose_checksums_are_wrong_is_still_taken() {
        let mut running = start(8);
        let mut client = Client::new("10.0.0.7:53200", "1.2.3.4:80");
        let mut syn = client.send(TcpControl::Syn, &[]);
        // The IPv4 header's checksum, then the TCP one, both made wrong.
        syn[10] ^= 0xff;
        let tcp = usize::from(syn[0] & 0x0f) * 4;
        syn[tcp + 16] ^= 0xff;
        running.into.send(syn).await.unwrap();
        // The answer is the evidence: a stack that threw the packet away
        // would send nothing, and the flow would never be handed over.
        let (control, _) = client.heard(&running.next().await);
        assert_eq!(
            control,
            TcpControl::Syn,
            "a SYN-ACK, wrong checksums and all"
        );
        let ack = client.send(TcpControl::None, &[]);
        running.into.send(ack).await.unwrap();
        assert_eq!(running.flow().await.to, Target::Addr(client.to));
    }

    #[test]
    fn what_a_packet_says_about_where_it_is_going() {
        let client = Client::new("10.0.0.7:51003", "140.82.121.4:443");
        let syn = client.packet(TcpControl::Syn, &[]);
        assert_eq!(
            peek(&syn),
            Some(Peek {
                from: client.from,
                to: client.to,
                opening: true
            })
        );
        // A SYN with an ACK is the answer to one, not the start of anything.
        let mut answering = Client::new("10.0.0.7:51003", "140.82.121.4:443");
        answering.ack = Some(TcpSeqNumber(5));
        let syn_ack = answering.packet(TcpControl::Syn, &[]);
        assert!(!peek(&syn_ack).unwrap().opening);
        // Data in the middle is not an opening either.
        assert!(!peek(&client.packet(TcpControl::Psh, b"x")).unwrap().opening);
        // Not an IP version it knows, too short to hold a header, not TCP:
        // nothing to say.
        assert_eq!(peek(&[0x50, 0, 0, 0]), None);
        assert_eq!(peek(&[0x60, 0, 0, 0]), None, "IPv6, but four bytes of it");
        assert_eq!(peek(&[]), None);
        let mut udp = client.packet(TcpControl::Syn, &[]);
        udp[9] = 17;
        assert_eq!(peek(&udp), None);
    }

    /// The same, where the addresses are forty bytes of header away and the
    /// protocol is named by `next_header` instead of by `protocol`.
    #[test]
    fn what_an_ipv6_packet_says_about_where_it_is_going() {
        let client = Client::new("[2001:db8::7]:51004", "[2606:50c0:8000::153]:443");
        let syn = client.packet(TcpControl::Syn, &[]);
        assert_eq!(syn[0] >> 4, 6, "an IPv6 packet, as smoltcp wrote it");
        assert_eq!(
            peek(&syn),
            Some(Peek {
                from: client.from,
                to: client.to,
                opening: true
            })
        );

        // An extension header moves the TCP header, so this is left alone
        // rather than read as though the TCP header were where it usually
        // is: byte 6 says what follows, and 0 is hop-by-hop options.
        let mut extended = syn.clone();
        extended[6] = 0;
        assert_eq!(peek(&extended), None, "hop-by-hop options, not TCP");

        let mut udp = syn.clone();
        udp[6] = 17;
        assert_eq!(peek(&udp), None);

        // The fixed header alone, with no room for a TCP header after it.
        assert_eq!(peek(&syn[..40]), None);

        let mut answering = Client::new("[2001:db8::7]:51004", "[2606:50c0:8000::153]:443");
        answering.ack = Some(TcpSeqNumber(5));
        assert!(
            !peek(&answering.packet(TcpControl::Syn, &[]))
                .unwrap()
                .opening
        );
    }

    /// A connection over IPv6, end to end: reading the packet is only half
    /// of it, and this is the half that shows the stack answers as the
    /// server the client was going to over IPv6 as well.
    ///
    /// It does **not** test the interface's IPv6 address or route. Taking
    /// both out leaves this green, because `set_any_ip` short-circuits the
    /// check that would have consulted them — see the note in `run`. Said
    /// here because the obvious reading of this test is the wrong one.
    #[tokio::test]
    async fn a_connection_over_ipv6_is_answered_and_carries_bytes() {
        let mut running = start(8);
        let mut client = Client::new("[2001:db8::7]:51005", "[2606:50c0:8000::153]:443");
        let flow = running.connect(&mut client).await;
        assert_eq!(flow.client, client.from);
        assert_eq!(
            flow.to,
            Target::Addr(client.to),
            "named by where the client was going, which is all a packet says"
        );
        let mut stream = flow.stream;

        let get = client.send(TcpControl::Psh, b"GET / HTTP/1.1\r\n\r\n");
        running.into.send(get).await.unwrap();
        let mut said = [0u8; 18];
        tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut said))
            .await
            .expect("the client's bytes reach the stream")
            .unwrap();
        assert_eq!(&said, b"GET / HTTP/1.1\r\n\r\n");

        stream
            .write_all(b"HTTP/1.1 204 No Content\r\n\r\n")
            .await
            .unwrap();
        let mut back = Vec::new();
        for _ in 0..8 {
            let packet = running.next().await;
            assert_eq!(packet[0] >> 4, 6, "answered over IPv6, as it arrived");
            let (_, payload) = client.heard(&packet);
            back.extend_from_slice(&payload);
            if back.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        assert_eq!(
            String::from_utf8_lossy(&back),
            "HTTP/1.1 204 No Content\r\n\r\n"
        );
    }

    /// Both at once, which is what a device will hand over: a phone with a
    /// dual-stack network uses whichever the name resolved to, flow by flow.
    #[tokio::test]
    async fn the_two_families_are_told_apart_and_not_mixed_up() {
        let mut running = start(8);
        let mut four = Client::new("10.0.0.7:51006", "140.82.121.4:443");
        let mut six = Client::new("[2001:db8::7]:51006", "[2606:50c0:8000::153]:443");

        let a = running.connect(&mut four).await;
        let b = running.connect(&mut six).await;
        assert_eq!(a.to, Target::Addr(four.to));
        assert_eq!(b.to, Target::Addr(six.to));
        assert!(a.client.is_ipv4() && b.client.is_ipv6());

        // The same client port on each: if the two were sorted by port alone
        // they would be one connection, and one of these streams would carry
        // the other's bytes.
        let mut a_stream = a.stream;
        let mut b_stream = b.stream;
        running
            .into
            .send(four.send(TcpControl::Psh, b"four"))
            .await
            .unwrap();
        running
            .into
            .send(six.send(TcpControl::Psh, b"six"))
            .await
            .unwrap();

        let mut got_four = [0u8; 4];
        tokio::time::timeout(Duration::from_secs(5), a_stream.read_exact(&mut got_four))
            .await
            .expect("the IPv4 client's bytes")
            .unwrap();
        let mut got_six = [0u8; 3];
        tokio::time::timeout(Duration::from_secs(5), b_stream.read_exact(&mut got_six))
            .await
            .expect("the IPv6 client's bytes")
            .unwrap();
        assert_eq!(&got_four, b"four");
        assert_eq!(&got_six, b"six");
    }
}
