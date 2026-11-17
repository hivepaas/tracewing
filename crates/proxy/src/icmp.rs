//! `ping` and `tracert`, which a route into an adapter otherwise silences.
//!
//! The stack turns TCP into flows, answers names, and passes other datagrams
//! on. ICMP met none of those and so met nothing: it went to [`smoltcp`],
//! which has no socket for it and drops it. **Measured, not assumed** — an
//! echo request to `8.8.8.8` through the stack produced no packet at all,
//! which was worth checking because the interface has `set_any_ip(true)` and
//! an interface that answers for every destination could as easily have
//! answered the ping itself, as the host being pinged. That would have been
//! far worse than silence: `ping` would report every address on the internet
//! as reachable in under a millisecond.
//!
//! Silence is bad enough. While the mode is on, `ping` and `tracert` are the
//! two things somebody reaches for *when the network looks wrong* — so the
//! mode breaks the tools whose job is to tell them what broke, and it breaks
//! them in the way that looks most like a network fault.
//!
//! So an echo request is **really sent**, and what comes back is turned into
//! a packet the client will accept:
//!
//! - the host answered → an **echo reply** from the address that answered.
//! - a router dropped it because the hop limit ran out → a **time exceeded**
//!   from that router, quoting the request. That quote is the whole of how
//!   `tracert` works: it sends echo requests with a hop limit of 1, 2, 3 …
//!   and reads the address of whoever complains.
//! - nothing answered → nothing, and the client times out, which is what it
//!   would have done on a network where nothing answered.
//!
//! How an echo is really sent is **injected**, for the same reason as
//! [`crate::datagrams`]'s socket: on Windows it has to be IP Helper's
//! `IcmpSendEcho2Ex`, which is what `ping` itself uses and which needs no
//! administrator, with the source address of the interface the machine's own
//! traffic leaves by — or the echo would be routed into tracewing's own
//! adapter and arrive back here. A test gives a table of answers, so
//! everything below it is exercised on Linux.

use crate::source::{Stop, stopped};
use smoltcp::wire::{
    Icmpv4Packet, Icmpv4Repr, Icmpv6Packet, Icmpv6Repr, IpProtocol, Ipv4Packet, Ipv4Repr,
    Ipv6Packet, Ipv6Repr,
};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::mpsc;

/// What carries ICMP, in the `protocol` of an IPv4 header.
const ICMP4: u8 = 1;

/// And in the `next_header` of an IPv6 one. A different number and a
/// different protocol: the checksum covers the addresses, and the message
/// types are its own.
const ICMP6: u8 = 58;

/// The least of an ICMP message: type, code, checksum, and four bytes that
/// mean something different per type. For an echo it is the identifier and
/// the sequence number.
const ICMP_HEAD: usize = 8;

/// How many echoes are waiting to be sent before the rest are dropped. A
/// `ping` sends one at a time and a `tracert` a handful, so a queue this deep
/// is already a client asking for more than it can use.
const QUEUED_MAX: usize = 64;

/// An echo request as it arrived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ping {
    /// The client, which the answer has to go back to.
    pub from: IpAddr,
    /// Where it was going, which is what to really echo.
    pub to: IpAddr,
    pub ident: u16,
    pub seq: u16,
    /// The hop limit it was sent with. `tracert` varies this and reads who
    /// complains, so passing it on is not an optimisation -- it is the whole
    /// feature.
    pub ttl: u8,
    /// The ICMP message whole, as it arrived. Kept because a *time exceeded*
    /// has to quote the request that caused it, and a client matches the
    /// quote rather than trusting the message.
    pub original: Vec<u8>,
}

impl Ping {
    /// The echo's payload, which an answer repeats back.
    fn payload(&self) -> &[u8] {
        &self.original[ICMP_HEAD..]
    }
}

/// What came of really sending it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The host answered, from this address.
    Answered { from: IpAddr, data: Vec<u8> },
    /// A router would not carry it any further, and said so. This is what
    /// `tracert` is reading.
    Expired { from: IpAddr },
    /// Nothing answered in time. The client times out, as it would have on a
    /// network where nothing answered.
    Silent,
}

/// How an echo is really sent. See the module's last paragraph for why this
/// is injected rather than written here.
pub type Echo = Arc<dyn Fn(Ping) -> Answering + Send + Sync>;

pub type Answering = Pin<Box<dyn Future<Output = Answer> + Send>>;

/// The echo request in an IP packet, where it is one.
///
/// `None` for anything that is not an ICMP echo request, and for the two
/// packets [`crate::stack::peek`] leaves alone for the same reason it does:
/// an IPv4 fragment that is not the first and an IPv6 packet with an
/// extension header both put the ICMP header somewhere this does not look.
pub fn echo_request(packet: &[u8]) -> Option<Ping> {
    match *packet.first()? >> 4 {
        4 => {
            let header = usize::from(packet[0] & 0x0f) * 4;
            if header < 20 || packet.len() < header + ICMP_HEAD || packet[9] != ICMP4 {
                return None;
            }
            let fragment = u16::from_be_bytes([packet[6], packet[7]]);
            if fragment & 0x1fff != 0 || fragment & 0x2000 != 0 {
                return None;
            }
            let icmp = Icmpv4Packet::new_checked(&packet[header..]).ok()?;
            let Icmpv4Repr::EchoRequest { ident, seq_no, .. } =
                Icmpv4Repr::parse(&icmp, &Default::default()).ok()?
            else {
                return None;
            };
            Some(Ping {
                from: Ipv4Addr::new(packet[12], packet[13], packet[14], packet[15]).into(),
                to: Ipv4Addr::new(packet[16], packet[17], packet[18], packet[19]).into(),
                ident,
                seq: seq_no,
                ttl: packet[8],
                original: packet[header..].to_vec(),
            })
        }
        6 => {
            const HEAD: usize = 40;
            if packet.len() < HEAD + ICMP_HEAD || packet[6] != ICMP6 {
                return None;
            }
            let from = Ipv6Addr::from(<[u8; 16]>::try_from(&packet[8..24]).ok()?);
            let to = Ipv6Addr::from(<[u8; 16]>::try_from(&packet[24..40]).ok()?);
            let icmp = Icmpv6Packet::new_checked(&packet[HEAD..]).ok()?;
            // The checksum covers the addresses, so parsing needs them.
            let Icmpv6Repr::EchoRequest { ident, seq_no, .. } =
                Icmpv6Repr::parse(&from, &to, &icmp, &Default::default()).ok()?
            else {
                return None;
            };
            Some(Ping {
                from: from.into(),
                to: to.into(),
                ident,
                seq: seq_no,
                // IPv6 calls it the hop limit and puts it in its own place.
                ttl: packet[7],
                original: packet[HEAD..].to_vec(),
            })
        }
        _ => None,
    }
}

/// The packet to send back to the client, for what came of the echo.
///
/// `None` for [`Answer::Silent`], and for an answer from the wrong family --
/// a reply has to look as though it came over the same kind of network the
/// request went out on, or the client will not have it.
pub fn answer_for(ping: &Ping, answer: &Answer) -> Option<Vec<u8>> {
    // The family is decided first and only one message is built. Building
    // both and picking afterwards is what the first version did, and the two
    // `?`s inside the unused one turned every IPv4 answer into `None`.
    let source = match answer {
        Answer::Silent => return None,
        Answer::Answered { from, .. } | Answer::Expired { from } => *from,
    };
    // At least eight bytes of the request, which is what a client matches a
    // complaint against: the type, the identifier and the sequence number.
    //
    // The quoted header's length field is set to what is actually quoted
    // rather than to the request's own length. RFC 792 has a router quote its
    // header **verbatim**, so a real one says 36 bytes while 8 follow -- but
    // nothing reads that field to match a probe, and a header that claims
    // more than it carries is one a strict parser refuses outright
    // (`smoltcp`'s does, which is how this was found). Self-consistent and
    // readable beats verbatim and rejected.
    let quote = &ping.original[..ICMP_HEAD.min(ping.original.len())];
    match (source, ping.from, ping.to) {
        (IpAddr::V4(src), IpAddr::V4(dst), IpAddr::V4(was_going_to)) => {
            let icmp = match answer {
                Answer::Answered { data, .. } => Icmpv4Repr::EchoReply {
                    ident: ping.ident,
                    seq_no: ping.seq,
                    // What the host sent back, where it sent anything; the
                    // request's own payload otherwise, which is what a
                    // client compares against.
                    data: if data.is_empty() {
                        ping.payload()
                    } else {
                        data
                    },
                },
                Answer::Expired { .. } => Icmpv4Repr::TimeExceeded {
                    reason: smoltcp::wire::Icmpv4TimeExceeded::TtlExpired,
                    header: Ipv4Repr {
                        src_addr: dst,
                        dst_addr: was_going_to,
                        next_header: IpProtocol::Icmp,
                        payload_len: quote.len(),
                        hop_limit: ping.ttl,
                    },
                    data: quote,
                },
                Answer::Silent => return None,
            };
            let ip = Ipv4Repr {
                src_addr: src,
                dst_addr: dst,
                next_header: IpProtocol::Icmp,
                payload_len: icmp.buffer_len(),
                hop_limit: 64,
            };
            let head = ip.buffer_len();
            let mut packet = vec![0u8; head + icmp.buffer_len()];
            ip.emit(
                &mut Ipv4Packet::new_unchecked(&mut packet),
                &Default::default(),
            );
            icmp.emit(
                &mut Icmpv4Packet::new_unchecked(&mut packet[head..]),
                &Default::default(),
            );
            Some(packet)
        }
        (IpAddr::V6(src), IpAddr::V6(dst), IpAddr::V6(was_going_to)) => {
            let icmp = match answer {
                Answer::Answered { data, .. } => Icmpv6Repr::EchoReply {
                    ident: ping.ident,
                    seq_no: ping.seq,
                    data: if data.is_empty() {
                        ping.payload()
                    } else {
                        data
                    },
                },
                Answer::Expired { .. } => Icmpv6Repr::TimeExceeded {
                    reason: smoltcp::wire::Icmpv6TimeExceeded::HopLimitExceeded,
                    header: Ipv6Repr {
                        src_addr: dst,
                        dst_addr: was_going_to,
                        next_header: IpProtocol::Icmpv6,
                        payload_len: quote.len(),
                        hop_limit: ping.ttl,
                    },
                    data: quote,
                },
                Answer::Silent => return None,
            };
            let ip = Ipv6Repr {
                src_addr: src,
                dst_addr: dst,
                next_header: IpProtocol::Icmpv6,
                payload_len: icmp.buffer_len(),
                hop_limit: 64,
            };
            let head = ip.buffer_len();
            let mut packet = vec![0u8; head + icmp.buffer_len()];
            ip.emit(&mut Ipv6Packet::new_unchecked(&mut packet));
            icmp.emit(
                &src,
                &dst,
                &mut Icmpv6Packet::new_unchecked(&mut packet[head..]),
                &Default::default(),
            );
            Some(packet)
        }
        // An answer from the other family is no answer to this request.
        _ => None,
    }
}

/// Where to send an echo from, asked per echo rather than once.
///
/// Per echo because the interface the machine leaves by changes as a machine
/// moves between networks, and the watcher that tracks it
/// ([`crate::roads::watch`]) is already the thing that knows. `None` means
/// there is no address to send from, and then nothing is sent: an echo from
/// the wrong interface would be routed into tracewing's own adapter.
#[cfg(any(windows, target_os = "linux"))]
pub type From = std::sync::Arc<dyn Fn(bool) -> Option<IpAddr> + Send + Sync>;

/// Echoes through IP Helper, which is what `ping` itself uses.
///
/// `IcmpSendEcho2Ex` and `Icmp6SendEcho2` take a **source address**, which is
/// the whole reason they are used rather than a raw socket: a raw ICMP socket
/// would need an administrator, and an echo that followed the ordinary routes
/// would be routed into tracewing's own adapter and arrive back here. These
/// need no privilege at all.
///
/// They are synchronous when given no event and no completion routine, so
/// each echo runs on a blocking thread rather than on the runtime.
#[cfg(windows)]
pub fn by_ip_helper(from: From, within: std::time::Duration) -> Echo {
    let within = within.as_millis().min(u128::from(u32::MAX)) as u32;
    std::sync::Arc::new(move |ping: Ping| {
        let from = std::sync::Arc::clone(&from);
        Box::pin(async move {
            let Some(source) = from(ping.to.is_ipv6()) else {
                tracing::debug!("no address to echo {} from", ping.to);
                return Answer::Silent;
            };
            tokio::task::spawn_blocking(move || echo_now(source, &ping, within))
                .await
                .unwrap_or(Answer::Silent)
        })
    })
}

/// One echo, synchronously. Only called on a blocking thread.
#[cfg(windows)]
fn echo_now(source: IpAddr, ping: &Ping, within: u32) -> Answer {
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        ICMP_ECHO_REPLY, ICMPV6_ECHO_REPLY_LH, IP_OPTION_INFORMATION, Icmp6CreateFile,
        Icmp6SendEcho2, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho2Ex,
    };
    /// What the reply buffer must hold, as the API asks: the reply structure,
    /// the data echoed back, and eight bytes for an error's quote.
    fn room(reply: usize, data: usize) -> usize {
        reply + data + 8
    }
    let payload = ping.payload();
    // The hop limit the client chose, which is the whole of `tracert`. Zero
    // would mean "the system default", so a client that sent zero gets one.
    let options = IP_OPTION_INFORMATION {
        Ttl: ping.ttl.max(1),
        ..Default::default()
    };
    match (source, ping.to) {
        (IpAddr::V4(src), IpAddr::V4(dst)) => {
            // SAFETY: a handle closed below on every path.
            let handle = unsafe { IcmpCreateFile() };
            if handle.is_null() || handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                return Answer::Silent;
            }
            let mut reply = vec![0u8; room(size_of::<ICMP_ECHO_REPLY>(), payload.len())];
            // SAFETY: the buffer is as long as it is said to be, the payload
            // outlives the call, and the handle is ours.
            let answers = unsafe {
                IcmpSendEcho2Ex(
                    handle,
                    std::ptr::null_mut(),
                    None,
                    std::ptr::null(),
                    u32::from_ne_bytes(src.octets()),
                    u32::from_ne_bytes(dst.octets()),
                    payload.as_ptr().cast(),
                    payload.len() as u16,
                    &options,
                    reply.as_mut_ptr().cast(),
                    reply.len() as u32,
                    within,
                )
            };
            let answer = if answers == 0 {
                // Nothing answered, or it could not be asked. Either way the
                // client gets the silence it would get from the network.
                Answer::Silent
            } else {
                // SAFETY: the call filled the buffer it was given, and it is
                // at least one reply long.
                let reply = unsafe { reply.as_ptr().cast::<ICMP_ECHO_REPLY>().read_unaligned() };
                answered(
                    reply.Status,
                    Ipv4Addr::from(reply.Address.to_ne_bytes()).into(),
                    // SAFETY: `Data` points into the buffer above, which is
                    // still alive, and `DataSize` is what the call wrote
                    // there. Copied out before the buffer goes.
                    unsafe {
                        std::slice::from_raw_parts(
                            reply.Data.cast::<u8>(),
                            usize::from(reply.DataSize),
                        )
                    }
                    .to_vec(),
                )
            };
            // SAFETY: the handle that call gave us, closed exactly once.
            unsafe { IcmpCloseHandle(handle) };
            answer
        }
        (IpAddr::V6(src), IpAddr::V6(dst)) => {
            // SAFETY: a handle closed below on every path.
            let handle = unsafe { Icmp6CreateFile() };
            if handle.is_null() || handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                return Answer::Silent;
            }
            let mut reply = vec![0u8; room(size_of::<ICMPV6_ECHO_REPLY_LH>(), payload.len())];
            let (src, dst) = (sockaddr6(src), sockaddr6(dst));
            // SAFETY: as above, with both ends as the call wants them.
            let answers = unsafe {
                Icmp6SendEcho2(
                    handle,
                    std::ptr::null_mut(),
                    None,
                    std::ptr::null(),
                    &src,
                    &dst,
                    payload.as_ptr().cast(),
                    payload.len() as u16,
                    &options,
                    reply.as_mut_ptr().cast(),
                    reply.len() as u32,
                    within,
                )
            };
            let answer = if answers == 0 {
                Answer::Silent
            } else {
                // SAFETY: as above.
                let reply = unsafe {
                    reply
                        .as_ptr()
                        .cast::<ICMPV6_ECHO_REPLY_LH>()
                        .read_unaligned()
                };
                answered(
                    reply.Status,
                    // `sin6_addr` is eight `u16`s holding the address in
                    // **network** order, while `Ipv6Addr::from([u16; 8])`
                    // takes segments in host order -- so each one is
                    // swapped. The same trap as `IP_UNICAST_IF`'s, and this
                    // one was caught by the loopback test rather than by
                    // reading: without the swap, `::1` came back as
                    // something else entirely.
                    Ipv6Addr::from(reply.Address.sin6_addr.map(u16::from_be)).into(),
                    // **The IPv6 reply has no data of its own.** The
                    // structure carries an address, a status and a round
                    // trip, and nothing else -- so the payload cannot be
                    // read back here, and `answer_for` repeats the
                    // request's, which is what a host echoes anyway. The
                    // one thing this hides is a host that echoed something
                    // *different*, which `ping` would otherwise report.
                    Vec::new(),
                )
            };
            // SAFETY: the handle that call gave us, closed exactly once.
            unsafe { IcmpCloseHandle(handle) };
            answer
        }
        // One family per echo: the source comes from the destination's.
        _ => Answer::Silent,
    }
}

/// What a reply's status means, which is the same in both families.
#[cfg(windows)]
fn answered(status: u32, from: IpAddr, data: Vec<u8>) -> Answer {
    use windows_sys::Win32::NetworkManagement::IpHelper::{IP_SUCCESS, IP_TTL_EXPIRED_TRANSIT};
    match status {
        IP_SUCCESS => Answer::Answered { from, data },
        // A router would not carry it any further and said so, and `Address`
        // is that router's. This is what `tracert` is reading, and it is why
        // the hop limit above is passed on rather than normalised.
        IP_TTL_EXPIRED_TRANSIT => Answer::Expired { from },
        // Unreachable, filtered, timed out: the client gets the silence it
        // would get from a network that did not answer.
        _ => Answer::Silent,
    }
}

#[cfg(windows)]
fn sockaddr6(addr: Ipv6Addr) -> windows_sys::Win32::Networking::WinSock::SOCKADDR_IN6 {
    use windows_sys::Win32::Networking::WinSock::{AF_INET6, SOCKADDR_IN6};
    let mut out: SOCKADDR_IN6 = unsafe { std::mem::zeroed() };
    out.sin6_family = AF_INET6;
    out.sin6_addr.u.Byte = addr.octets();
    out
}

/// Echoes through a **ping socket**, which is what `ping` itself uses on
/// Linux and which needs no privilege either.
///
/// `socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMP)` is allowed to any process
/// whose group is inside `net.ipv4.ping_group_range`. Measured here with
/// `CapEff: 0000000000000000`: the range is `0 2147483647`, both families
/// open, an echo to `1.1.1.1` comes back, and the payload returns verbatim.
/// A raw ICMP socket would need `CAP_NET_RAW`, which is the privilege this
/// whole mode is shaped to avoid.
///
/// `None` where the machine will not allow one at all: with
/// `ping_group_range` set to `1 0` both families answer **EACCES**, measured
/// by setting it and putting it back. The mode then runs with no echo, which
/// is a feature missing rather than traffic going where it should not -- see
/// [`crate::tun::Tun::answering`].
///
/// ## Two things the kernel does that the caller has to know
///
/// - **The identifier is the kernel's, not ours.** A ping socket rewrites it
///   to the socket's own port and recomputes the checksum: an echo sent with
///   `0xbeef` came back as `3164`. So a reply is matched by its **sequence
///   number**, which is passed through untouched. The answer put back on the
///   wire carries the client's identifier, because [`answer_for`] builds it
///   from the request rather than from the reply.
/// - **A router's complaint arrives on the error queue**, not as a datagram.
///   With `IP_RECVERR` set, `recv` fails with `EHOSTUNREACH` and
///   `recvmsg(MSG_ERRQUEUE)` then carries a `sock_extended_err` and the
///   router's address. Measured with a hop limit of 1 to `1.1.1.1`: origin
///   `SO_EE_ORIGIN_ICMP`, type 11, and `192.168.1.1`, which is this
///   machine's gateway. That is what `tracert` reads.
#[cfg(target_os = "linux")]
pub fn by_ping_socket(from: From, within: std::time::Duration) -> Option<Echo> {
    // A trial open, because the setting is the machine's rather than this
    // echo's: if it is forbidden it is forbidden for every ping, and the
    // caller wants to know before it offers the feature.
    let allowed = ping_socket(false).is_ok() || ping_socket(true).is_ok();
    if !allowed {
        tracing::debug!("no ping socket: net.ipv4.ping_group_range excludes this process");
        return None;
    }
    Some(std::sync::Arc::new(move |ping: Ping| {
        let from = std::sync::Arc::clone(&from);
        Box::pin(async move {
            let Some(source) = from(ping.to.is_ipv6()) else {
                tracing::debug!("no address to echo {} from", ping.to);
                return Answer::Silent;
            };
            tokio::task::spawn_blocking(move || echo_now(source, &ping, within))
                .await
                .unwrap_or(Answer::Silent)
        })
    }))
}

/// A ping socket of one family, owned so that it closes however this returns.
#[cfg(target_os = "linux")]
fn ping_socket(v6: bool) -> std::io::Result<std::os::fd::OwnedFd> {
    use std::os::fd::FromRawFd;
    let (family, proto) = match v6 {
        true => (libc::AF_INET6, libc::IPPROTO_ICMPV6),
        false => (libc::AF_INET, libc::IPPROTO_ICMP),
    };
    // SAFETY: a socket call with constants, which returns a descriptor or -1.
    let fd = unsafe { libc::socket(family, libc::SOCK_DGRAM, proto) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: a descriptor this call has just made and nothing else holds.
    Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) })
}

/// One echo, synchronously. Only called on a blocking thread.
#[cfg(target_os = "linux")]
fn echo_now(source: IpAddr, ping: &Ping, within: std::time::Duration) -> Answer {
    use std::os::fd::AsRawFd;
    let v6 = ping.to.is_ipv6();
    // A source address of one family and a destination of the other is
    // nothing anybody sent, and `from` is asked for the family being echoed.
    if source.is_ipv6() != v6 {
        return Answer::Silent;
    }
    let Ok(socket) = ping_socket(v6) else {
        return Answer::Silent;
    };
    let fd = socket.as_raw_fd();
    let (level, recverr, hops) = match v6 {
        true => (
            libc::IPPROTO_IPV6,
            libc::IPV6_RECVERR,
            libc::IPV6_UNICAST_HOPS,
        ),
        false => (libc::IPPROTO_IP, libc::IP_RECVERR, libc::IP_TTL),
    };
    // The hop limit is passed on rather than normalised: varying it and
    // reading who complains is what a `tracert` **is**.
    let ttl = libc::c_int::from(ping.ttl.max(1));
    let timeout = libc::timeval {
        tv_sec: within.as_secs().min(i64::MAX as u64) as libc::time_t,
        tv_usec: libc::suseconds_t::from(within.subsec_micros()),
    };
    // SAFETY: each value is of the type its option takes, with its own size
    // given, on a socket this function owns.
    let set = unsafe {
        let one = 1 as libc::c_int;
        let int = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
        libc::setsockopt(fd, level, recverr, std::ptr::addr_of!(one).cast(), int)
            | libc::setsockopt(fd, level, hops, std::ptr::addr_of!(ttl).cast(), int)
            | libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                std::ptr::addr_of!(timeout).cast(),
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            )
    };
    if set != 0 {
        return Answer::Silent;
    }
    let (ours, ours_len) = sockaddr(source, 0);
    // SAFETY: a `sockaddr_storage` of ours with the length its family needs.
    if unsafe { libc::bind(fd, std::ptr::addr_of!(ours).cast(), ours_len) } != 0 {
        tracing::debug!("a ping socket would not bind to {source}");
        return Answer::Silent;
    }
    let (theirs, theirs_len) = sockaddr(ping.to, 0);
    // The request goes out as it arrived. The kernel rewrites the identifier
    // and the checksum with it, so there is nothing to put together here.
    // SAFETY: a buffer of ours with its own length, and an address likewise.
    let sent = unsafe {
        libc::sendto(
            fd,
            ping.original.as_ptr().cast(),
            ping.original.len(),
            0,
            std::ptr::addr_of!(theirs).cast(),
            theirs_len,
        )
    };
    if sent < 0 {
        tracing::debug!(
            "a ping to {} would not send: {}",
            ping.to,
            std::io::Error::last_os_error()
        );
        return Answer::Silent;
    }
    let mut reply = [0u8; 1500];
    loop {
        // SAFETY: a buffer of ours with its own length.
        let got = unsafe { libc::recv(fd, reply.as_mut_ptr().cast(), reply.len(), 0) };
        if got >= 0 {
            let reply = &reply[..got as usize];
            // The sequence number, and not the identifier: that one is the
            // kernel's now. An answer to somebody else's probe on the same
            // socket is not ours, so it is read past rather than taken.
            if seq_of(reply) == Some(ping.seq) {
                return Answer::Answered {
                    from: ping.to,
                    data: reply.get(ICMP_HEAD..).unwrap_or_default().to_vec(),
                };
            }
            continue;
        }
        let why = std::io::Error::last_os_error();
        return match why.kind() {
            // Nothing came in time, which is the silence a client would have
            // had from a network that did not answer.
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => Answer::Silent,
            // Something complained, and what it said is on the error queue.
            _ => from_error_queue(fd, ping),
        };
    }
}

/// The sequence number of an ICMP message, which both families keep in the
/// same place.
#[cfg(target_os = "linux")]
fn seq_of(message: &[u8]) -> Option<u16> {
    Some(u16::from_be_bytes([*message.get(6)?, *message.get(7)?]))
}

/// Reads the complaint the kernel queued, and says whether it was a router
/// refusing to carry the probe further.
#[cfg(target_os = "linux")]
fn from_error_queue(fd: libc::c_int, ping: &Ping) -> Answer {
    let mut body = [0u8; 1500];
    let mut control = [0u8; 512];
    let mut who: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    let mut parts = libc::iovec {
        iov_base: body.as_mut_ptr().cast(),
        iov_len: body.len(),
    };
    let mut head: libc::msghdr = unsafe { std::mem::zeroed() };
    head.msg_name = std::ptr::addr_of_mut!(who).cast();
    head.msg_namelen = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    head.msg_iov = std::ptr::addr_of_mut!(parts);
    head.msg_iovlen = 1;
    head.msg_control = control.as_mut_ptr().cast();
    head.msg_controllen = control.len();
    // SAFETY: a message header of ours, pointing at buffers of ours with
    // their own lengths, on a socket the caller owns.
    let got = unsafe { libc::recvmsg(fd, std::ptr::addr_of_mut!(head), libc::MSG_ERRQUEUE) };
    if got < 0 {
        return Answer::Silent;
    }
    // SAFETY: the header the call above filled in.
    let mut cmsg = unsafe { libc::CMSG_FIRSTHDR(std::ptr::addr_of!(head)) };
    while !cmsg.is_null() {
        // SAFETY: a control message the kernel wrote, read for its own
        // length and never past it.
        let (level, kind, bytes) = unsafe {
            let len = (*cmsg).cmsg_len as usize;
            let head_len = libc::CMSG_DATA(cmsg) as usize - cmsg as usize;
            (
                (*cmsg).cmsg_level,
                (*cmsg).cmsg_type,
                std::slice::from_raw_parts(libc::CMSG_DATA(cmsg), len.saturating_sub(head_len)),
            )
        };
        let theirs = match (ping.to.is_ipv6(), level, kind) {
            (false, libc::IPPROTO_IP, libc::IP_RECVERR) => expired(bytes, false),
            (true, libc::IPPROTO_IPV6, libc::IPV6_RECVERR) => expired(bytes, true),
            _ => None,
        };
        if let Some(from) = theirs {
            return Answer::Expired { from };
        }
        // SAFETY: the header and a message that came out of it.
        cmsg = unsafe { libc::CMSG_NXTHDR(std::ptr::addr_of!(head), cmsg) };
    }
    Answer::Silent
}

/// The router that would not carry the probe further, out of the bytes the
/// kernel wrote: a `sock_extended_err` and, after it, the address it came
/// from.
///
/// `None` for every other complaint -- unreachable, administratively
/// filtered, a local error -- which the client is given as silence, the same
/// as a network that said nothing.
///
/// **The fields are in this machine's own byte order**, not the network's,
/// which is the trap here: read big-endian, the measured `ee_errno` of 113
/// comes out as 1895825408.
#[cfg(target_os = "linux")]
fn expired(bytes: &[u8], v6: bool) -> Option<IpAddr> {
    /// `struct sock_extended_err`, the 16 bytes before the offender.
    const HEAD: usize = 16;
    const ORIGIN_ICMP: u8 = 2;
    const ORIGIN_ICMP6: u8 = 3;
    /// ICMP "time exceeded" is 11, ICMPv6's is 3.
    const TIME_EXCEEDED: u8 = 11;
    const TIME_EXCEEDED_V6: u8 = 3;
    let origin = *bytes.get(4)?;
    let kind = *bytes.get(5)?;
    let wanted = match v6 {
        true => (ORIGIN_ICMP6, TIME_EXCEEDED_V6),
        false => (ORIGIN_ICMP, TIME_EXCEEDED),
    };
    if (origin, kind) != wanted {
        return None;
    }
    // The address is a `sockaddr` of its own family, which is read rather
    // than assumed from `v6`: the two have the address at different offsets
    // and a mistake here is a plausible-looking wrong address.
    let family = u16::from_ne_bytes([*bytes.get(HEAD)?, *bytes.get(HEAD + 1)?]);
    match i32::from(family) {
        libc::AF_INET => {
            let octets: [u8; 4] = bytes.get(HEAD + 4..HEAD + 8)?.try_into().ok()?;
            Some(IpAddr::from(octets))
        }
        libc::AF_INET6 => {
            let octets: [u8; 16] = bytes.get(HEAD + 8..HEAD + 24)?.try_into().ok()?;
            Some(IpAddr::from(octets))
        }
        _ => None,
    }
}

/// An address as the kernel takes one, with the length its family needs.
#[cfg(target_os = "linux")]
fn sockaddr(addr: IpAddr, port: u16) -> (libc::sockaddr_storage, libc::socklen_t) {
    // SAFETY: zeroes are a valid `sockaddr_storage`, and the family below
    // says which of its shapes this is.
    let mut out: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    match addr {
        IpAddr::V4(addr) => {
            // SAFETY: writing the `sockaddr_in` shape into storage sized for
            // the largest of them.
            let it = unsafe { &mut *std::ptr::addr_of_mut!(out).cast::<libc::sockaddr_in>() };
            it.sin_family = libc::AF_INET as libc::sa_family_t;
            it.sin_port = port.to_be();
            it.sin_addr.s_addr = u32::from_ne_bytes(addr.octets());
            (
                out,
                std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
            )
        }
        IpAddr::V6(addr) => {
            // SAFETY: the same for `sockaddr_in6`.
            let it = unsafe { &mut *std::ptr::addr_of_mut!(out).cast::<libc::sockaddr_in6>() };
            it.sin6_family = libc::AF_INET6 as libc::sa_family_t;
            it.sin6_port = port.to_be();
            it.sin6_addr.s6_addr = addr.octets();
            (
                out,
                std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
            )
        }
    }
}

/// Starts answering echo requests, and returns where to put them.
///
/// Each one is sent in a task of its own: a `tracert` has several probes out
/// at once and answering them one after another would turn a thirty-hop trace
/// into thirty timeouts in a row.
pub fn answering(out: mpsc::Sender<Vec<u8>>, echo: Echo, stop: Stop) -> mpsc::Sender<Ping> {
    let (asked, mut asking) = mpsc::channel::<Ping>(QUEUED_MAX);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                ping = asking.recv() => match ping {
                    Some(ping) => {
                        let echo = Arc::clone(&echo);
                        let out = out.clone();
                        tokio::spawn(async move {
                            let answer = echo(ping.clone()).await;
                            if let Some(packet) = answer_for(&ping, &answer)
                                && out.send(packet).await.is_err() {
                                    tracing::debug!("nobody left to take a ping's answer");
                                }
                        });
                    }
                    // The stack has let go: nothing more will arrive.
                    None => break,
                },
                _ = stopped(stop.clone()) => break,
            }
        }
    });
    asked
}

#[cfg(test)]
mod tests {

    /// The bytes this machine's kernel really wrote on the error queue, for
    /// a probe sent to `1.1.1.1` with a hop limit of 1. Kept as they came:
    /// `ee_errno` 113 (`EHOSTUNREACH`), origin 2, type 11, then a
    /// `sockaddr_in` holding 192.168.1.1, which is this machine's gateway.
    #[cfg(target_os = "linux")]
    const A_ROUTER_COMPLAINING: &str =
        "71000000020b0000000000000000000002000000c0a801010000000000000000";

    #[cfg(target_os = "linux")]
    fn bytes(hex: &str) -> Vec<u8> {
        (0..hex.len() / 2)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex"))
            .collect()
    }

    /// A real complaint reads as the router that made it.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_router_that_would_not_carry_it_further() {
        let said = bytes(A_ROUTER_COMPLAINING);
        assert_eq!(
            expired(&said, false),
            Some("192.168.1.1".parse::<IpAddr>().unwrap())
        );
        // Asked as though it were IPv6, the same bytes are not a hop limit
        // complaint: origin and type both differ between the families, and
        // taking one for the other would answer a `tracert` with a router
        // that never spoke.
        assert_eq!(expired(&said, true), None);
    }

    /// Everything else a complaint can be is silence, not a wrong router.
    #[cfg(target_os = "linux")]
    #[test]
    fn what_is_not_a_hop_limit_complaint() {
        let mut other_type = bytes(A_ROUTER_COMPLAINING);
        other_type[5] = 3; // destination unreachable, not time exceeded
        assert_eq!(expired(&other_type, false), None);
        let mut local = bytes(A_ROUTER_COMPLAINING);
        local[4] = 1; // SO_EE_ORIGIN_LOCAL: our own machine, not a router
        assert_eq!(expired(&local, false), None);
        // A family nobody sent, and bytes that stop early: both `None`
        // rather than a panic or an address read out of whatever follows.
        let mut family = bytes(A_ROUTER_COMPLAINING);
        family[16] = 99;
        assert_eq!(expired(&family, false), None);
        for cut in [0, 4, 5, 16, 17, 19] {
            assert_eq!(
                expired(&bytes(A_ROUTER_COMPLAINING)[..cut], false),
                None,
                "{cut} bytes"
            );
        }
    }

    /// The sequence number is what a reply is matched by, because the
    /// identifier is the kernel's. These are the bytes of the reply that
    /// came back from `1.1.1.1`.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_reply_is_matched_by_its_sequence() {
        // type 0, code 0, checksum, ident 0x0c5f (the socket's port, not
        // the 0x1234 that was sent), sequence 9.
        let reply = bytes("0000dfdb0c5f0009747261636577696e67");
        assert_eq!(seq_of(&reply), Some(9));
        assert_eq!(seq_of(&[]), None);
        assert_eq!(seq_of(&reply[..7]), None);
    }

    /// Really echoes, on this machine's network: `1.1.1.1` answers, and a
    /// hop limit of 1 is refused by the first router, which is what a
    /// `tracert` reads.
    ///
    /// `cargo test -p tracewing-proxy -- --ignored really_echoes`
    #[cfg(target_os = "linux")]
    #[tokio::test]
    #[ignore = "sends real ICMP and needs a network that answers it"]
    async fn really_echoes_and_hears_a_router() {
        // This machine's own address for the family, found the way anything
        // finds it: by asking the routing table where it would send from.
        let probe = std::net::UdpSocket::bind("0.0.0.0:0").expect("a socket");
        probe.connect("1.1.1.1:53").expect("a route to 1.1.1.1");
        let ours = probe.local_addr().expect("an address").ip();
        let from: From = std::sync::Arc::new(move |v6| (!v6).then_some(ours));
        let echo = by_ping_socket(from, std::time::Duration::from_secs(3))
            .expect("a ping socket on this machine");
        let ping = |ttl: u8, seq: u16| Ping {
            from: "10.77.0.9".parse().unwrap(),
            to: "1.1.1.1".parse().unwrap(),
            ident: 0x1234,
            seq,
            ttl,
            original: {
                let mut message = vec![8u8, 0, 0, 0, 0x12, 0x34, (seq >> 8) as u8, seq as u8];
                message.extend_from_slice(b"tracewing");
                message
            },
        };
        match echo(ping(64, 101)).await {
            Answer::Answered { from, data } => {
                println!("answered from {from}, {} bytes back", data.len());
                assert_eq!(data, b"tracewing", "the payload comes back as it went");
            }
            other => panic!("1.1.1.1 answers an echo: {other:?}"),
        }
        match echo(ping(1, 102)).await {
            Answer::Expired { from } => println!("a hop limit of 1 was refused by {from}"),
            other => panic!("the first router refuses a hop limit of 1: {other:?}"),
        }
    }
    use super::*;

    fn request(from: &str, to: &str, ttl: u8, seq: u16) -> Vec<u8> {
        let (from, to) = (from.parse().unwrap(), to.parse().unwrap());
        let icmp4 = Icmpv4Repr::EchoRequest {
            ident: 0x4321,
            seq_no: seq,
            data: b"abcdefgh",
        };
        let icmp6 = Icmpv6Repr::EchoRequest {
            ident: 0x4321,
            seq_no: seq,
            data: b"abcdefgh",
        };
        match (from, to) {
            (IpAddr::V4(src), IpAddr::V4(dst)) => {
                let ip = Ipv4Repr {
                    src_addr: src,
                    dst_addr: dst,
                    next_header: IpProtocol::Icmp,
                    payload_len: icmp4.buffer_len(),
                    hop_limit: ttl,
                };
                let mut bytes = vec![0u8; ip.buffer_len() + icmp4.buffer_len()];
                let head = ip.buffer_len();
                ip.emit(
                    &mut Ipv4Packet::new_unchecked(&mut bytes),
                    &Default::default(),
                );
                icmp4.emit(
                    &mut Icmpv4Packet::new_unchecked(&mut bytes[head..]),
                    &Default::default(),
                );
                bytes
            }
            (IpAddr::V6(src), IpAddr::V6(dst)) => {
                let ip = Ipv6Repr {
                    src_addr: src,
                    dst_addr: dst,
                    next_header: IpProtocol::Icmpv6,
                    payload_len: icmp6.buffer_len(),
                    hop_limit: ttl,
                };
                let mut bytes = vec![0u8; ip.buffer_len() + icmp6.buffer_len()];
                let head = ip.buffer_len();
                ip.emit(&mut Ipv6Packet::new_unchecked(&mut bytes));
                icmp6.emit(
                    &src,
                    &dst,
                    &mut Icmpv6Packet::new_unchecked(&mut bytes[head..]),
                    &Default::default(),
                );
                bytes
            }
            _ => panic!("one family per request"),
        }
    }

    #[test]
    fn an_echo_request_is_read_in_both_families() {
        let four = echo_request(&request("10.0.0.7", "8.8.8.8", 7, 3)).expect("a ping");
        assert_eq!(four.from, "10.0.0.7".parse::<IpAddr>().unwrap());
        assert_eq!(four.to, "8.8.8.8".parse::<IpAddr>().unwrap());
        assert_eq!((four.ident, four.seq, four.ttl), (0x4321, 3, 7));
        assert_eq!(four.payload(), b"abcdefgh");

        let six = echo_request(&request("2001:db8::7", "2001:db8::9", 5, 4)).expect("a ping");
        assert_eq!(six.to, "2001:db8::9".parse::<IpAddr>().unwrap());
        assert_eq!(
            (six.ident, six.seq, six.ttl),
            (0x4321, 4, 5),
            "IPv6 keeps the hop limit in its own place, not where IPv4 does"
        );
        assert_eq!(six.payload(), b"abcdefgh");
    }

    #[test]
    fn what_is_not_an_echo_request() {
        // An echo *reply*, which is somebody else's business.
        let mut reply = request("10.0.0.7", "8.8.8.8", 7, 1);
        reply[20] = 0; // type 0 is a reply
        assert_eq!(echo_request(&reply), None, "a reply is not a request");
        // TCP.
        let mut tcp = request("10.0.0.7", "8.8.8.8", 7, 1);
        tcp[9] = 6;
        assert_eq!(echo_request(&tcp), None);
        // A fragment that is not the first.
        let mut later = request("10.0.0.7", "8.8.8.8", 7, 1);
        later[6] = 0x00;
        later[7] = 0x01;
        assert_eq!(echo_request(&later), None);
        // An IPv6 packet with an extension header, which moves the ICMP.
        let mut six = request("2001:db8::7", "2001:db8::9", 7, 1);
        six[6] = 0;
        assert_eq!(echo_request(&six), None);
        // Cut short, and nothing at all.
        let whole = request("10.0.0.7", "8.8.8.8", 7, 1);
        assert_eq!(echo_request(&whole[..20]), None);
        assert_eq!(echo_request(&[]), None);
    }

    /// The answer a client gets for a host that answered: a reply from that
    /// host, carrying the identifier and sequence number it sent.
    #[test]
    fn a_host_that_answered_becomes_an_echo_reply() {
        let ping = echo_request(&request("10.0.0.7", "8.8.8.8", 64, 9)).unwrap();
        let packet = answer_for(
            &ping,
            &Answer::Answered {
                from: "8.8.8.8".parse().unwrap(),
                data: b"abcdefgh".to_vec(),
            },
        )
        .expect("a packet");
        let ip = Ipv4Packet::new_checked(&packet[..]).expect("an IPv4 packet");
        assert!(ip.verify_checksum(), "the IPv4 header's checksum");
        assert_eq!(ip.src_addr().octets(), [8, 8, 8, 8], "from the host pinged");
        assert_eq!(ip.dst_addr().octets(), [10, 0, 0, 7], "back to the client");
        let icmp = Icmpv4Packet::new_checked(ip.payload()).expect("an ICMP message");
        assert!(icmp.verify_checksum(), "and ICMP's own");
        match Icmpv4Repr::parse(&icmp, &Default::default()).unwrap() {
            Icmpv4Repr::EchoReply {
                ident,
                seq_no,
                data,
            } => {
                assert_eq!((ident, seq_no), (0x4321, 9), "the client matches on these");
                assert_eq!(data, b"abcdefgh");
            }
            other => panic!("an echo reply, not {other:?}"),
        }
    }

    /// And the one `tracert` lives on: a router that would not carry it,
    /// quoting the request so the client can tell which probe it answers.
    #[test]
    fn a_hop_limit_that_ran_out_becomes_a_time_exceeded_quoting_the_request() {
        let ping = echo_request(&request("10.0.0.7", "8.8.8.8", 3, 11)).unwrap();
        let packet = answer_for(
            &ping,
            &Answer::Expired {
                from: "203.0.113.1".parse().unwrap(),
            },
        )
        .expect("a packet");
        let ip = Ipv4Packet::new_checked(&packet[..]).expect("an IPv4 packet");
        assert_eq!(
            ip.src_addr().octets(),
            [203, 0, 113, 1],
            "from the router that complained, which is the address tracert prints"
        );
        assert_eq!(ip.dst_addr().octets(), [10, 0, 0, 7]);
        let icmp = Icmpv4Packet::new_checked(ip.payload()).expect("an ICMP message");
        assert!(icmp.verify_checksum());
        match Icmpv4Repr::parse(&icmp, &Default::default()).unwrap() {
            Icmpv4Repr::TimeExceeded { header, data, .. } => {
                assert_eq!(
                    header.src_addr.octets(),
                    [10, 0, 0, 7],
                    "the quote is the request"
                );
                assert_eq!(header.dst_addr.octets(), [8, 8, 8, 8]);
                assert_eq!(header.hop_limit, 3, "the hop limit that ran out");
                // The quote carries the identifier and sequence number, which
                // is how the client tells one probe from another.
                assert_eq!(u16::from_be_bytes([data[4], data[5]]), 0x4321);
                assert_eq!(u16::from_be_bytes([data[6], data[7]]), 11);
            }
            other => panic!("a time exceeded, not {other:?}"),
        }
    }

    #[test]
    fn the_same_two_answers_in_ipv6() {
        let ping = echo_request(&request("2001:db8::7", "2001:db8::9", 4, 2)).unwrap();
        for (answer, expect_reply) in [
            (
                Answer::Answered {
                    from: "2001:db8::9".parse().unwrap(),
                    data: b"abcdefgh".to_vec(),
                },
                true,
            ),
            (
                Answer::Expired {
                    from: "2001:db8::1".parse().unwrap(),
                },
                false,
            ),
        ] {
            let packet = answer_for(&ping, &answer).expect("a packet");
            let ip = Ipv6Packet::new_checked(&packet[..]).expect("an IPv6 packet");
            let icmp = Icmpv6Packet::new_checked(ip.payload()).expect("an ICMP message");
            assert!(
                icmp.verify_checksum(&ip.src_addr(), &ip.dst_addr()),
                "ICMPv6's checksum covers the addresses, so it is not optional"
            );
            let repr =
                Icmpv6Repr::parse(&ip.src_addr(), &ip.dst_addr(), &icmp, &Default::default())
                    .unwrap();
            match (repr, expect_reply) {
                (Icmpv6Repr::EchoReply { seq_no, .. }, true) => assert_eq!(seq_no, 2),
                (Icmpv6Repr::TimeExceeded { header, .. }, false) => {
                    assert_eq!(header.hop_limit, 4)
                }
                (other, _) => panic!("the wrong message: {other:?}"),
            }
        }
    }

    #[test]
    fn nothing_answering_is_nothing_sent() {
        let ping = echo_request(&request("10.0.0.7", "8.8.8.8", 64, 1)).unwrap();
        assert_eq!(
            answer_for(&ping, &Answer::Silent),
            None,
            "the client times out, as it would where nothing answered"
        );
    }

    #[test]
    fn an_answer_from_the_other_family_is_no_answer() {
        let ping = echo_request(&request("10.0.0.7", "8.8.8.8", 64, 1)).unwrap();
        assert_eq!(
            answer_for(
                &ping,
                &Answer::Answered {
                    from: "2001:db8::9".parse().unwrap(),
                    data: Vec::new(),
                }
            ),
            None
        );
    }

    /// IP Helper really echoing, against loopback -- which answers its own
    /// echo on any machine, wants no administrator and needs no network, so
    /// it is a test rather than something only a person can try.
    ///
    /// What this does **not** cover is the `Expired` arm: that wants a
    /// router some hops away to complain, which no test can arrange. Its
    /// mapping from `IP_TTL_EXPIRED_TRANSIT` is read from the API's contract
    /// and the packet it becomes is covered above, on Linux.
    #[cfg(windows)]
    #[tokio::test]
    async fn ip_helper_really_echoes_loopback() {
        for (v6, address) in [(false, "127.0.0.1"), (true, "::1")] {
            let here: IpAddr = address.parse().unwrap();
            let echo = by_ip_helper(
                std::sync::Arc::new(move |wants_v6| (wants_v6 == v6).then_some(here)),
                std::time::Duration::from_secs(5),
            );
            let ping = echo_request(&request(address, address, 64, 1)).expect("a ping");
            match echo(ping).await {
                Answer::Answered { from, data } => {
                    assert_eq!(from, here, "loopback answers as itself");
                    if !v6 {
                        assert_eq!(
                            data, b"abcdefgh",
                            "and echoes the payload, which IPv4 reports and IPv6 does not"
                        );
                    }
                }
                other => panic!("loopback should answer its own echo, not {other:?}"),
            }
        }
    }

    /// No address to send from is silence rather than an echo out of the
    /// wrong interface -- which would be routed into tracewing's own adapter.
    #[cfg(windows)]
    #[tokio::test]
    async fn no_source_address_is_silence() {
        let echo = by_ip_helper(
            std::sync::Arc::new(|_| None),
            std::time::Duration::from_secs(1),
        );
        let ping = echo_request(&request("127.0.0.1", "127.0.0.1", 64, 1)).unwrap();
        assert_eq!(echo(ping).await, Answer::Silent);
    }

    /// The whole of it: a request goes in, the echoer is asked, and the
    /// answer comes out as a packet the client would accept.
    #[tokio::test]
    async fn a_request_is_echoed_and_its_answer_comes_back() {
        let (out, mut coming) = mpsc::channel(8);
        let (_stop_tx, stop) = tokio::sync::watch::channel(false);
        let (seen, mut seeing) = mpsc::unbounded_channel();
        let echo: Echo = Arc::new(move |ping: Ping| {
            let seen = seen.clone();
            Box::pin(async move {
                let _ = seen.send(ping.clone());
                // A router at hop three, as a tracert probe would meet.
                match ping.ttl {
                    3 => Answer::Expired {
                        from: "203.0.113.1".parse().unwrap(),
                    },
                    _ => Answer::Answered {
                        from: ping.to,
                        data: b"abcdefgh".to_vec(),
                    },
                }
            })
        });
        let asked = answering(out, echo, stop);

        for ttl in [64u8, 3] {
            asked
                .send(echo_request(&request("10.0.0.7", "8.8.8.8", ttl, u16::from(ttl))).unwrap())
                .await
                .unwrap();
            let ping = tokio::time::timeout(std::time::Duration::from_secs(5), seeing.recv())
                .await
                .expect("the echoer is asked")
                .expect("one ask");
            assert_eq!(
                ping.ttl, ttl,
                "the hop limit is passed on, which is tracert"
            );
            let packet = tokio::time::timeout(std::time::Duration::from_secs(5), coming.recv())
                .await
                .expect("an answer comes back")
                .expect("a packet");
            let ip = Ipv4Packet::new_checked(&packet[..]).unwrap();
            let icmp = Icmpv4Packet::new_checked(ip.payload()).unwrap();
            let repr = Icmpv4Repr::parse(&icmp, &Default::default()).unwrap();
            match (ttl, repr) {
                (64, Icmpv4Repr::EchoReply { .. }) => {}
                (3, Icmpv4Repr::TimeExceeded { .. }) => {}
                (_, other) => panic!("the wrong answer for ttl {ttl}: {other:?}"),
            }
        }
    }
}
