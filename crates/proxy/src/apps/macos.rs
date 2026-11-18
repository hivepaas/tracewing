//! macOS: each process's sockets through libproc, as `lsof` finds them.

use crate::apps::{Look, canonical};
use std::{
    collections::HashMap,
    ffi::c_void,
    mem::size_of,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

/// `sys/proc_info.h`, which libc does not have.
const PROC_PIDFDSOCKETINFO: i32 = 3;
/// A socket of one of the internet families, which a UDP one is: its part of
/// the union is an `in_sockinfo`, and a TCP socket's starts with one, so both
/// are read at the same offset.
///
/// Measured on macOS 15.8.2: a connected datagram socket is `soi_kind` 1 with
/// `soi_protocol` 17, and its local end is the address its datagrams really
/// carry (`10.0.2.15:49784`, not `0.0.0.0`) -- so here, as on Linux and
/// unlike on Windows, a datagram is answered for by its address.
const SOCKINFO_IN: i32 = 1;
const SOCKINFO_TCP: i32 = 2;
const INI_IPV4: u8 = 0x1;
const INI_IPV6: u8 = 0x2;
const PATH_MAX: usize = 4 * 1024;

// Mirrors of the C structs: not every field is read.
#[allow(dead_code)]
#[repr(C)]
struct ProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: i64,
    fi_type: i32,
    fi_guardflags: u32,
}

// Mirrors of the C structs: not every field is read.
#[allow(dead_code)]
#[repr(C)]
struct VinfoStat {
    vst_dev: u32,
    vst_mode: u16,
    vst_nlink: u16,
    vst_ino: u64,
    vst_uid: u32,
    vst_gid: u32,
    vst_times: [i64; 8],
    vst_size: i64,
    vst_blocks: i64,
    vst_blksize: i32,
    vst_flags: u32,
    vst_gen: u32,
    vst_rdev: u32,
    vst_qspare: [i64; 2],
}

// Mirrors of the C structs: not every field is read.
#[allow(dead_code)]
#[repr(C)]
struct SockbufInfo {
    sbi_cc: u32,
    sbi_hiwat: u32,
    sbi_mbcnt: u32,
    sbi_mbmax: u32,
    sbi_lowat: u32,
    sbi_flags: i16,
    sbi_timeo: i16,
}

/// `in_sockinfo` up to the local address, which is all that is read.
// Mirrors of the C structs: not every field is read.
#[allow(dead_code)]
#[repr(C)]
struct InSockinfo {
    insi_fport: i32,
    insi_lport: i32,
    insi_gencnt: u64,
    insi_flags: u32,
    insi_flow: u32,
    insi_vflag: u8,
    insi_ip_ttl: u8,
    rfu_1: u32,
    insi_faddr: [u8; 16],
    insi_laddr: [u8; 16],
}

/// `socket_fdinfo` up to the start of its protocol's part. The kernel writes
/// all of it, so the buffer is larger than this.
// Mirrors of the C structs: not every field is read.
#[allow(dead_code)]
#[repr(C)]
struct SocketFdInfo {
    pfi: ProcFileInfo,
    soi_stat: VinfoStat,
    soi_so: u64,
    soi_pcb: u64,
    soi_type: i32,
    soi_protocol: i32,
    soi_family: i32,
    soi_options: i16,
    soi_linger: i16,
    soi_state: i16,
    soi_qlen: i16,
    soi_incqlen: i16,
    soi_qlimit: i16,
    soi_timeo: i16,
    soi_error: u16,
    soi_oobmark: u32,
    soi_rcv: SockbufInfo,
    soi_snd: SockbufInfo,
    soi_kind: i32,
    rfu_1: u32,
    tcp: InSockinfo,
}

/// Room for the whole of `socket_fdinfo`, which is about 800 bytes.
type Buffer = [u64; 256];

pub fn owners(looks: &[Look], ours: &[u16], recent: &[u32]) -> Vec<Option<u32>> {
    let mut found = vec![None; looks.len()];
    let mut wanted: HashMap<Look, usize> = HashMap::new();
    let mut datagrams: HashMap<SocketAddr, usize> = HashMap::new();
    for (i, look) in looks.iter().enumerate() {
        match look.plain() {
            Look::Datagram { client } => {
                datagrams.insert(client, i);
            }
            look => {
                wanted.insert(look, i);
            }
        }
    }
    // Every datagram socket passed on the way, because which one answers an
    // ask is not decided until they have all been seen: a socket bound to one
    // address wins over a socket bound to all of them, and the second may be
    // met first. Only gathered when something asks.
    let mut sent: Vec<(u32, SocketAddr)> = Vec::new();
    let all = all_pids();
    let pids = recent
        .iter()
        .copied()
        .chain(all.into_iter().filter(|pid| !recent.contains(pid)));
    let mut buffer: Box<Buffer> = Box::new([0; 256]);
    for pid in pids {
        for fd in socket_fds(pid) {
            match ends(pid, fd, &mut buffer) {
                // Taken on its way: all four addresses say which it is. A
                // client of tracewing's: its own end, among the sockets whose
                // far end is one of our ports.
                Some(Open::Connection { client, remote }) => {
                    let hit = wanted
                        .remove(&Look::Connection {
                            client,
                            to: Some(remote),
                        })
                        .or_else(|| {
                            ours.contains(&remote.port())
                                .then(|| wanted.remove(&Look::Connection { client, to: None }))
                                .flatten()
                        });
                    if let Some(i) = hit {
                        found[i] = Some(pid);
                        if wanted.is_empty() && datagrams.is_empty() {
                            return found;
                        }
                    }
                }
                Some(Open::Datagram { client }) if !datagrams.is_empty() => {
                    sent.push((pid, client));
                }
                _ => {}
            }
        }
    }
    for (client, i) in datagrams {
        found[i] = which_sent(&sent, client);
    }
    found
}

/// Which process sent a datagram from `client`: the one holding the socket
/// bound to that address, or else -- and only else -- one bound to every
/// address on that port. A program that bound `0.0.0.0:443` and one that
/// bound an interface's `:443` are different programs, and the packet says
/// which address it left from.
fn which_sent(sent: &[(u32, SocketAddr)], client: SocketAddr) -> Option<u32> {
    let at = |(_, local): &&(u32, SocketAddr)| *local == client;
    let anywhere = |(_, local): &&(u32, SocketAddr)| {
        local.port() == client.port() && local.ip().is_unspecified()
    };
    let (pid, _) = sent
        .iter()
        .find(at)
        .or_else(|| sent.iter().find(anywhere))?;
    Some(*pid)
}

/// What one of a process's sockets is, as much of it as is asked for.
enum Open {
    /// A connection, by both its ends.
    Connection {
        client: SocketAddr,
        remote: SocketAddr,
    },
    /// A datagram socket, by the end on this machine. Its far end is not
    /// read: a socket that never connected has none, and nothing in a
    /// datagram says which kind sent it. See [`Look`].
    Datagram { client: SocketAddr },
}

pub fn path_of(pid: u32) -> Option<String> {
    let mut path = vec![0u8; PATH_MAX];
    // SAFETY: the buffer is as long as it says.
    let n = unsafe { libc::proc_pidpath(pid as i32, path.as_mut_ptr().cast(), PATH_MAX as u32) };
    if n <= 0 {
        return None;
    }
    path.truncate(n as usize);
    String::from_utf8(path).ok()
}

fn all_pids() -> Vec<u32> {
    // SAFETY: with no buffer it says how many there are.
    let count = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Vec::new();
    }
    // Room for some started since.
    let mut pids = vec![0i32; count as usize + 64];
    let bytes = (pids.len() * size_of::<i32>()) as i32;
    // SAFETY: the buffer is as long as it says.
    let count = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), bytes) };
    pids.truncate(count.max(0) as usize);
    pids.into_iter()
        .filter(|pid| *pid > 0)
        .map(|pid| pid as u32)
        .collect()
}

fn socket_fds(pid: u32) -> Vec<i32> {
    let pid = pid as i32;
    // SAFETY: with no buffer it says how much room the list needs.
    let bytes =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    if bytes <= 0 {
        return Vec::new();
    }
    let room = bytes as usize / size_of::<libc::proc_fdinfo>() + 32;
    let mut fds: Vec<libc::proc_fdinfo> = Vec::with_capacity(room);
    let size = (room * size_of::<libc::proc_fdinfo>()) as i32;
    // SAFETY: the buffer has room for `size` bytes, and the kernel says how much
    // of it it filled.
    let bytes = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            fds.as_mut_ptr().cast::<c_void>(),
            size,
        )
    };
    if bytes <= 0 {
        return Vec::new();
    }
    // SAFETY: the kernel filled this many whole entries.
    unsafe { fds.set_len((bytes as usize / size_of::<libc::proc_fdinfo>()).min(room)) };
    fds.into_iter()
        .filter(|fd| fd.proc_fdtype == libc::PROX_FDTYPE_SOCKET as u32)
        .map(|fd| fd.proc_fd)
        .collect()
}

/// What a socket of a process is, with its addresses as a packet would carry
/// them. Anything that is neither a connection nor a datagram socket of one
/// of the internet families -- a pipe to another program, a kernel control --
/// is `None`.
fn ends(pid: u32, fd: i32, buffer: &mut Buffer) -> Option<Open> {
    let size = size_of::<Buffer>() as i32;
    // SAFETY: the buffer is as long as it says.
    let bytes = unsafe {
        libc::proc_pidfdinfo(
            pid as i32,
            fd,
            PROC_PIDFDSOCKETINFO,
            buffer.as_mut_ptr().cast(),
            size,
        )
    };
    if bytes < size_of::<SocketFdInfo>() as i32 {
        return None;
    }
    // SAFETY: the buffer is aligned for u64 and holds at least this much.
    let info = unsafe { &*buffer.as_ptr().cast::<SocketFdInfo>() };
    let datagram = match info.soi_kind {
        SOCKINFO_TCP => false,
        SOCKINFO_IN if info.soi_protocol == libc::IPPROTO_UDP => true,
        _ => return None,
    };
    let tcp = &info.tcp;
    // Which arm of the sixteen-byte union is live, for both ends: an address
    // is one family or the other, never both, and `insi_vflag` says which. A
    // connection over an IPv6 socket that is really IPv4 -- one reached
    // through a v4-mapped address -- is `INI_IPV4` here, not `INI_IPV6`, and
    // the union's IPv4 arm sits after three words of padding, which is why
    // the four bytes are taken from the end.
    //
    // Measured rather than reasoned about, because this reads the **far** end
    // as well, so a server's IPv6 address goes through it too, and `INI_IPV4`
    // winning over a real v6 address would take four bytes out of its middle
    // and hand back an address that looks perfectly ordinary. Read at these
    // offsets on macOS 15.8.2 (x86_64): a v6 connection is `0b10`, a v4 one
    // `0b01`, and one over an AF_INET6 socket reached through a v4-mapped
    // address is `0b01` as written above. Exactly one of the two in every
    // case, never both.
    let at = |addr: &[u8; 16]| -> Option<IpAddr> {
        if tcp.insi_vflag & INI_IPV4 != 0 {
            let v4: [u8; 4] = addr[12..16].try_into().ok()?;
            Some(IpAddr::V4(Ipv4Addr::from(v4)))
        } else if tcp.insi_vflag & INI_IPV6 != 0 {
            Some(IpAddr::V6(Ipv6Addr::from(*addr)))
        } else {
            None
        }
    };
    // Ports are in network order in the low half. A socket whose family the
    // kernel does not name drops out here, which for a datagram socket means
    // it is left alone rather than taken for one bound to every address:
    // that rule goes by the port alone, so a guess would hand back whatever
    // program happens to hold the same port and look exactly like an answer.
    let client = canonical(SocketAddr::new(
        at(&tcp.insi_laddr)?,
        u16::from_be(tcp.insi_lport as u16),
    ));
    if datagram {
        return Some(Open::Datagram { client });
    }
    Some(Open::Connection {
        client,
        remote: canonical(SocketAddr::new(
            at(&tcp.insi_faddr)?,
            u16::from_be(tcp.insi_fport as u16),
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsRawFd as _;

    /// Which family the kernel names, and so which arm of a sixteen-byte
    /// union this file reads an address out of. It decides the **far** end
    /// as well, so `INI_IPV4` winning over a real v6 address would hand back
    /// four bytes out of its middle and look perfectly ordinary.
    ///
    /// This was a comment saying nobody had a Mac to check it. It is a test
    /// now so that it stays checked: each socket here is one the test opened,
    /// so what it asserts is what the kernel said about a socket whose ends
    /// are already known.
    #[test]
    fn one_family_is_named_and_only_one() {
        let mut buffer: Box<Buffer> = Box::new([0; 256]);
        let me = std::process::id();

        // A pure IPv6 connection: both ends come back as they were given.
        let six = std::net::TcpListener::bind("[::1]:0").unwrap();
        let to = six.local_addr().unwrap();
        let client = std::net::TcpStream::connect(to).unwrap();
        let _accepted = six.accept().unwrap();
        let from = client.local_addr().unwrap();
        match ends(me, client.as_raw_fd(), &mut buffer) {
            Some(Open::Connection { client, remote }) => {
                assert_eq!((client, remote), (from, to), "a v6 connection's ends");
            }
            _ => panic!("a v6 connection is a connection"),
        }

        // A v4 listener reached over an AF_INET6 socket, whose peer is a
        // v4-mapped address: the kernel names that IPv4, and both ends come
        // back as the addresses a packet carries rather than as mapped ones.
        let four = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let plain = four.local_addr().unwrap();
        let mapped: SocketAddr = format!("[::ffff:127.0.0.1]:{}", plain.port())
            .parse()
            .unwrap();
        let client = std::net::TcpStream::connect(mapped).unwrap();
        let _accepted = four.accept().unwrap();
        let from = canonical(client.local_addr().unwrap());
        match ends(me, client.as_raw_fd(), &mut buffer) {
            Some(Open::Connection { client, remote }) => {
                assert_eq!((client, remote), (from, plain), "a v4-mapped connection");
                assert!(client.is_ipv4() && remote.is_ipv4(), "as a packet has them");
            }
            _ => panic!("a v4-mapped connection is a connection"),
        }

        // A datagram socket is the other kind, and its end is its own.
        let datagram = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let at = datagram.local_addr().unwrap();
        match ends(me, datagram.as_raw_fd(), &mut buffer) {
            Some(Open::Datagram { client }) => assert_eq!(client, at),
            _ => panic!("a datagram socket is a datagram socket"),
        }

        // And something that is no socket at all is nothing here, which is
        // what keeps the three above from passing on a reader that answers
        // the same thing whatever it is handed.
        let file = std::fs::File::open("/dev/null").unwrap();
        assert!(
            ends(me, file.as_raw_fd(), &mut buffer).is_none(),
            "a file is not a socket"
        );
    }

    /// Where the header puts things: a wrong offset reads nonsense, not an error.
    #[test]
    fn the_layout_is_the_headers() {
        assert_eq!(size_of::<ProcFileInfo>(), 24);
        assert_eq!(size_of::<VinfoStat>(), 136);
        assert_eq!(size_of::<SockbufInfo>(), 24);
        assert_eq!(std::mem::offset_of!(SocketFdInfo, soi_stat), 24);
        assert_eq!(std::mem::offset_of!(SocketFdInfo, soi_kind), 24 + 232);
        assert_eq!(std::mem::offset_of!(SocketFdInfo, tcp), 24 + 240);
        assert_eq!(std::mem::offset_of!(InSockinfo, insi_laddr), 48);
    }
}
