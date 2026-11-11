//! Letting other devices on the network use the proxy, switched on and off
//! while it runs.
//!
//! The proxy always listens on this machine's loopback. For other devices it
//! listens, on the same port, on each address this machine has on its networks
//! too: listening on them all at once (`0.0.0.0`) would clash with the
//! loopback's socket. Those addresses come and go as Wi-Fi changes, so they are
//! looked at again every few seconds while the switch is on, which takes tens
//! of microseconds.

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::Notify;

/// Whether other devices may use the proxy, and where it listens for them.
#[derive(Debug, Default)]
pub struct OtherDevices {
    on: AtomicBool,
    changed: Notify,
    listening: Mutex<Vec<SocketAddr>>,
    /// What went wrong listening on an address, if something did.
    problem: Mutex<Option<String>>,
}

impl OtherDevices {
    pub fn new(on: bool) -> Self {
        Self {
            on: AtomicBool::new(on),
            ..Self::default()
        }
    }

    pub fn is_on(&self) -> bool {
        self.on.load(Ordering::Relaxed)
    }

    /// Lets other devices in, or keeps them out, at once.
    pub fn set(&self, on: bool) {
        self.on.store(on, Ordering::Relaxed);
        self.changed.notify_one();
    }

    /// Where other devices reach the proxy now.
    pub fn listening(&self) -> Vec<SocketAddr> {
        lock(&self.listening).clone()
    }

    pub fn problem(&self) -> Option<String> {
        lock(&self.problem).clone()
    }

    pub(crate) fn changed(&self) -> &Notify {
        &self.changed
    }

    pub(crate) fn listen_at(&self, addrs: Vec<SocketAddr>, problem: Option<String>) {
        *lock(&self.listening) = addrs;
        *lock(&self.problem) = problem;
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Whether other devices could reach this machine at `ip`: not its loopback,
/// and not a link-local address, which a network without DHCP falls back to.
pub fn reachable(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified()
}

/// This machine's IPv4 addresses on its networks that are up, sorted.
pub fn addresses() -> Vec<Ipv4Addr> {
    let mut found: Vec<Ipv4Addr> = os::addresses()
        .into_iter()
        .filter(|ip| reachable(*ip))
        .collect();
    found.sort_unstable();
    found.dedup();
    found
}

#[cfg(unix)]
mod os {
    use std::net::Ipv4Addr;

    pub fn addresses() -> Vec<Ipv4Addr> {
        let mut found = Vec::new();
        let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
        // SAFETY: getifaddrs writes a list that freeifaddrs frees; each entry
        // is read while the list lives, and an address only as its family says.
        unsafe {
            if libc::getifaddrs(&mut list) != 0 {
                return found;
            }
            let mut at = list;
            while !at.is_null() {
                let entry = &*at;
                let up = entry.ifa_flags & libc::IFF_UP as u32 != 0;
                if up
                    && !entry.ifa_addr.is_null()
                    && i32::from((*entry.ifa_addr).sa_family) == libc::AF_INET
                {
                    let addr = &*(entry.ifa_addr as *const libc::sockaddr_in);
                    found.push(Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr)));
                }
                at = entry.ifa_next;
            }
            libc::freeifaddrs(list);
        }
        found
    }
}

#[cfg(windows)]
mod os {
    use std::net::Ipv4Addr;
    use windows_sys::Win32::{
        NetworkManagement::IpHelper::{
            GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
            GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
        },
        NetworkManagement::Ndis::IfOperStatusUp,
        Networking::WinSock::{AF_INET, SOCKADDR_IN},
    };

    pub fn addresses() -> Vec<Ipv4Addr> {
        let mut found = Vec::new();
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
        let mut size: u32 = 16 * 1024;
        let mut buffer: Vec<u64> = Vec::new();
        // The list's size is known only once asked for; it may grow in between.
        for _ in 0..3 {
            buffer.resize((size as usize).div_ceil(8), 0);
            // SAFETY: the buffer is `size` bytes, aligned for the structures.
            let result = unsafe {
                GetAdaptersAddresses(
                    u32::from(AF_INET),
                    flags,
                    std::ptr::null(),
                    buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>(),
                    &mut size,
                )
            };
            match result {
                0 => break,
                // ERROR_BUFFER_OVERFLOW: `size` now says how much.
                111 => continue,
                _ => return found,
            }
        }
        // SAFETY: GetAdaptersAddresses filled the buffer with a linked list
        // of adapters, each with a linked list of unicast addresses.
        unsafe {
            let mut adapter = buffer.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
            while !adapter.is_null() {
                let a = &*adapter;
                if a.OperStatus == IfOperStatusUp {
                    let mut unicast = a.FirstUnicastAddress;
                    while !unicast.is_null() {
                        let u = &*unicast;
                        let sockaddr = u.Address.lpSockaddr;
                        if !sockaddr.is_null() && (*sockaddr).sa_family == AF_INET {
                            let v4 = &*(sockaddr as *const SOCKADDR_IN);
                            found.push(Ipv4Addr::from(u32::from_be(v4.sin_addr.S_un.S_addr)));
                        }
                        unicast = u.Next;
                    }
                }
                adapter = a.Next;
            }
        }
        found
    }
}

#[cfg(not(any(unix, windows)))]
mod os {
    pub fn addresses() -> Vec<std::net::Ipv4Addr> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_addresses_other_devices_can_reach() {
        assert!(!reachable(Ipv4Addr::LOCALHOST));
        assert!(!reachable(Ipv4Addr::new(169, 254, 1, 2)));
        assert!(!reachable(Ipv4Addr::UNSPECIFIED));
        assert!(reachable(Ipv4Addr::new(192, 168, 1, 13)));
        for ip in addresses() {
            assert!(reachable(ip), "{ip}");
        }
    }

    #[test]
    fn the_switch_says_where_it_listens() {
        let devices = OtherDevices::new(false);
        assert!(!devices.is_on());
        devices.set(true);
        assert!(devices.is_on());
        devices.listen_at(vec!["192.168.1.13:9090".parse().unwrap()], None);
        assert_eq!(devices.listening().len(), 1);
    }
}
