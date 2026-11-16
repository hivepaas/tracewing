//! Linux: the socket's inode, asked of the kernel by its addresses, then the
//! process with that inode among its open files.
//!
//! `/proc/net/tcp` has the inodes too, but it is read a page at a time while
//! connections come and go, and a busy machine's can leave some out. So it is
//! only where the kernel is not asked directly.

use crate::apps::{Look, canonical};
use std::{
    collections::HashMap,
    fs,
    mem::size_of,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

pub fn owners(looks: &[Look], ours: &[u16], recent: &[u32]) -> Vec<Option<u32>> {
    let mut found = vec![None; looks.len()];
    // Another user's files cannot be read, so their sockets are not looked for.
    // SAFETY: geteuid cannot fail.
    let me = unsafe { libc::geteuid() };
    let mine = |uid: u32| uid == me || me == 0;
    let mut inodes: HashMap<u64, usize> = HashMap::new();
    let mut missing: HashMap<Look, usize> = HashMap::new();
    let diag = Diag::open();
    for (i, look) in looks.iter().enumerate() {
        let look = look.plain();
        match diag.as_ref().and_then(|diag| diag.find(look, ours)) {
            Some((uid, inode)) if mine(uid) => {
                inodes.insert(inode, i);
            }
            Some(_) => {}
            None => {
                missing.insert(look, i);
            }
        }
    }
    if !missing.is_empty() {
        for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
            let Ok(text) = fs::read_to_string(table) else {
                continue;
            };
            inodes.extend(in_table(&text, ours, &missing, &mine));
        }
    }
    if inodes.is_empty() {
        return found;
    }
    let mut all: Vec<u32> = fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
        .filter(|pid| !recent.contains(pid))
        .collect();
    // The newest first: what was just started is what is being looked at.
    all.sort_unstable_by(|a, b| b.cmp(a));
    let pids = recent.iter().copied().chain(all);
    for pid in pids {
        let Ok(fds) = fs::read_dir(format!("/proc/{pid}/fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            let Ok(target) = fs::read_link(fd.path()) else {
                continue;
            };
            let inode = target
                .to_str()
                .and_then(|t| t.strip_prefix("socket:["))
                .and_then(|t| t.strip_suffix(']'))
                .and_then(|t| t.parse::<u64>().ok());
            if let Some(i) = inode.and_then(|inode| inodes.remove(&inode)) {
                found[i] = Some(pid);
                if inodes.is_empty() {
                    return found;
                }
            }
        }
    }
    found
}

/// `linux/netlink.h`, `linux/sock_diag.h` and `linux/inet_diag.h`.
const NETLINK_SOCK_DIAG: i32 = 4;
const SOCK_DIAG_BY_FAMILY: u16 = 20;
const NLM_F_REQUEST: u16 = 1;
const NLMSG_ERROR: u16 = 2;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NlMsgHdr {
    len: u32,
    kind: u16,
    flags: u16,
    seq: u32,
    pid: u32,
}

/// Addresses in network order, an IPv4 one in the first four bytes.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InetDiagSockId {
    sport: u16,
    dport: u16,
    src: [u8; 16],
    dst: [u8; 16],
    interface: u32,
    cookie: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InetDiagReqV2 {
    family: u8,
    protocol: u8,
    ext: u8,
    pad: u8,
    states: u32,
    id: InetDiagSockId,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Request {
    header: NlMsgHdr,
    body: InetDiagReqV2,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InetDiagMsg {
    family: u8,
    state: u8,
    timer: u8,
    retrans: u8,
    id: InetDiagSockId,
    expires: u32,
    rqueue: u32,
    wqueue: u32,
    uid: u32,
    inode: u32,
}

/// The kernel's socket diagnostics, which find one socket by its addresses.
struct Diag {
    socket: OwnedFd,
    seq: std::cell::Cell<u32>,
}

impl Diag {
    fn open() -> Option<Self> {
        // SAFETY: a plain socket call; the descriptor is owned from here.
        let fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
                NETLINK_SOCK_DIAG,
            )
        };
        if fd < 0 {
            return None;
        }
        // SAFETY: just made, and no one else has it.
        let socket = unsafe { OwnedFd::from_raw_fd(fd) };
        let timeout = libc::timeval {
            tv_sec: 1,
            tv_usec: 0,
        };
        // SAFETY: the option is a timeval, as given.
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                (&raw const timeout).cast(),
                size_of::<libc::timeval>() as u32,
            );
        }
        Some(Self {
            socket,
            seq: 0.into(),
        })
    }

    /// The owner and inode of the client's socket, whichever of our ports it
    /// came in on. One ask names one port, so a client that came in on the
    /// first costs one and the rest cost one more each: the proxy port is
    /// first because that is where most of them come from.
    fn find(&self, look: Look, ours: &[u16]) -> Option<(u32, u64)> {
        match look.to {
            // Taken on its way: the four addresses are known, so one ask does.
            Some(to) => self.find_at(look.client, to),
            // A client of tracewing's connected to an address of this machine,
            // and the kernel gives such a connection that address as its own,
            // so both ends have the client's -- which port of ours it chose is
            // what is not known, and is asked for one at a time.
            None => ours.iter().find_map(|&port| {
                self.find_at(look.client, SocketAddr::new(look.client.ip(), port))
            }),
        }
    }

    /// The same, for one pair of ends.
    fn find_at(&self, client: SocketAddr, to: SocketAddr) -> Option<(u32, u64)> {
        match (client.ip(), to.ip()) {
            (IpAddr::V4(from), IpAddr::V4(at)) => self
                .ask(
                    libc::AF_INET as u8,
                    &from.octets(),
                    &at.octets(),
                    client.port(),
                    to.port(),
                )
                .or_else(|| {
                    // A socket for both, which holds them as IPv6.
                    self.ask(
                        libc::AF_INET6 as u8,
                        &from.to_ipv6_mapped().octets(),
                        &at.to_ipv6_mapped().octets(),
                        client.port(),
                        to.port(),
                    )
                }),
            (IpAddr::V6(from), IpAddr::V6(at)) => self.ask(
                libc::AF_INET6 as u8,
                &from.octets(),
                &at.octets(),
                client.port(),
                to.port(),
            ),
            // One end of each family is no connection the kernel holds.
            _ => None,
        }
    }

    fn ask(
        &self,
        family: u8,
        from: &[u8],
        at: &[u8],
        port: u16,
        far_port: u16,
    ) -> Option<(u32, u64)> {
        let seq = self.seq.get().wrapping_add(1);
        self.seq.set(seq);
        let mut id = InetDiagSockId {
            sport: port.to_be(),
            dport: far_port.to_be(),
            // No cookie: find it by its addresses.
            cookie: [u32::MAX; 2],
            ..InetDiagSockId::default()
        };
        id.src[..from.len()].copy_from_slice(from);
        id.dst[..at.len()].copy_from_slice(at);
        let request = Request {
            header: NlMsgHdr {
                len: size_of::<Request>() as u32,
                kind: SOCK_DIAG_BY_FAMILY,
                flags: NLM_F_REQUEST,
                seq,
                pid: 0,
            },
            body: InetDiagReqV2 {
                family,
                protocol: libc::IPPROTO_TCP as u8,
                states: u32::MAX,
                id,
                ..InetDiagReqV2::default()
            },
        };
        let fd = self.socket.as_raw_fd();
        // SAFETY: the request is plain data of the size given.
        let sent = unsafe { libc::send(fd, (&raw const request).cast(), size_of::<Request>(), 0) };
        if sent != size_of::<Request>() as isize {
            return None;
        }
        let mut reply = [0u64; 128];
        loop {
            // SAFETY: the buffer is as long as it says.
            let got = unsafe { libc::recv(fd, reply.as_mut_ptr().cast(), size_of_val(&reply), 0) };
            if got < size_of::<NlMsgHdr>() as isize {
                return None;
            }
            // SAFETY: aligned for u64, and long enough for a header.
            let header = unsafe { *reply.as_ptr().cast::<NlMsgHdr>() };
            // An answer to an earlier ask that timed out.
            if header.seq != seq {
                continue;
            }
            if header.kind == NLMSG_ERROR {
                return None;
            }
            if header.kind != SOCK_DIAG_BY_FAMILY
                || (got as usize) < size_of::<NlMsgHdr>() + size_of::<InetDiagMsg>()
            {
                return None;
            }
            // SAFETY: the message follows its header, which is 16 bytes, and the
            // reply is long enough for it.
            let message = unsafe {
                *reply
                    .as_ptr()
                    .cast::<u8>()
                    .add(size_of::<NlMsgHdr>())
                    .cast::<InetDiagMsg>()
            };
            return Some((message.uid, u64::from(message.inode)));
        }
    }
}

pub fn path_of(pid: u32) -> Option<String> {
    let path = fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    let path = path.to_string_lossy();
    // A program replaced since it started, as by an update.
    Some(path.strip_suffix(" (deleted)").unwrap_or(&path).to_owned())
}

#[derive(Debug, PartialEq, Eq)]
struct Socket {
    local: SocketAddr,
    remote: SocketAddr,
    uid: u32,
    inode: u64,
}

impl Socket {
    /// `sl local_address rem_address st tx_queue:rx_queue tr:tm->when retrnsmt uid timeout inode`
    fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (local, remote) = (fields.get(1)?, fields.get(2)?);
        Some(Self {
            local: address(local)?,
            remote: address(remote)?,
            uid: fields.get(7)?.parse().ok()?,
            inode: fields.get(9)?.parse().ok()?,
        })
    }
}

/// `0100007F:1F90`: the address as the kernel holds it, word by word in this
/// machine's byte order, then the port.
fn address(text: &str) -> Option<SocketAddr> {
    let (ip, port) = text.split_once(':')?;
    let port = u16::from_str_radix(port, 16).ok()?;
    let mut bytes = Vec::with_capacity(16);
    for word in (0..ip.len()).step_by(8) {
        let word = u32::from_str_radix(ip.get(word..word + 8)?, 16).ok()?;
        bytes.extend_from_slice(&word.to_ne_bytes());
    }
    let ip = match bytes.len() {
        4 => IpAddr::V4(Ipv4Addr::from(<[u8; 4]>::try_from(bytes).ok()?)),
        16 => IpAddr::V6(Ipv6Addr::from(<[u8; 16]>::try_from(bytes).ok()?)),
        _ => return None,
    };
    Some(SocketAddr::new(ip, port))
}

/// Which ask each line of `/proc/net/tcp` answers, as `(inode, ask)`.
///
/// Apart from the reading on purpose. This is the path taken only where
/// `sock_diag` is missing or answered nothing, so on a machine that has it --
/// every machine the tests run on -- nothing would otherwise go through here
/// at all, and a mistake in it would sit unseen until it was the only path
/// left. Disabling the first of the two rules below passed all ten tests
/// before this existed.
fn in_table(
    text: &str,
    ours: &[u16],
    missing: &HashMap<Look, usize>,
    mine: &impl Fn(u32) -> bool,
) -> Vec<(u64, usize)> {
    let mut found = Vec::new();
    for line in text.lines().skip(1) {
        let Some(socket) = Socket::parse(line) else {
            continue;
        };
        if !mine(socket.uid) {
            continue;
        }
        let client = canonical(socket.local);
        // Taken on its way: all four addresses say which it is.
        if let Some(&i) = missing.get(&Look {
            client,
            to: Some(canonical(socket.remote)),
        }) {
            found.push((socket.inode, i));
            continue;
        }
        // A client of tracewing's: its own end, among the sockets whose far
        // end is one of our ports.
        if ours.contains(&socket.remote.port())
            && let Some(&i) = missing.get(&Look { client, to: None })
        {
            found.push((socket.inode, i));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A client of tracewing's, asked for by its own end: which port of ours
    /// it chose is what is not known.
    fn asking(client: SocketAddr) -> Look {
        Look { client, to: None }
    }

    #[test]
    fn the_kernel_finds_a_socket_by_its_addresses() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let diag = Diag::open().expect("sock_diag");
        let (uid, inode) = diag
            .find(asking(client.local_addr().unwrap()), &[port])
            .unwrap();
        // SAFETY: geteuid cannot fail.
        assert_eq!(uid, unsafe { libc::geteuid() });
        let fd = format!("/proc/self/fd/{}", client.as_raw_fd());
        assert_eq!(
            fs::read_link(fd).unwrap().to_str().unwrap(),
            format!("socket:[{inode}]")
        );
        assert_eq!(
            diag.find(asking("127.0.0.1:9".parse().unwrap()), &[port]),
            None
        );
        // A second ask on the same socket works as the first.
        assert!(
            diag.find(asking(client.local_addr().unwrap()), &[port])
                .is_some()
        );
        // One of several ports: the ones before it are asked and found nothing.
        let elsewhere = port.wrapping_add(1).max(1);
        assert!(
            diag.find(asking(client.local_addr().unwrap()), &[elsewhere, port])
                .is_some()
        );
    }

    /// A connection taken on its way: the far end is the real server, so the
    /// ask carries it and no list of tracewing's ports is consulted at all.
    #[test]
    fn the_kernel_finds_a_taken_socket_by_all_four_addresses() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let to = listener.local_addr().unwrap();
        let client = std::net::TcpStream::connect(to).unwrap();
        let from = client.local_addr().unwrap();
        let diag = Diag::open().expect("sock_diag");
        let taken = Look {
            client: from,
            to: Some(to),
        };
        // No port of ours is given, and it is still found.
        assert!(diag.find(taken, &[]).is_some());
        // The far end is part of the question: another one is another socket.
        let elsewhere = SocketAddr::new(to.ip(), to.port().wrapping_add(1).max(1));
        assert_eq!(
            diag.find(
                Look {
                    client: from,
                    to: Some(elsewhere)
                },
                &[]
            ),
            None
        );
        // That the rule for a client of ours cannot find it is not assertable
        // here and the attempt passed for the wrong reason: a loopback client
        // has the server's address as its own, so `asking(from)` with the
        // server's port builds exactly this connection's four addresses and
        // finds it. The distinction needs a destination that is not this
        // machine, which no unit test can connect to -- so it is asserted one
        // layer up in `apps`, where what stands in for it is real: tracewing's
        // ports are a list, and a server's port is not on it.
    }

    /// The two rules over one table, where `sock_diag` is not there to ask.
    #[test]
    #[cfg(target_endian = "little")]
    fn the_table_answers_both_kinds_of_ask() {
        // 0100007F is 127.0.0.1 and 097100CB is 203.0.113.9, each word in this
        // machine's own byte order, as the kernel writes them.
        let rest = "01 00000000:00000000 00:00000000 00000000";
        let text = format!(
            "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
             0: 0100007F:D431 097100CB:01BB {rest}  1000 0 4242 1\n\
             1: 0100007F:D432 0100007F:1F90 {rest}  1000 0 4243 1\n\
             2: 0100007F:D433 097100CB:0050 {rest}  1000 0 4244 1\n\
             3: 0100007F:D434 097100CB:01BB {rest}   999 0 4245 1\n\
             4: 0100007F:D432 097100CB:01BB {rest}  1000 0 4246 1\n"
        );
        let taken = |port: u16| Look {
            client: SocketAddr::new("127.0.0.1".parse().unwrap(), port),
            to: Some("203.0.113.9:443".parse().unwrap()),
        };
        let missing = HashMap::from([
            // Taken on its way, and this user's.
            (taken(54321), 0),
            // A client of tracewing's, which came in on 8080.
            (asking("127.0.0.1:54322".parse().unwrap()), 1),
            // Taken on its way, and another user's: not ours to look in.
            (taken(54324), 2),
        ]);
        let mine = |uid: u32| uid == 1000;
        assert_eq!(
            in_table(&text, &[8080], &missing, &mine),
            vec![(4242, 0), (4243, 1)]
        );
        // Line 2 is a socket nobody asked about; line 3 is another user's;
        // and line 4 has the client of ask 1 at its near end with a server at
        // its far end, so the rule for a client of ours must not take it --
        // which is what `ours` narrowing is for.
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn the_kernels_lines_read() {
        let line = "   3: 0100007F:D431 0100007F:1F90 01 00000000:00000000 00:00000000 00000000  1000        0 4242 1 0000000000000000 20 4 30 10 -1";
        assert_eq!(
            Socket::parse(line),
            Some(Socket {
                local: "127.0.0.1:54321".parse().unwrap(),
                remote: "127.0.0.1:8080".parse().unwrap(),
                uid: 1000,
                inode: 4242,
            })
        );
        let six = "   0: 0000000000000000FFFF00000100007F:D431 00000000000000000000000001000000:1F90 01 00000000:00000000 00:00000000 00000000  1000        0 99 1";
        let socket = Socket::parse(six).unwrap();
        assert_eq!(socket.local, "[::ffff:127.0.0.1]:54321".parse().unwrap());
        assert_eq!(canonical(socket.local), "127.0.0.1:54321".parse().unwrap());
        assert_eq!(socket.inode, 99);
        assert_eq!(Socket::parse("  sl  local_address rem_address"), None);
    }
}
