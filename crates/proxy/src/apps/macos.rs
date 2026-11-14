//! macOS: each process's sockets through libproc, as `lsof` finds them.

use std::{
    collections::HashMap,
    ffi::c_void,
    mem::size_of,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

/// `sys/proc_info.h`, which libc does not have.
const PROC_PIDFDSOCKETINFO: i32 = 3;
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

pub fn owners(clients: &[SocketAddr], ours: &[u16], recent: &[u32]) -> Vec<Option<u32>> {
    let mut found = vec![None; clients.len()];
    let mut wanted: HashMap<SocketAddr, usize> = clients
        .iter()
        .enumerate()
        .map(|(i, client)| (canonical(*client), i))
        .collect();
    let all = all_pids();
    let pids = recent
        .iter()
        .copied()
        .chain(all.into_iter().filter(|pid| !recent.contains(pid)));
    let mut buffer: Box<Buffer> = Box::new([0; 256]);
    for pid in pids {
        for fd in socket_fds(pid) {
            let Some(local) = tcp_local(pid, fd, ours, &mut buffer) else {
                continue;
            };
            if let Some(i) = wanted.remove(&local) {
                found[i] = Some(pid);
                if wanted.is_empty() {
                    return found;
                }
            }
        }
    }
    found
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

fn canonical(addr: SocketAddr) -> SocketAddr {
    SocketAddr::new(addr.ip().to_canonical(), addr.port())
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

/// The local address of a TCP socket connected to one of tracewing's ports.
fn tcp_local(pid: u32, fd: i32, ours: &[u16], buffer: &mut Buffer) -> Option<SocketAddr> {
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
    if info.soi_kind != SOCKINFO_TCP {
        return None;
    }
    let tcp = &info.tcp;
    // Ports are in network order in the low half.
    if !ours.contains(&u16::from_be(tcp.insi_fport as u16)) {
        return None;
    }
    let ip = if tcp.insi_vflag & INI_IPV4 != 0 {
        let v4: [u8; 4] = tcp.insi_laddr[12..16].try_into().ok()?;
        IpAddr::V4(Ipv4Addr::from(v4))
    } else if tcp.insi_vflag & INI_IPV6 != 0 {
        IpAddr::V6(Ipv6Addr::from(tcp.insi_laddr))
    } else {
        return None;
    };
    Some(canonical(SocketAddr::new(
        ip,
        u16::from_be(tcp.insi_lport as u16),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

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
