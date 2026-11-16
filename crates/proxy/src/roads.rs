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
            road.v6 == v6 && !road.loopback && road.luid != ours && road.bits <= WIDE_ENOUGH
        })
        .min_by_key(|road| (std::cmp::Reverse(road.bits), road.metric, road.interface))
        .map(|road| road.interface)
}

/// This machine's routes as they are now, both families.
#[cfg(windows)]
pub fn roads_now() -> anyhow::Result<Vec<Road>> {
    use windows_sys::Win32::{
        Foundation::NO_ERROR,
        NetworkManagement::IpHelper::{FreeMibTable, GetIpForwardTable2, MIB_IPFORWARD_TABLE2},
        Networking::WinSock::{AF_INET6, AF_UNSPEC},
    };
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
            })
            .collect()
    };
    // SAFETY: the pointer that call gave us, given back exactly once.
    unsafe { FreeMibTable(table as *const core::ffi::c_void) };
    Ok(roads)
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
        }
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
            ..road(1, 0, 0)
        };
        assert_eq!(out_by(&[home], 99, false), None);
    }

    #[test]
    fn each_family_is_asked_separately() {
        let six = Road {
            v6: true,
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
            ..road(11, 0, 0)
        };
        let mine_v6 = Road {
            interface: 22,
            luid: ours,
            v6: true,
            ..road(22, 0, 0)
        };
        let other_v4 = road(7, 0, 25);
        let other_v6 = Road {
            v6: true,
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
