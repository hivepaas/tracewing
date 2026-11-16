//! Windows: the TCP table says which process holds each connection.
//!
//! It is read whole and asked twice over, because there are two questions.
//! A client that asked for the proxy is found by **the end on this machine**,
//! among the rows whose far end is a port of tracewing's. A connection taken
//! off an adapter has the real server at its far end, so that narrowing finds
//! nothing and it is found by **all four addresses** instead -- which is what
//! identifies a TCP connection anyway, and what a source of packets has in
//! hand. See [`Look`].

use crate::apps::Look;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, NO_ERROR},
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
    },
    Networking::WinSock::{AF_INET, AF_INET6},
    System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    },
};

pub fn owners(looks: &[Look], ours: &[u16], _recent: &[u32]) -> Vec<Option<u32>> {
    // A connection to tracewing, by the end on this machine: its far end is a
    // port of ours, which is what tells it from everything else the machine
    // has open.
    let mut to_ours: HashMap<SocketAddr, u32> = HashMap::new();
    // A connection taken on its way, by all four addresses. Only built when
    // something asks for one, because this one is as long as the table.
    let mut whole: HashMap<(SocketAddr, SocketAddr), u32> = HashMap::new();
    let any_taken = looks.iter().any(|look| look.to.is_some());
    let mut keep = |local: SocketAddr, remote: SocketAddr, pid: u32| {
        if ours.contains(&remote.port()) {
            to_ours.insert(local, pid);
        }
        if any_taken {
            whole.insert((local, remote), pid);
        }
    };
    for row in table::<MIB_TCPROW_OWNER_PID>(AF_INET) {
        let at = |addr: u32, p: u32| {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::from(addr.to_ne_bytes())), port(p))
        };
        keep(
            at(row.dwLocalAddr, row.dwLocalPort),
            at(row.dwRemoteAddr, row.dwRemotePort),
            row.dwOwningPid,
        );
    }
    for row in table::<MIB_TCP6ROW_OWNER_PID>(AF_INET6) {
        let at =
            |addr: [u8; 16], p: u32| SocketAddr::new(Ipv6Addr::from(addr).to_canonical(), port(p));
        keep(
            at(row.ucLocalAddr, row.dwLocalPort),
            at(row.ucRemoteAddr, row.dwRemotePort),
            row.dwOwningPid,
        );
    }
    looks
        .iter()
        .map(|look| {
            let look = look.plain();
            match look.to {
                Some(to) => whole.get(&(look.client, to)).copied(),
                None => to_ours.get(&look.client).copied(),
            }
        })
        .collect()
}

pub fn path_of(pid: u32) -> Option<String> {
    // SAFETY: a handle that is closed below, and a buffer as long as it says.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut path = vec![0u16; 32 * 1024];
        let mut len = path.len() as u32;
        let ok =
            QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, path.as_mut_ptr(), &mut len);
        CloseHandle(process);
        (ok != 0).then(|| String::from_utf16_lossy(&path[..len as usize]))
    }
}

/// Ports are in network order in the low half.
fn port(value: u32) -> u16 {
    u16::from_be(value as u16)
}

/// The rows of the table for one family. Each table is a count, then its rows.
fn table<Row: Copy>(family: u16) -> Vec<Row> {
    let mut size = 0u32;
    let mut buffer: Vec<u64> = Vec::new();
    for _ in 0..4 {
        // SAFETY: the buffer is as long as `size` says, and the call says how
        // long it must be when it is not.
        let result = unsafe {
            GetExtendedTcpTable(
                buffer.as_mut_ptr().cast(),
                &mut size,
                0,
                u32::from(family),
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };
        if result == ERROR_INSUFFICIENT_BUFFER {
            // Room for connections opened since.
            size += 64 * size_of::<Row>() as u32;
            buffer = vec![0; (size as usize).div_ceil(8)];
            continue;
        }
        if result != NO_ERROR || buffer.is_empty() {
            return Vec::new();
        }
        // SAFETY: the table starts with its count, and its rows follow at their
        // own alignment, which is no more than 4.
        unsafe {
            let base = buffer.as_ptr().cast::<u8>();
            let count = *base.cast::<u32>() as usize;
            let rows = base.add(4.max(align_of::<Row>())).cast::<Row>();
            let room = (size as usize).saturating_sub(4) / size_of::<Row>();
            return std::slice::from_raw_parts(rows, count.min(room)).to_vec();
        }
    }
    Vec::new()
}
