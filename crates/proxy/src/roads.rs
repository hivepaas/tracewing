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
//! **Nothing watches yet, and that is somebody's job rather than nobody's.**
//! WireGuard for Windows solves the same problem and *watches* for the
//! default-route interface rather than reading it once, because which one
//! that is changes as a machine moves between Wi-Fi, Ethernet and
//! tethering — and with none at all, its outgoing packets are blackholed.
//! Whoever starts the adapter owns keeping the answer current. This answers
//! the question; it does not keep answering it.
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
