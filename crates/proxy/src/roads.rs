//! Which interface tracewing's own connections should leave by.
//!
//! [`crate::upstream::leave_by`] pins them, one interface per family; this
//! is what decides which. The answer is the interface this machine would
//! reach the internet
//! by **if tracewing's own adapter were not there** — because that adapter
//! is where everything else is being routed, and a connection of ours that
//! followed it would arrive straight back at us.
//!
//! The choosing is here in full, and reading the table is a dozen lines of
//! Windows beneath it. That split is for the usual reason: the choosing is
//! where the judgement is, and Linux is what sees every change.
//!
//! [`out_by`] answers the question once. [`watch`] keeps answering it,
//! which is not a convenience: WireGuard for Windows solves the same
//! problem and watches for the default-route interface rather than reading
//! it once, because which one that is changes as a machine moves between
//! Wi-Fi, Ethernet and tethering — and with none at all, its outgoing
//! packets are blackholed.
//!
//! See [capture-modes.md](../../../docs/design/capture-modes.md).

/// One row of the routing table, as much of it as choosing needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Road {
    /// The interface it leaves by, as Windows numbers them for this family.
    /// **The two families number an adapter separately**, so this says
    /// nothing without [`Road::v6`] beside it — and it is what the socket
    /// option wants, since that option is per family too.
    pub interface: u32,
    /// The same interface, as Windows *identifies* rather than numbers it.
    /// One LUID per interface, whatever the family, which is why the adapter
    /// to leave out is named by this and not by an index.
    pub luid: u64,
    /// How many bits of the destination it matches. Zero is a default route;
    /// one is each half of a default written as two, which is how a TUN
    /// adapter is routed and how some VPNs route themselves.
    pub bits: u8,
    /// The metric the table gives this route.
    ///
    /// Windows ranks two equally long routes by this **plus the metric of
    /// the interface**, and the interface's half is not in this row: adding
    /// it needs a `GetIpInterfaceEntry` for each interface. So two
    /// candidates whose route metrics tie may be ordered differently here
    /// than the operating system would order them. Reaching further decides
    /// first and settles the ordinary cases; this is only the tie-break, and
    /// what it would take to make it exact is written down rather than
    /// guessed at.
    pub metric: u32,
    /// Over the loopback, which goes nowhere this is asking about.
    pub loopback: bool,
    /// Of the IPv6 table rather than the IPv4 one.
    pub v6: bool,
    /// Whether the interface's link is really up.
    ///
    /// **A disconnected interface keeps every one of its routes**, which is
    /// what makes this a field rather than something the table says. Measured
    /// on a Windows machine with two NAT adapters: pulling the second cable
    /// left all six of its rows in place, default route included, with the
    /// row's own `Metric` unchanged at 0 -- and the *interface* metric
    /// improved, 25 to 5, so nothing in the numbers makes a dead interface
    /// look worse than a live one. Linux is the same shape (#332): `RTF_UP`
    /// stays set and `/proc/net/route` has nothing to mark it.
    ///
    /// Read rather than guessed, and read as **plainly connected or not**:
    /// unknown, and a reading that fails, both count as not. The asymmetry
    /// that settles it is that the two mistakes are not alike -- excluding
    /// too much ends as [`Out::Nowhere`], which refuses the connection and
    /// says why, while excluding too little pins to an interface the stack is
    /// ignoring, and that does not fail, it **hangs**.
    pub connected: bool,
}

/// How far a route may reach and still count as a way out of here: all of
/// the address space, or half of it.
const WIDE_ENOUGH: u8 = 1;

/// The interface tracewing's own connections of family `v6` should leave by,
/// given the routing table and `ours`, the **LUID** of the adapter tracewing
/// itself holds.
///
/// A LUID rather than an index, and that is not a detail. An index belongs
/// to one family: the documentation says `Ipv6IfIndex` is zero where an
/// interface has no IPv6 at all, and says neither index is persistent across
/// an adapter being disabled and enabled. A machine where ours happened to
/// be numbered differently in the two families would have had one family
/// excluded and the other not — and the family that was not excluded is the
/// one whose connections loop. A LUID names the interface in both.
///
/// The number that comes *back* is still the per-family index, because that
/// is what `IP_UNICAST_IF` and `IPV6_UNICAST_IF` take, and it comes from the
/// row of the family that was asked for rather than from any conversion.
///
/// Ours is never the answer; that is the entire purpose. Among the rest the
/// order is the one the operating system forwards by: **the route that
/// reaches furthest wins**, so a VPN's half of the space beats an Ethernet
/// card's whole of it, exactly as it would for any other program. Then the
/// cheaper metric. Then the lower interface, only so that the answer is the
/// same twice rather than whichever the table happened to list first.
///
/// `None` means there is no way out that is not ours. That is the case
/// WireGuard describes as its packets being blackholed, and it is a caller's
/// to handle: pinning to nothing is not the same as not pinning.
pub fn out_by(roads: &[Road], ours: u64, v6: bool) -> Option<u32> {
    roads
        .iter()
        .filter(|road| {
            road.v6 == v6
                && !road.loopback
                && road.luid != ours
                && road.bits <= WIDE_ENOUGH
                // A route on an interface whose link is down is still in the
                // table, and nothing in its numbers says so: see
                // `Road::connected`. Pinning to one does not fail, it hangs.
                && road.connected
        })
        .min_by_key(|road| (std::cmp::Reverse(road.bits), road.metric, road.interface))
        .map(|road| road.interface)
}

/// One row of a Linux routing table, before its interface has a number.
///
/// Linux names an interface in its tables and numbers it elsewhere, so the
/// reading and the numbering are two steps. Keeping them apart is also what
/// lets the reading be tested anywhere: [`rows_v4`] and [`rows_v6`] are text
/// in, rows out, and run on every platform's CI rather than only where
/// `/proc` exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub iface: String,
    pub bits: u8,
    pub metric: u32,
    pub v6: bool,
}

/// `RTF_UP`: the route is in use. A row without it is in the table and not
/// being forwarded by — an unreachable default sits there on an ordinary
/// machine, and taking it for a way out would pin tracewing's own
/// connections to nowhere.
const RTF_UP: u32 = 0x0001;

/// The rows of `/proc/net/route`, which is the IPv4 main table.
///
/// Columns are `Iface Destination Gateway Flags RefCnt Use Metric Mask …`,
/// with the addresses as little-endian hex. A row that cannot be read is
/// skipped rather than failing the lot: the file is a kernel interface that
/// has grown columns before, and one strange line is not a reason to have no
/// routes at all.
pub fn rows_v4(text: &str) -> Vec<Row> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let iface = fields.next()?;
            let _destination = fields.next()?;
            let _gateway = fields.next()?;
            let flags = u32::from_str_radix(fields.next()?, 16).ok()?;
            let (_refcnt, _use) = (fields.next()?, fields.next()?);
            let metric = fields.next()?.parse::<u32>().ok()?;
            let mask = u32::from_str_radix(fields.next()?, 16).ok()?;
            (flags & RTF_UP != 0).then(|| Row {
                iface: iface.to_owned(),
                // The mask is little-endian, and how many bits it covers is
                // all that is wanted: a contiguous mask has the same count
                // whichever end it is read from.
                bits: mask.count_ones() as u8,
                metric,
                v6: false,
            })
        })
        .collect()
}

/// The rows of `/proc/net/ipv6_route`.
///
/// Columns are the destination, **its prefix length in hex**, the source and
/// its length, the next hop, the metric, two counts, the flags, and the
/// interface last. No header line, unlike the IPv4 file.
pub fn rows_v6(text: &str) -> Vec<Row> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [
                _destination,
                bits,
                _source,
                _source_bits,
                _hop,
                metric,
                _refcnt,
                _use,
                flags,
                iface,
            ] = fields.as_slice()
            else {
                return None;
            };
            let flags = u32::from_str_radix(flags, 16).ok()?;
            if flags & RTF_UP == 0 {
                return None;
            }
            Some(Row {
                iface: (*iface).to_owned(),
                bits: u8::from_str_radix(bits, 16).ok()?,
                metric: u32::from_str_radix(metric, 16).ok()?,
                v6: true,
            })
        })
        .collect()
}

/// This machine's routes as they are now, both families.
///
/// `luid` and `interface` hold the same number here, which looks like
/// laziness and is not: **Linux numbers an interface once**, for both
/// families, where Windows numbers it per family and identifies it by a LUID
/// that spans them. The distinction `Road` makes exists because Windows
/// needs it; on Linux the one number is both answers.
///
/// What this cannot see: `/proc/net/route` is the **main** table, so a
/// machine routing by `ip rule` has routes that are not here — the same
/// class of gap as an interface's own metric being absent from a Windows
/// row.
#[cfg(target_os = "linux")]
pub fn roads_now() -> anyhow::Result<Vec<Road>> {
    let v4 = std::fs::read_to_string("/proc/net/route").unwrap_or_default();
    let v6 = std::fs::read_to_string("/proc/net/ipv6_route").unwrap_or_default();
    let rows: Vec<Row> = rows_v4(&v4).into_iter().chain(rows_v6(&v6)).collect();
    anyhow::ensure!(
        !rows.is_empty(),
        "no routes in /proc/net/route or /proc/net/ipv6_route"
    );
    Ok(rows
        .into_iter()
        .filter(|row| carrier_is_up(&row.iface))
        .filter_map(|row| {
            let index = index_of(&row.iface)?;
            Some(Road {
                interface: index,
                luid: u64::from(index),
                bits: row.bits,
                metric: row.metric,
                loopback: is_loopback(&row.iface),
                v6: row.v6,
                // The rows of an interface whose carrier is down were taken
                // out above, so what reaches here is connected by
                // construction. Windows reads it per interface instead,
                // because its rows cannot be dropped that early: our own
                // adapter's have to stay in the table to be recognised as
                // ours (`wintun_live.rs`).
                connected: true,
            })
        })
        .collect())
}

/// Whether the link is really up, from `/sys/class/net/<iface>/carrier`.
///
/// **This is the one thing the tables cannot say.** A route the kernel is
/// ignoring — `dead linkdown` in `ip route show` — is listed in
/// `/proc/net/route` with `RTF_UP` set and nothing else to mark it. Without
/// this, a device that is up with its carrier down is a way out: measured on
/// 2026-11-17 with a second device in that state carrying a default route of
/// metric 40 beside the real one's 600, [`out_by`] chose it — index 44 —
/// while the kernel itself routed through the Wi-Fi, index 6. An unplugged
/// cable with a static route is enough to arrange it, and a connection
/// pinned to an interface the kernel is ignoring **hangs** rather than
/// fails: 0.84s against a 3s timeout, measured when the exclusion went in.
///
/// **Anything but a plain `1` excludes the interface, unreadable included.**
/// That is the safe direction rather than the cautious-looking one: a family
/// left with no way out at all is `Out::Nowhere` in
/// [`crate::upstream::leave_by`]'s reader, which refuses the connection and
/// says so, where keeping a dead interface means pinning to it and waiting.
/// A loud failure beats a silent wait, and neither is the loop this seam
/// exists to prevent.
#[cfg(target_os = "linux")]
fn carrier_is_up(iface: &str) -> bool {
    says_up(
        std::fs::read_to_string(format!("/sys/class/net/{iface}/carrier"))
            .ok()
            .as_deref(),
    )
}

/// The policy above as a function, so that it is a test rather than a
/// sentence.
#[cfg(target_os = "linux")]
fn says_up(carrier: Option<&str>) -> bool {
    carrier.is_some_and(|carrier| carrier.trim() == "1")
}

/// What the kernel numbers an interface, from its name.
#[cfg(target_os = "linux")]
fn index_of(name: &str) -> Option<u32> {
    let name = std::ffi::CString::new(name).ok()?;
    // SAFETY: a null-terminated name, borrowed for the call.
    let index = unsafe { libc::if_nametoindex(name.as_ptr()) };
    (index != 0).then_some(index)
}

/// Whether an interface is the loopback, from the flags the kernel shows.
///
/// By its flags rather than by its name: `lo` is what every machine calls
/// it, which is not the same as a promise, and the answer decides whether a
/// route is a candidate for carrying tracewing's own traffic.
#[cfg(target_os = "linux")]
fn is_loopback(name: &str) -> bool {
    const IFF_LOOPBACK: u64 = 0x8;
    let flags = std::fs::read_to_string(format!("/sys/class/net/{name}/flags")).ok();
    match flags
        .as_deref()
        .map(str::trim)
        .and_then(|text| u64::from_str_radix(text.trim_start_matches("0x"), 16).ok())
    {
        Some(flags) => flags & IFF_LOOPBACK != 0,
        // Nothing to read is not nothing to say: the name is what is left.
        None => name == "lo",
    }
}

/// This machine's routes as they are now, both families.
#[cfg(windows)]
pub fn roads_now() -> anyhow::Result<Vec<Road>> {
    use windows_sys::Win32::{
        Foundation::NO_ERROR,
        NetworkManagement::IpHelper::{FreeMibTable, GetIpForwardTable2, MIB_IPFORWARD_TABLE2},
        Networking::WinSock::{AF_INET6, AF_UNSPEC},
    };
    let connected = connected_luids();
    let mut table: *mut MIB_IPFORWARD_TABLE2 = std::ptr::null_mut();
    // SAFETY: a pointer the call fills in, and nothing is read from it
    // unless it says it succeeded.
    let got = unsafe { GetIpForwardTable2(AF_UNSPEC, &mut table) };
    if got != NO_ERROR {
        anyhow::bail!("read this machine's routes: error {got}");
    }
    // SAFETY: the table's first row is followed by `NumEntries` of them,
    // which is how this structure is always read, and none of what is taken
    // here borrows from it past the free below.
    let roads: Vec<Road> = unsafe {
        std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize)
            .iter()
            .map(|row| Road {
                interface: row.InterfaceIndex,
                // SAFETY: a union over a `u64` and a bitfield of the same
                // eight bytes; the whole value is what identifies it.
                luid: row.InterfaceLuid.Value,
                bits: row.DestinationPrefix.PrefixLength,
                metric: row.Metric,
                loopback: row.Loopback,
                v6: row.DestinationPrefix.Prefix.si_family == AF_INET6,
                connected: connected.contains(&row.InterfaceLuid.Value),
            })
            .collect()
    };
    // SAFETY: the pointer that call gave us, given back exactly once.
    unsafe { FreeMibTable(table as *const core::ffi::c_void) };
    Ok(roads)
}

/// The interfaces whose link is plainly connected, by LUID.
///
/// Windows' answer to Linux's `/sys/class/net/<iface>/carrier`. Taken from
/// `GetIfTable2` in one call rather than `GetIfEntry2` per route, since an
/// ordinary machine has a few interfaces and a good many routes.
///
/// `MediaConnectState` is the field that says it -- measured: a NAT adapter
/// whose cable was pulled went to `MediaConnectStateDisconnected` while every
/// one of its six routes stayed in `GetIpForwardTable2`. `OperStatus` said
/// `IfOperStatusDown` in the same breath, so either would have done; this one
/// is named for the link itself rather than for the interface's willingness
/// to carry traffic, which is the question being asked.
///
/// A table that cannot be read leaves **nothing** connected, which turns
/// every way out into [`Out::Nowhere`] -- loud, and the safe direction of the
/// two (see [`Road::connected`]).
#[cfg(windows)]
fn connected_luids() -> std::collections::HashSet<u64> {
    use windows_sys::Win32::{
        Foundation::NO_ERROR,
        NetworkManagement::{
            IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2},
            Ndis::MediaConnectStateConnected,
        },
    };
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: a pointer the call fills in, and nothing is read from it unless
    // it says it succeeded.
    if unsafe { GetIfTable2(&mut table) } != NO_ERROR {
        return std::collections::HashSet::new();
    }
    // SAFETY: the table's first row is followed by `NumEntries` of them,
    // which is how this structure is always read, and the LUID is a plain
    // `u64` taken out of a union over the same eight bytes.
    let luids = unsafe {
        std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize)
            .iter()
            .filter(|row| row.MediaConnectState == MediaConnectStateConnected)
            .map(|row| row.InterfaceLuid.Value)
            .collect()
    };
    // SAFETY: the pointer that call gave us, given back exactly once.
    unsafe { FreeMibTable(table as *const core::ffi::c_void) };
    luids
}

/// One address an interface holds, as the table has it.
///
/// Only the parts that decide whether it may be a **source**: which
/// interface, which family, and the three things that disqualify one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mine {
    /// The interface it belongs to, identified rather than numbered, as
    /// [`Road::luid`] is and for the same reason.
    pub luid: u64,
    pub addr: std::net::IpAddr,
    /// Whether duplicate address detection has settled on it. An address
    /// that is still tentative, or found to be a duplicate, is one the
    /// machine must not send from -- Windows would refuse it, and the echo
    /// would come back as an error that looks like the network.
    pub settled: bool,
    /// Windows' own "do not choose this as a source". It marks addresses
    /// that exist to be reached rather than to send from -- a cluster
    /// address, some tunnel endpoints -- so honouring it is not politeness,
    /// it is the difference between an echo that leaves and one that does
    /// not.
    pub skip_as_source: bool,
}

/// The address to send an echo from, out of what the machine holds.
///
/// `luid` is the interface the machine's own traffic leaves by, which the
/// watcher already knows; `v6` picks the family, because an echo to an IPv6
/// address has to leave from an IPv6 address.
///
/// Why this is not simply "the first one": an interface commonly holds
/// several, and most of them cannot carry an echo to the internet. A
/// **link-local** address (`169.254.0.0/16`, `fe80::/10`) is reachable only
/// on the wire it is on, so an echo sent from one goes nowhere; an address
/// still in duplicate address detection is one Windows will not send from;
/// and one marked `SkipAsSource` is one it is told not to choose.
///
/// Among what is left the lowest is taken, and **deterministically** on
/// purpose: a choice that moved between one echo and the next would change
/// the source address mid-`tracert`, and a trace whose probes come from
/// different addresses is a trace nobody can read.
pub fn source_on(mine: &[Mine], luid: u64, v6: bool) -> Option<std::net::IpAddr> {
    mine.iter()
        .filter(|one| {
            one.luid == luid
                && one.addr.is_ipv6() == v6
                && one.settled
                && !one.skip_as_source
                && !is_link_local(one.addr)
        })
        .map(|one| one.addr)
        .min()
}

/// Reachable only on the wire it is on, so no use as a source for anything
/// further away. `Ipv4Addr::is_link_local` exists; IPv6's is unstable, so
/// `fe80::/10` is written out.
fn is_link_local(addr: std::net::IpAddr) -> bool {
    match addr {
        std::net::IpAddr::V4(addr) => addr.is_link_local(),
        std::net::IpAddr::V6(addr) => (addr.segments()[0] & 0xffc0) == 0xfe80,
    }
}

/// Every unicast address this machine holds, in both families.
#[cfg(windows)]
pub fn addresses_now() -> anyhow::Result<Vec<Mine>> {
    use windows_sys::Win32::{
        Foundation::NO_ERROR,
        NetworkManagement::IpHelper::{
            FreeMibTable, GetUnicastIpAddressTable, MIB_UNICASTIPADDRESS_TABLE,
        },
        Networking::WinSock::{AF_UNSPEC, IpDadStatePreferred},
    };
    let mut table: *mut MIB_UNICASTIPADDRESS_TABLE = std::ptr::null_mut();
    // SAFETY: a pointer the call fills in, read only if it says it worked.
    let got = unsafe { GetUnicastIpAddressTable(AF_UNSPEC, &mut table) };
    if got != NO_ERROR {
        anyhow::bail!("read this machine's addresses: error {got}");
    }
    // SAFETY: the first row is followed by `NumEntries` of them, which is how
    // this structure is always read, and nothing taken here borrows from it
    // past the free below.
    let mine: Vec<Mine> = unsafe {
        std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize)
            .iter()
            .filter_map(|row| {
                Some(Mine {
                    // SAFETY: a union over a `u64` and a bitfield of the same
                    // eight bytes; the whole value identifies the interface.
                    luid: row.InterfaceLuid.Value,
                    addr: from_sockaddr(&row.Address)?,
                    settled: row.DadState == IpDadStatePreferred,
                    skip_as_source: row.SkipAsSource,
                })
            })
            .collect()
    };
    // SAFETY: the pointer that call gave us, given back exactly once.
    unsafe { FreeMibTable(table as *const core::ffi::c_void) };
    Ok(mine)
}

/// The address in a `SOCKADDR_INET`, whichever arm is live.
#[cfg(windows)]
fn from_sockaddr(
    addr: &windows_sys::Win32::Networking::WinSock::SOCKADDR_INET,
) -> Option<std::net::IpAddr> {
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
    // SAFETY: `si_family` is what says which arm of the union is the live
    // one, and each arm is read only under its own family.
    unsafe {
        match addr.si_family {
            AF_INET => {
                Some(std::net::Ipv4Addr::from(addr.Ipv4.sin_addr.S_un.S_addr.to_ne_bytes()).into())
            }
            AF_INET6 => Some(std::net::Ipv6Addr::from(addr.Ipv6.sin6_addr.u.Byte).into()),
            _ => None,
        }
    }
}

/// Pins the way out now, and keeps it pinned, until `stop`.
///
/// **The first look happens before this returns, and that is the point of
/// the shape.** A caller must be able to know the exclusion is in place
/// *before* it routes anything into the adapter: between a route going in
/// and the pin being set, every connection tracewing makes is routed back
/// into itself. So a first look that fails is an error here rather than a
/// warning in a loop, and a caller that gets one must not add its routes.
///
/// After that it looks every `every` and pins again only when the answer has
/// changed. A read that fails later is a warning and not the end — the table
/// is read from a live system and a momentary failure is not a reason to
/// stop excluding our own adapter.
///
/// **Polling rather than `NotifyRouteChange2`**, which is a callback into
/// our code from a Windows worker thread. The cost of polling is bounded and
/// worth naming: for up to `every` after the network changes, the pin names
/// an interface that is no longer the way out, so tracewing's own
/// connections can fail until the next look. What it cannot do in that
/// window is loop, because a stale answer still is not our own adapter.
///
/// `look` and `pin` are taken rather than called directly so that all of the
/// above is tested where it can be run, which is everywhere;
/// [`roads_now`] and [`crate::upstream::leave_by`] are what a caller passes.
pub fn watch(
    ours: u64,
    every: std::time::Duration,
    look: impl Fn() -> anyhow::Result<Vec<Road>> + Send + 'static,
    pin: impl Fn(Option<crate::upstream::OutBy>) + Send + 'static,
    stop: crate::source::Stop,
) -> anyhow::Result<()> {
    let first = asked(&look, ours)?;
    pin(Some(first));
    tokio::spawn(async move {
        let mut last = first;
        loop {
            tokio::select! {
                _ = crate::source::stopped(stop.clone()) => break,
                _ = tokio::time::sleep(every) => {}
            }
            match asked(&look, ours) {
                Ok(now) if now != last => {
                    tracing::debug!("the way out changed: {last:?} to {now:?}");
                    pin(Some(now));
                    last = now;
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("looking for the way out: {e:#}"),
            }
        }
        // Holding nothing is not the same as having no preference, which is
        // why this is said rather than left: see `leave_by`.
        pin(None);
    });
    Ok(())
}

/// One look, as both halves of an answer.
fn asked(
    look: &impl Fn() -> anyhow::Result<Vec<Road>>,
    ours: u64,
) -> anyhow::Result<crate::upstream::OutBy> {
    let roads = look()?;
    Ok(crate::upstream::OutBy {
        v4: out_by(&roads, ours, false),
        v6: out_by(&roads, ours, true),
    })
}

#[cfg(test)]
mod watching {
    use super::*;
    use crate::upstream::OutBy;
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
        time::Duration,
    };

    /// Short enough that a test does not wait on it, long enough that it is
    /// a real interval rather than a busy loop.
    const EVERY: Duration = Duration::from_millis(20);
    const SOON: Duration = Duration::from_secs(5);

    fn road(interface: u32, luid: u64) -> Road {
        Road {
            interface,
            luid,
            bits: 0,
            metric: 10,
            loopback: false,
            v6: false,
            connected: true,
        }
    }

    /// A table to look at and a record of what was pinned. When the queue
    /// runs out it keeps answering with the last table it gave, which is
    /// what a machine whose network is not changing looks like.
    #[derive(Default)]
    struct Fake {
        /// `None` is a look that fails.
        queue: Mutex<VecDeque<Option<Vec<Road>>>>,
        last: Mutex<Vec<Road>>,
        pinned: Mutex<Vec<Option<OutBy>>>,
    }

    impl Fake {
        fn with(looks: Vec<Option<Vec<Road>>>) -> Arc<Self> {
            Arc::new(Self {
                queue: Mutex::new(looks.into()),
                ..Self::default()
            })
        }

        fn look(&self) -> anyhow::Result<Vec<Road>> {
            match self.queue.lock().unwrap().pop_front() {
                Some(Some(roads)) => {
                    *self.last.lock().unwrap() = roads.clone();
                    Ok(roads)
                }
                Some(None) => anyhow::bail!("the table could not be read"),
                None => Ok(self.last.lock().unwrap().clone()),
            }
        }

        fn pinned(&self) -> Vec<Option<OutBy>> {
            self.pinned.lock().unwrap().clone()
        }
    }

    /// Starts a watch over `fake`, and hands back the stop so a test can
    /// keep it alive — dropping it would look like a stop.
    fn watching(
        fake: &Arc<Fake>,
        ours: u64,
    ) -> (anyhow::Result<()>, tokio::sync::watch::Sender<bool>) {
        let (stop_tx, stop) = tokio::sync::watch::channel(false);
        let looking = Arc::clone(fake);
        let pinning = Arc::clone(fake);
        let started = watch(
            ours,
            EVERY,
            move || looking.look(),
            move |out| pinning.pinned.lock().unwrap().push(out),
            stop,
        );
        (started, stop_tx)
    }

    async fn until(what: &str, f: impl Fn() -> bool) {
        let end = tokio::time::Instant::now() + SOON;
        while !f() {
            assert!(tokio::time::Instant::now() < end, "waiting for {what}");
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    /// The ordering the whole shape is for: a caller must know the exclusion
    /// is in place before it routes anything in.
    #[tokio::test]
    async fn the_first_look_is_pinned_before_watch_returns() {
        let fake = Fake::with(vec![Some(vec![road(7, 70)])]);
        let (started, _stop) = watching(&fake, 4242);
        started.expect("the first look worked");
        assert_eq!(
            fake.pinned(),
            vec![Some(OutBy {
                v4: Some(7),
                v6: None
            })],
            "pinned already, with nothing awaited in between"
        );
    }

    #[tokio::test]
    async fn a_first_look_that_fails_is_an_error_and_pins_nothing() {
        let fake = Fake::with(vec![None]);
        let (started, _stop) = watching(&fake, 4242);
        started.expect_err("the caller must not go on to add routes");
        assert!(
            fake.pinned().is_empty(),
            "nothing was pinned, so nothing claims an exclusion that is not there"
        );
    }

    #[tokio::test]
    async fn an_answer_that_has_not_changed_is_not_pinned_again() {
        let fake = Fake::with(vec![Some(vec![road(7, 70)])]);
        let (started, _stop) = watching(&fake, 4242);
        started.expect("the first look worked");
        // Several intervals of the same table.
        tokio::time::sleep(EVERY * 5).await;
        assert_eq!(fake.pinned().len(), 1, "pinned once, not once per look");
    }

    #[tokio::test]
    async fn a_way_out_that_changes_is_pinned_again() {
        let fake = Fake::with(vec![Some(vec![road(7, 70)]), Some(vec![road(9, 90)])]);
        let (started, _stop) = watching(&fake, 4242);
        started.expect("the first look worked");
        until("the new way out", || fake.pinned().len() == 2).await;
        assert_eq!(
            fake.pinned()[1],
            Some(OutBy {
                v4: Some(9),
                v6: None
            })
        );
    }

    /// The table is read from a live system; a momentary failure is not a
    /// reason to stop excluding our own adapter.
    #[tokio::test]
    async fn a_look_that_fails_later_does_not_end_the_watch() {
        let fake = Fake::with(vec![Some(vec![road(7, 70)]), None, Some(vec![road(9, 90)])]);
        let (started, _stop) = watching(&fake, 4242);
        started.expect("the first look worked");
        until("the change after the failure", || fake.pinned().len() == 2).await;
        assert_eq!(
            fake.pinned()[1],
            Some(OutBy {
                v4: Some(9),
                v6: None
            }),
            "the failure was skipped rather than fatal"
        );
    }

    #[tokio::test]
    async fn the_stop_pins_nothing_rather_than_leaving_the_last_answer() {
        let fake = Fake::with(vec![Some(vec![road(7, 70)])]);
        let (started, stop) = watching(&fake, 4242);
        started.expect("the first look worked");
        let _ = stop.send(true);
        until("the unpin", || fake.pinned().last() == Some(&None)).await;
    }
}

/// Reading this machine's real table, which wants no privilege, so the
/// Windows job runs it.
///
/// What it guards is the **shape** of the structure, because that is the
/// part of this which would go wrong quietly: a layout that disagrees with
/// Windows does not error, it hands back rows of nonsense.
#[cfg(all(windows, test))]
mod reading {
    use super::*;

    #[test]
    fn the_table_reads_as_routes_and_not_as_nonsense() {
        let roads = roads_now().expect("this machine has a routing table");
        assert!(
            !roads.is_empty(),
            "every machine has at least a route to itself"
        );
        for road in &roads {
            let most = match road.v6 {
                true => 128,
                false => 32,
            };
            assert!(
                road.bits <= most,
                "a prefix of {} bits: {road:?}",
                road.bits
            );
            assert_ne!(road.interface, 0, "a route leaves by some interface");
        }
        // Printed rather than asserted: no assertion here can say whether
        // the interface chosen is the right one, and a person reading the
        // run can.
        println!(
            "{} routes, {} of them IPv6",
            roads.len(),
            roads.iter().filter(|road| road.v6).count()
        );
        println!("the way out for IPv4: {:?}", out_by(&roads, 0, false));
        println!("the way out for IPv6: {:?}", out_by(&roads, 0, true));
    }
}

#[cfg(test)]
mod linux_tables {
    use super::*;

    /// `/proc/net/route` as this machine wrote it, header and all.
    const V4: &str = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
wlp8s0\t00000000\t0101A8C0\t0003\t0\t0\t600\t00000000\t0\t0\t0
docker0\t000011AC\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
wlp8s0\t0001A8C0\t00000000\t0001\t0\t0\t600\t00FFFFFF\t0\t0\t0
";

    /// `/proc/net/ipv6_route`, with the two rows that matter: the default
    /// this machine forwards by, and an **unreachable** default over `lo`
    /// that is in the table with `RTF_UP` clear.
    const V6: &str = "\
00000000000000000000000000000000 00 00000000000000000000000000000000 00 fe800000000000000000000000000001 00000258 00000034 00000000 18000003   wlp8s0
20010ee040c17deb0000000000000000 40 00000000000000000000000000000000 00 00000000000000000000000000000000 00000258 00000001 00000000 00000001   wlp8s0
00000000000000000000000000000000 00 00000000000000000000000000000000 00 00000000000000000000000000000000 ffffffff 00000001 00000000 00200200       lo
";

    /// The mask is little-endian hex and what is wanted is how far the route
    /// reaches. A `/0` read as anything else would make the way out look
    /// narrower than the adapter's own half-default and lose to it.
    #[test]
    fn an_ipv4_table_reads_as_the_routes_it_is() {
        let rows = rows_v4(V4);
        assert_eq!(
            rows,
            vec![
                Row {
                    iface: "wlp8s0".into(),
                    bits: 0,
                    metric: 600,
                    v6: false
                },
                Row {
                    iface: "docker0".into(),
                    bits: 16,
                    metric: 0,
                    v6: false
                },
                Row {
                    iface: "wlp8s0".into(),
                    bits: 24,
                    metric: 600,
                    v6: false
                },
            ],
            "the header is skipped and every mask counts its own bits"
        );
    }

    /// The row this test exists for: an unreachable `::/0` sits in the table
    /// of an ordinary machine with `RTF_UP` clear. Taken as a way out it
    /// would be a default route — the widest there is — and tracewing's own
    /// connections would be pinned to the loopback.
    #[test]
    fn a_route_that_is_not_up_is_not_a_way_out() {
        let rows = rows_v6(V6);
        assert_eq!(
            rows,
            vec![
                Row {
                    iface: "wlp8s0".into(),
                    bits: 0,
                    metric: 0x258,
                    v6: true
                },
                Row {
                    iface: "wlp8s0".into(),
                    bits: 0x40,
                    metric: 0x258,
                    v6: true
                },
            ],
            "the unreachable default over lo is dropped"
        );
        assert!(
            !rows.iter().any(|row| row.iface == "lo"),
            "and nothing of the loopback's is left to choose from"
        );
    }

    /// A line the kernel grows a column for, or a line of nonsense, costs
    /// that line and not the table: a machine with no routes at all is how
    /// `out_by` is told there is no way out, which is a different thing.
    #[test]
    fn a_line_that_cannot_be_read_costs_only_itself() {
        let bent = format!("{V4}wlp8s0\tnonsense\n\n");
        assert_eq!(
            rows_v4(&bent).len(),
            3,
            "the three good rows are still there"
        );
        assert!(rows_v6("").is_empty());
        assert!(rows_v4("Iface\tDestination\n").is_empty());
    }

    /// End to end over the chooser, with the numbers this machine has:
    /// given both tables and an adapter of ours, the way out is the one the
    /// machine really uses — and it is chosen per family from that family's
    /// rows.
    #[test]
    fn the_way_out_is_chosen_from_what_the_tables_say() {
        let roads: Vec<Road> = rows_v4(V4)
            .into_iter()
            .chain(rows_v6(V6))
            .enumerate()
            .map(|(i, row)| Road {
                // Numbered as Linux does: one number per interface, both
                // families, so the two wlp8s0 rows share theirs.
                interface: if row.iface == "wlp8s0" {
                    3
                } else {
                    4 + i as u32
                },
                luid: if row.iface == "wlp8s0" {
                    3
                } else {
                    4 + i as u64
                },
                bits: row.bits,
                metric: row.metric,
                loopback: row.iface == "lo",
                v6: row.v6,
                connected: true,
            })
            .collect();
        assert_eq!(out_by(&roads, 99, false), Some(3), "IPv4 leaves by wlp8s0");
        assert_eq!(out_by(&roads, 99, true), Some(3), "and so does IPv6");
        assert_eq!(
            out_by(&roads, 3, false),
            None,
            "and if wlp8s0 were ours there would be no way out that is not"
        );
    }

    /// The real tables of the machine this runs on, which is the one thing
    /// the parsing tests cannot say: that the columns are where this kernel
    /// puts them, and that the names in them resolve to numbers.
    ///
    /// The carrier policy: a plain `1`, and nothing else.
    #[cfg(target_os = "linux")]
    #[test]
    fn only_a_carrier_that_says_one_is_a_way_out() {
        assert!(says_up(Some("1\n")));
        assert!(says_up(Some("1")));
        // A device up with its carrier down, which is what an unplugged
        // cable leaves and what the tables cannot see.
        assert!(!says_up(Some("0\n")));
        // Unreadable, empty, or something else: excluded, on purpose. A
        // family with no way out at all is refused loudly in `upstream`,
        // where a dead interface kept is pinned to and waited on.
        assert!(!says_up(None));
        assert!(!says_up(Some("")));
        assert!(!says_up(Some("yes")));
        assert!(!says_up(Some("10")));
    }

    /// No road came from an interface whose carrier is down, asked of this
    /// machine both ways round: the roads, and every interface it has.
    ///
    /// Before the carrier was read this failed with a device up, carrier
    /// down, and a default route of its own -- which is how the bug was
    /// found.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "asks this machine about its own interfaces"]
    fn no_way_out_is_an_interface_the_kernel_is_ignoring() {
        let roads = roads_now().expect("the tables read");
        let indices: Vec<u32> = roads.iter().map(|road| road.interface).collect();
        let mut looked = 0;
        for entry in std::fs::read_dir("/sys/class/net").expect("the interfaces") {
            let name = entry.expect("an entry").file_name();
            let name = name.to_string_lossy();
            let carrier = std::fs::read_to_string(format!("/sys/class/net/{name}/carrier")).ok();
            if says_up(carrier.as_deref()) {
                continue;
            }
            looked += 1;
            let Some(index) = index_of(&name) else {
                continue;
            };
            assert!(
                !indices.contains(&index),
                "{name} ({index}) has carrier {carrier:?} and is a way out: {roads:?}"
            );
        }
        println!("{looked} interface(s) with no carrier, none of them a road");
    }

    /// `#[ignore]`d because it asserts about the machine rather than about
    /// the code — a build box with no network has no way out and would fail
    /// it honestly. Run it beside `ip -4 route show default` and the index
    /// it prints should be that interface's:
    ///
    /// ```sh
    /// cargo test -p tracewing-proxy --release --lib \
    ///   roads::linux_tables::the_real_tables -- --ignored --nocapture
    /// ```
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "asks this machine about its own routes"]
    fn the_real_tables_name_a_way_out() {
        let roads = roads_now().expect("the tables read");
        let v4 = out_by(&roads, u64::MAX, false);
        let v6 = out_by(&roads, u64::MAX, true);
        println!("roads: {}", roads.len());
        for road in &roads {
            println!(
                "  if {} bits {} metric {} loopback {} v6 {}",
                road.interface, road.bits, road.metric, road.loopback, road.v6
            );
        }
        println!("way out v4: {v4:?}, v6: {v6:?}");
        assert!(
            v4.is_some(),
            "a machine on a network has an IPv4 way out that is not ours"
        );
        assert!(
            roads.iter().any(|road| road.loopback),
            "and the loopback is in the table, so the flag was read rather than guessed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mine(luid: u64, addr: &str) -> Mine {
        Mine {
            luid,
            addr: addr.parse().unwrap(),
            settled: true,
            skip_as_source: false,
        }
    }

    /// The address an echo leaves from: this interface, this family, and
    /// none of the three kinds that cannot carry one.
    #[test]
    fn the_address_to_echo_from() {
        let ours = 0x0035_0080_0000_0000;
        let other = 0x0099_0080_0000_0000;
        let held = vec![
            // Another interface's, which is not ours to send from.
            mine(other, "192.0.2.9"),
            // Link-local: reachable only on the wire it is on, so an echo
            // sent from it goes nowhere.
            mine(ours, "169.254.1.1"),
            // Two of ours, and the lower is taken so the choice cannot move
            // between one probe of a trace and the next.
            mine(ours, "198.51.100.20"),
            mine(ours, "198.51.100.7"),
            // The other family's.
            mine(ours, "2001:db8::7"),
        ];
        assert_eq!(
            source_on(&held, ours, false),
            Some("198.51.100.7".parse().unwrap())
        );
        assert_eq!(
            source_on(&held, ours, true),
            Some("2001:db8::7".parse().unwrap()),
            "an echo to an IPv6 address has to leave from one"
        );
        assert_eq!(
            source_on(&held, other, true),
            None,
            "that interface holds no IPv6 address"
        );
    }

    /// The three disqualifications, each on its own, so that none of them is
    /// passing because another happened to apply.
    #[test]
    fn what_cannot_be_a_source() {
        let ours = 0x0035_0080_0000_0000;
        for (what, one) in [
            (
                "still in duplicate address detection",
                Mine {
                    settled: false,
                    ..mine(ours, "198.51.100.7")
                },
            ),
            (
                "marked by Windows as not to be chosen",
                Mine {
                    skip_as_source: true,
                    ..mine(ours, "198.51.100.7")
                },
            ),
            ("link-local in IPv4", mine(ours, "169.254.1.1")),
            ("link-local in IPv6", mine(ours, "fe80::1")),
            // And the edge of `fe80::/10`, which is a ten-bit prefix and not
            // the sixteen a reader assumes: `febf::` is inside it.
            ("the far edge of fe80::/10", mine(ours, "febf::1")),
        ] {
            let v6 = one.addr.is_ipv6();
            assert_eq!(source_on(&[one], ours, v6), None, "{what}");
        }
        // And just outside that prefix, which must *not* be excluded.
        assert_eq!(
            source_on(&[mine(ours, "fec0::1")], ours, true),
            Some("fec0::1".parse().unwrap()),
            "fec0:: is outside fe80::/10 and is a usable source"
        );
    }

    #[test]
    fn nothing_held_is_no_source() {
        assert_eq!(source_on(&[], 0x0035_0080_0000_0000, false), None);
    }

    /// A road whose interface and LUID are the same number, which is only a
    /// convenience: nothing but `the_same_adapter_numbered_differently…`
    /// cares that they are two things.
    fn road(interface: u32, bits: u8, metric: u32) -> Road {
        Road {
            interface,
            luid: interface as u64,
            bits,
            metric,
            loopback: false,
            v6: false,
            connected: true,
        }
    }

    /// What this machine says, with the control in the same run.
    ///
    /// Needs arranging, which is why it is `#[ignore]`d: a second interface
    /// whose link is **down**, holding a route wide enough to be a candidate
    /// and wider than the live interface's -- a `/1` beats a `/0` on reach.
    /// On the VM:
    ///
    /// On the VirtualBox test machine, from the host and then in the guest:
    ///
    /// ```powershell
    /// # host, with the VM off: a second interface to take down
    /// VBoxManage modifyvm windows-11 --nic2 nat --cableconnected2 on
    /// # guest, while its cable is still on
    /// New-NetRoute -InterfaceIndex 20 -DestinationPrefix 0.0.0.0/1 -NextHop 10.0.3.2
    /// # host again
    /// VBoxManage controlvm windows-11 setlinkstate2 off
    /// ```
    ///
    /// What that run said, and it is the whole point of the arrangement:
    /// `without the link check: Some(20); out_by: Some(5)` -- 20 being the
    /// interface whose cable had been pulled, and whose fourteen rows all
    /// came back `connected false`.
    ///
    /// The control is computed here rather than in another run: the same
    /// ordering with the one new filter left out. So a passing run says both
    /// halves -- that without the link check a dead interface **wins**, and
    /// that with it the answer is a live one. A run on a machine that is not
    /// arranged says so instead of quietly passing.
    #[cfg(windows)]
    #[test]
    #[ignore = "needs a second interface whose link is down"]
    fn a_dead_interface_that_would_otherwise_win_is_not_the_way_out() {
        let roads = roads_now().expect("the tables read");
        for road in &roads {
            println!(
                "  if {} luid {:#018x} bits {} metric {} loopback {} v6 {} connected {}",
                road.interface,
                road.luid,
                road.bits,
                road.metric,
                road.loopback,
                road.v6,
                road.connected
            );
        }
        let dead: Vec<u32> = roads
            .iter()
            .filter(|road| !road.connected)
            .map(|road| road.interface)
            .collect();
        // Nothing of ours is held in this test, so no LUID is excluded.
        let out = out_by(&roads, 0, false);
        let blind = roads
            .iter()
            .filter(|road| !road.v6 && !road.loopback && road.bits <= WIDE_ENOUGH)
            .min_by_key(|road| (std::cmp::Reverse(road.bits), road.metric, road.interface))
            .map(|road| road.interface);
        println!("dead interfaces: {dead:?}");
        println!("without the link check: {blind:?}; out_by: {out:?}");
        assert!(
            !dead.is_empty(),
            "this test needs an interface whose link is down; see the doc comment"
        );
        let blind = blind.expect("something is wide enough to be a way out");
        assert!(
            dead.contains(&blind),
            "the arrangement is not set up: without the link check the answer is \
             interface {blind}, which is not one of the dead ones {dead:?}"
        );
        let out = out.expect("a live interface is still a way out");
        assert!(
            !dead.contains(&out),
            "out_by chose interface {out}, whose link is down"
        );
    }

    /// A route on an interface whose link is down is not a way out, even
    /// when the table makes it the best one going.
    ///
    /// The arrangement is the one measured on Windows: the dead interface
    /// wins every tie-break `out_by` has -- as wide, as cheap, lower index --
    /// because a pulled cable changes none of them. On that machine the live
    /// interface happened to hold the lower index and so was chosen by luck;
    /// with the numbers the other way round, this is what it does.
    #[test]
    fn a_route_on_a_dead_interface_is_not_a_way_out() {
        let dead = Road {
            connected: false,
            ..road(3, 0, 0)
        };
        assert_eq!(
            out_by(&[dead, road(7, 0, 0)], 99, false),
            Some(7),
            "the live interface, although it is dearer by every number here"
        );
        assert_eq!(
            out_by(&[dead], 99, false),
            None,
            "and with nothing else there is no way out at all, which refuses \
             the connection and says why, rather than hanging on a dead one"
        );
    }

    #[test]
    fn the_one_default_route_is_the_way_out() {
        assert_eq!(out_by(&[road(7, 0, 25)], 99, false), Some(7));
    }

    /// The whole reason this function exists rather than `GetBestRoute2`.
    #[test]
    fn our_own_adapter_is_never_the_way_out() {
        // Ours reaches further and costs nothing, and is still not it.
        let roads = [road(4, 1, 0), road(4, 1, 0), road(7, 0, 25)];
        assert_eq!(
            out_by(&roads, 4, false),
            Some(7),
            "the adapter we hold is exactly where the loop is"
        );
    }

    #[test]
    fn the_cheaper_of_two_that_reach_as_far() {
        assert_eq!(out_by(&[road(7, 0, 25), road(8, 0, 5)], 99, false), Some(8));
    }

    /// What the operating system forwards by, so what this has to answer.
    #[test]
    fn reaching_further_beats_costing_less() {
        assert_eq!(
            out_by(&[road(7, 0, 1), road(8, 1, 500)], 99, false),
            Some(8)
        );
    }

    #[test]
    fn a_route_that_does_not_reach_far_enough_is_not_a_way_out() {
        // A LAN: it reaches the neighbours, not the internet.
        assert_eq!(out_by(&[road(7, 24, 5)], 99, false), None);
    }

    #[test]
    fn loopback_is_not_a_way_out() {
        let home = Road {
            loopback: true,
            connected: true,
            ..road(1, 0, 0)
        };
        assert_eq!(out_by(&[home], 99, false), None);
    }

    #[test]
    fn each_family_is_asked_separately() {
        let six = Road {
            v6: true,
            connected: true,
            ..road(9, 0, 10)
        };
        let roads = [road(7, 0, 25), six];
        assert_eq!(out_by(&roads, 99, false), Some(7));
        assert_eq!(out_by(&roads, 99, true), Some(9));
    }

    /// Why this takes a LUID. An adapter can be numbered differently in the
    /// two families, and an index can only ever exclude one of them — the
    /// other family's connections would then be routed into the adapter we
    /// are holding, which is the loop.
    #[test]
    fn the_same_adapter_numbered_differently_in_each_family_is_still_ours() {
        let ours = 0x1234_5678_9abc_def0;
        let mine_v4 = Road {
            interface: 11,
            luid: ours,
            connected: true,
            ..road(11, 0, 0)
        };
        let mine_v6 = Road {
            interface: 22,
            luid: ours,
            v6: true,
            connected: true,
            ..road(22, 0, 0)
        };
        let other_v4 = road(7, 0, 25);
        let other_v6 = Road {
            v6: true,
            connected: true,
            ..road(8, 0, 25)
        };
        let roads = [mine_v4, mine_v6, other_v4, other_v6];
        assert_eq!(out_by(&roads, ours, false), Some(7));
        assert_eq!(
            out_by(&roads, ours, true),
            Some(8),
            "excluding by index would have left this family pointing at ours"
        );
    }

    #[test]
    fn nothing_but_ours_is_none_rather_than_ours() {
        assert_eq!(out_by(&[road(4, 0, 0)], 4, false), None);
    }

    /// Two equal candidates must not give one answer today and the other
    /// tomorrow because the table came back in a different order.
    #[test]
    fn the_answer_does_not_depend_on_the_order_the_table_came_in() {
        let a = road(7, 0, 25);
        let b = road(8, 0, 25);
        assert_eq!(out_by(&[a, b], 99, false), out_by(&[b, a], 99, false));
    }
}
