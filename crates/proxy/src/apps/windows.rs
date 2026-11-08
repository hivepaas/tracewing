//! Windows: the TCP table says which process holds each connection.

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

pub fn owners(clients: &[SocketAddr], proxy_port: u16, _recent: &[u32]) -> Vec<Option<u32>> {
    let mut by_local: HashMap<SocketAddr, u32> = HashMap::new();
    for row in table::<MIB_TCPROW_OWNER_PID>(AF_INET) {
        if port(row.dwRemotePort) == proxy_port {
            let ip = Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes());
            by_local.insert(
                SocketAddr::new(IpAddr::V4(ip), port(row.dwLocalPort)),
                row.dwOwningPid,
            );
        }
    }
    for row in table::<MIB_TCP6ROW_OWNER_PID>(AF_INET6) {
        if port(row.dwRemotePort) == proxy_port {
            let ip = Ipv6Addr::from(row.ucLocalAddr).to_canonical();
            by_local.insert(SocketAddr::new(ip, port(row.dwLocalPort)), row.dwOwningPid);
        }
    }
    clients
        .iter()
        .map(|client| {
            let client = SocketAddr::new(client.ip().to_canonical(), client.port());
            by_local.get(&client).copied()
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
