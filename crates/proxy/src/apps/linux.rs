//! Linux: the socket's inode, asked of the kernel by its addresses, then the
//! process with that inode among its open files.
//!
//! `/proc/net/tcp` has the inodes too, but it is read a page at a time while
//! connections come and go, and a busy machine's can leave some out. So it is
//! only where the kernel is not asked directly.

use std::{
    collections::HashMap,
    fs,
    mem::size_of,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

pub fn owners(clients: &[SocketAddr], ours: &[u16], recent: &[u32]) -> Vec<Option<u32>> {
    let mut found = vec![None; clients.len()];
    // Another user's files cannot be read, so their sockets are not looked for.
    // SAFETY: geteuid cannot fail.
    let me = unsafe { libc::geteuid() };
    let mine = |uid: u32| uid == me || me == 0;
    let mut inodes: HashMap<u64, usize> = HashMap::new();
    let mut missing: HashMap<SocketAddr, usize> = HashMap::new();
    let diag = Diag::open();
    for (i, client) in clients.iter().enumerate() {
        let client = canonical(*client);
        match diag.as_ref().and_then(|diag| diag.find(client, ours)) {
            Some((uid, inode)) if mine(uid) => {
                inodes.insert(inode, i);
            }
            Some(_) => {}
            None => {
                missing.insert(client, i);
            }
        }
    }
    if !missing.is_empty() {
        for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
            let Ok(text) = fs::read_to_string(table) else {
                continue;
            };
            for line in text.lines().skip(1) {
                let Some(socket) = Socket::parse(line) else {
                    continue;
                };
                if !ours.contains(&socket.remote_port) || !mine(socket.uid) {
                    continue;
                }
                if let Some(&i) = missing.get(&canonical(socket.local)) {
                    inodes.insert(socket.inode, i);
                }
            }
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
    fn find(&self, client: SocketAddr, ours: &[u16]) -> Option<(u32, u64)> {
        ours.iter().find_map(|&port| self.find_on(client, port))
    }

    /// The same, on one port. It connected to an address of this machine, and
    /// the kernel gives such a connection that address as its own, so both
    /// ends have the client's.
    fn find_on(&self, client: SocketAddr, proxy_port: u16) -> Option<(u32, u64)> {
        match client.ip() {
            IpAddr::V4(ip) => self
                .ask(libc::AF_INET as u8, &ip.octets(), client.port(), proxy_port)
                .or_else(|| {
                    // A socket for both, which holds it as IPv6.
                    let mapped = ip.to_ipv6_mapped().octets();
                    self.ask(libc::AF_INET6 as u8, &mapped, client.port(), proxy_port)
                }),
            IpAddr::V6(ip) => self.ask(
                libc::AF_INET6 as u8,
                &ip.octets(),
                client.port(),
                proxy_port,
            ),
        }
    }

    fn ask(&self, family: u8, ip: &[u8], port: u16, proxy_port: u16) -> Option<(u32, u64)> {
        let seq = self.seq.get().wrapping_add(1);
        self.seq.set(seq);
        let mut id = InetDiagSockId {
            sport: port.to_be(),
            dport: proxy_port.to_be(),
            // No cookie: find it by its addresses.
            cookie: [u32::MAX; 2],
            ..InetDiagSockId::default()
        };
        id.src[..ip.len()].copy_from_slice(ip);
        id.dst[..ip.len()].copy_from_slice(ip);
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

fn canonical(addr: SocketAddr) -> SocketAddr {
    SocketAddr::new(addr.ip().to_canonical(), addr.port())
}

#[derive(Debug, PartialEq, Eq)]
struct Socket {
    local: SocketAddr,
    remote_port: u16,
    uid: u32,
    inode: u64,
}

impl Socket {
    /// `sl local_address rem_address st tx_queue:rx_queue tr:tm->when retrnsmt uid timeout inode`
    fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (local, remote) = (fields.get(1)?, fields.get(2)?);
        let (_, remote_port) = remote.split_once(':')?;
        Some(Self {
            local: address(local)?,
            remote_port: u16::from_str_radix(remote_port, 16).ok()?,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kernel_finds_a_socket_by_its_addresses() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        let diag = Diag::open().expect("sock_diag");
        let (uid, inode) = diag.find(client.local_addr().unwrap(), &[port]).unwrap();
        // SAFETY: geteuid cannot fail.
        assert_eq!(uid, unsafe { libc::geteuid() });
        let fd = format!("/proc/self/fd/{}", client.as_raw_fd());
        assert_eq!(
            fs::read_link(fd).unwrap().to_str().unwrap(),
            format!("socket:[{inode}]")
        );
        assert_eq!(diag.find("127.0.0.1:9".parse().unwrap(), &[port]), None);
        // A second ask on the same socket works as the first.
        assert!(diag.find(client.local_addr().unwrap(), &[port]).is_some());
        // One of several ports: the ones before it are asked and found nothing.
        let elsewhere = port.wrapping_add(1).max(1);
        assert!(
            diag.find(client.local_addr().unwrap(), &[elsewhere, port])
                .is_some()
        );
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn the_kernels_lines_read() {
        let line = "   3: 0100007F:D431 0100007F:1F90 01 00000000:00000000 00:00000000 00000000  1000        0 4242 1 0000000000000000 20 4 30 10 -1";
        assert_eq!(
            Socket::parse(line),
            Some(Socket {
                local: "127.0.0.1:54321".parse().unwrap(),
                remote_port: 8080,
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
