//! Linux: the one privileged step local mode needs, and the only one.
//!
//! Capturing itself is unprivileged. tracewing opens a TUN device that already
//! exists and belongs to this user, reads packets off it, and pins its own
//! connections to the real way out with a socket option that needs no
//! capability either. What needs `CAP_NET_ADMIN` is *making* that device: the
//! adapter, its two addresses, its two default routes. That is this file, it
//! runs once, and it goes through the same polkit dialog the certificate and
//! the HTTP/3 block already ask through.
//!
//! What makes once enough is that the kernel keeps the addresses and the
//! routes when the holder lets go, and marks the routes `dead linkdown`
//! instead of using them. So holding the device *is* the switch: hold it and
//! the machine routes through tracewing, let go — or crash, or be killed —
//! and the machine routes as it did before, with nothing to put back.
//!
//! That only works if the two `sysctl` lines come before the routes, which is
//! the one ordering trap here and why [`script`] is a function with a test
//! rather than a string with a comment. The measurements behind all of it are
//! in `docs/design/capture-modes.md`.

use anyhow::Result;
use std::{
    net::Ipv4Addr,
    os::fd::{AsRawFd, FromRawFd},
};

/// The adapter's name, which is also a sysctl path component, so no dots.
pub const DEVICE: &str = "tracewing";
/// The address the adapter answers on, and the source address of what
/// tracewing sends back down it.
pub const ADDRESS_V4: &str = "10.77.0.1/24";
/// The same for IPv6, out of `fd00::/8`, which is private by definition.
pub const ADDRESS_V6: &str = "fd00:77::1/64";
/// Below any real default route, so that ours wins while the device is held.
/// This machine's Wi-Fi default is 600 for both families, and NetworkManager's
/// floor is 50 higher than this.
pub const METRIC: u32 = 50;

/// The privileged step, as a shell script for `sh -c`.
///
/// `user` is the uid the device belongs to, so that tracewing can open it
/// afterwards without being root.
///
/// **It writes nothing that outlives the kernel**, which is the question
/// somebody asks about any step that ran as an administrator: is there
/// something of it left to clean up. These nine lines are an `ip tuntap`,
/// two `sysctl -qw`, two `ip addr add`, an `ip link set up` and two
/// `ip route add` -- live kernel state, every one of them. `ip tuntap add`
/// makes a device persistent across **the process that opened it**, which
/// is the trick the whole mode rests on, and not across a boot.
///
/// What is this file's to answer rather than the kernel's is whether
/// tracewing puts any of it back, and it does not. Nowhere in the tree is
/// there a unit, a udev rule, a network profile or anything under
/// `/etc/network`, and the knobs are set with `sysctl -qw` rather than from
/// `/etc/sysctl.d`, so the next boot does not set them either. In
/// production the script is run from one place, [`make_adapter`]; the other
/// `run_as_admin` for an adapter in this file is inside its tests.
///
/// `the_step_writes_nothing_that_outlives_the_kernel`, below, is what keeps
/// that true rather than merely written down: every command in the script,
/// and not merely the first on each line, has to be `ip` or `sysctl`.
///
/// What a reader is told about it, and the one thing this reading cannot
/// rule out -- somebody else's software keeping a profile of a device it
/// saw -- are in `setup_page::local_when_done`, which is where the question
/// is actually asked.
pub fn script(user: u32) -> String {
    let (device, metric) = (DEVICE, METRIC);
    format!(
        "\
set -e
# Deleting first is what makes this repeatable: switching the mode on twice
# must not fail, and a device that was not there is not an error. Nothing
# holds the device at this point, since this runs before tracewing opens it.
ip tuntap del dev {device} mode tun 2>/dev/null || :
ip tuntap add dev {device} mode tun user {user}
# Before the routes, not beside them. A knob set after the carrier has
# already dropped leaves an IPv4 route marked `linkdown` and *still chosen*,
# while IPv6 falls through; set first, both families fall through alike.
sysctl -qw net.ipv4.conf.{device}.ignore_routes_with_linkdown=1
sysctl -qw net.ipv6.conf.{device}.ignore_routes_with_linkdown=1
ip addr add {ADDRESS_V4} dev {device}
ip addr add {ADDRESS_V6} dev {device}
ip link set {device} up
ip route add default dev {device} metric {metric}
ip -6 route add default dev {device} metric {metric}
"
    )
}

/// What the adapter is, read without asking anybody for anything.
///
/// Every field comes from a file or an `ioctl` an ordinary user may read, so
/// the mode can look before it asks: a password dialog for a machine that is
/// already set up is how people learn to click dialogs away.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Look {
    /// A persistent TUN device by that name exists.
    pub device: bool,
    /// ...and belongs to this user, so tracewing can open it.
    pub ours: bool,
    /// Its `IFF_UP` is set. The carrier is not part of this: that follows
    /// whoever holds the device, which is the point of the whole shape.
    pub up: bool,
    /// Both `ignore_routes_with_linkdown` knobs are on. Without them a
    /// default route down an unheld adapter is still used, and the packets
    /// go nowhere.
    pub knobs: bool,
    /// Both addresses are on it, exactly as [`ADDRESS_V4`] and
    /// [`ADDRESS_V6`] spell them.
    pub addresses: bool,
    /// Both default routes are installed down it.
    pub routes: bool,
}

impl Look {
    /// Nothing left for the privileged step to do.
    pub fn ready(&self) -> bool {
        let Self {
            device,
            ours,
            up,
            knobs,
            addresses,
            routes,
        } = self;
        *device && *ours && *up && *knobs && *addresses && *routes
    }

    /// What is missing, for an error or a log to name rather than say
    /// "something".
    pub fn missing(&self) -> Vec<&'static str> {
        [
            (!self.device, "the device"),
            (self.device && !self.ours, "the device's owner"),
            (!self.up, "the link"),
            (!self.knobs, "the linkdown knobs"),
            (!self.addresses, "the addresses"),
            (!self.routes, "the routes"),
        ]
        .into_iter()
        .filter_map(|(missing, what)| missing.then_some(what))
        .collect()
    }
}

/// Reads the adapter, every part of it unprivileged.
pub fn look() -> Look {
    let sys = |what: &str| std::fs::read_to_string(format!("/sys/class/net/{DEVICE}/{what}")).ok();
    let knob = |family: &str| {
        std::fs::read_to_string(format!(
            "/proc/sys/net/{family}/conf/{DEVICE}/ignore_routes_with_linkdown"
        ))
        .is_ok_and(|on| on.trim() == "1")
    };
    let proc =
        |what: &str| std::fs::read_to_string(format!("/proc/net/{what}")).unwrap_or_default();
    let flags = sys("tun_flags");
    Look {
        device: flags.as_deref().is_some_and(is_tun),
        ours: sys("owner").is_some_and(|owner| owner.trim() == users_id().to_string()),
        up: sys("flags").as_deref().is_some_and(is_up),
        knobs: knob("ipv4") && knob("ipv6"),
        addresses: address_v4() == Some(wanted_v4()) && has_address_v6(&proc("if_inet6")),
        routes: has_default_v4(&proc("route")) && has_default_v6(&proc("ipv6_route")),
    }
}

/// `IFF_TUN | IFF_PERSIST`: a tap device, or one that goes when its holder
/// does, is not the adapter this made.
fn is_tun(tun_flags: &str) -> bool {
    const IFF_TUN: u32 = 0x0001;
    const IFF_PERSIST: u32 = 0x0800;
    let flags = tun_flags
        .trim()
        .strip_prefix("0x")
        .unwrap_or(tun_flags.trim());
    u32::from_str_radix(flags, 16)
        .is_ok_and(|flags| flags & (IFF_TUN | IFF_PERSIST) == IFF_TUN | IFF_PERSIST)
}

/// `IFF_UP`, which `/sys/class/net/<dev>/flags` carries in its lowest bit.
fn is_up(flags: &str) -> bool {
    let flags = flags.trim().strip_prefix("0x").unwrap_or(flags.trim());
    u32::from_str_radix(flags, 16).is_ok_and(|flags| flags & 0x1 != 0)
}

/// Routes in the two `/proc` tables are **up** whether the kernel is using
/// them or not: measured with nobody holding the device, the default route
/// is there with `RTF_UP` set and nothing says it is being ignored. That is
/// what makes it readable before the mode is switched on, and why this
/// answers "installed" rather than "in use".
const RTF_UP: u32 = 0x0001;

fn up_flags(field: &str) -> bool {
    u32::from_str_radix(field, 16).is_ok_and(|flags| flags & RTF_UP != 0)
}

/// `Iface Destination Gateway Flags RefCnt Use Metric Mask ...`, with a
/// header line first.
fn has_default_v4(table: &str) -> bool {
    table.lines().skip(1).any(|line| {
        let field: Vec<&str> = line.split_whitespace().collect();
        field.len() > 7
            && field[0] == DEVICE
            && field[1] == "00000000"
            && field[7] == "00000000"
            && up_flags(field[3])
    })
}

/// Ten fields, the prefix length in hex, the interface last.
fn has_default_v6(table: &str) -> bool {
    table.lines().any(|line| {
        let field: Vec<&str> = line.split_whitespace().collect();
        field.len() == 10
            && field[9] == DEVICE
            && field[1] == "00"
            && field[0].bytes().all(|b| b == b'0')
            && up_flags(field[8])
    })
}

/// `<address> <index> <prefix> <scope> <flags> <name>`, the address and the
/// prefix in hex.
fn has_address_v6(table: &str) -> bool {
    let (wanted, bits) = wanted_v6();
    table.lines().any(|line| {
        let field: Vec<&str> = line.split_whitespace().collect();
        field.len() == 6
            && field[5] == DEVICE
            && field[0] == wanted
            && u32::from_str_radix(field[2], 16) == Ok(bits)
    })
}

/// [`ADDRESS_V4`] as the address and the mask an `ioctl` answers with.
fn wanted_v4() -> (Ipv4Addr, Ipv4Addr) {
    let (address, bits) = ADDRESS_V4.split_once('/').expect("a prefix length");
    let bits: u32 = bits.parse().expect("a number");
    let mask = u32::MAX.checked_shl(32 - bits).unwrap_or(0);
    (address.parse().expect("an address"), Ipv4Addr::from(mask))
}

/// [`ADDRESS_V6`] as `/proc/net/if_inet6` spells it, and its prefix length.
fn wanted_v6() -> (String, u32) {
    let (address, bits) = ADDRESS_V6.split_once('/').expect("a prefix length");
    let address: std::net::Ipv6Addr = address.parse().expect("an address");
    let hex = address
        .octets()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            use std::fmt::Write;
            let _ = write!(hex, "{byte:02x}");
            hex
        });
    (hex, bits.parse().expect("a number"))
}

/// The address and mask on the adapter, by the two `ioctl`s that answer for
/// IPv4. There is no file to read them from: `/proc/net/route` shows the
/// subnet an address implies, which would take `10.77.0.5/24` for
/// `10.77.0.1/24`, and the stack needs the address itself.
fn address_v4() -> Option<(Ipv4Addr, Ipv4Addr)> {
    address_v4_of(DEVICE)
}

/// The same for any interface, so that the call itself can be tested on one
/// every machine has.
fn address_v4_of(name: &str) -> Option<(Ipv4Addr, Ipv4Addr)> {
    const SIOCGIFADDR: libc::c_ulong = 0x8915;
    const SIOCGIFNETMASK: libc::c_ulong = 0x891b;
    /// `struct ifreq`: a name, and a `sockaddr_in` where the answer goes.
    #[repr(C)]
    struct Request {
        name: [libc::c_char; libc::IFNAMSIZ],
        address: libc::sockaddr_in,
        rest: [u8; 8],
    }
    // SAFETY: a datagram socket, which these two calls only read from.
    let socket = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if socket < 0 {
        return None;
    }
    // Closed however this returns.
    let socket = unsafe { std::os::fd::OwnedFd::from_raw_fd(socket) };
    let ask = |what: libc::c_ulong| {
        let mut request = Request {
            name: [0; libc::IFNAMSIZ],
            // SAFETY: a sockaddr_in of zeros is a sockaddr_in.
            address: unsafe { std::mem::zeroed() },
            rest: [0; 8],
        };
        for (slot, byte) in request.name.iter_mut().zip(name.as_bytes()) {
            *slot = *byte as libc::c_char;
        }
        // SAFETY: the socket is ours and open, and the request is the shape
        // these calls write into, with its name ending in a zero.
        let got = unsafe { libc::ioctl(socket.as_raw_fd(), what, &raw mut request) };
        (got == 0).then(|| Ipv4Addr::from(u32::from_be(request.address.sin_addr.s_addr)))
    };
    Some((ask(SIOCGIFADDR)?, ask(SIOCGIFNETMASK)?))
}

/// Makes the adapter if [`look`] says anything is missing, asking for
/// administrator rights only then.
///
/// The script is still the whole answer rather than a difference against
/// what is there -- it deletes the device and makes it again -- so this is
/// all or nothing: one part missing means the whole step runs.
pub fn make_adapter() -> Result<()> {
    if look().ready() {
        return Ok(());
    }
    crate::os::run_as_admin("make tracewing's network adapter", &script(users_id()))
}

/// This user, whom the device must belong to.
fn users_id() -> u32 {
    // SAFETY: a call that reads nothing and cannot fail.
    unsafe { libc::geteuid() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ordering the measurement settled, as a test rather than a comment:
    /// both knobs above both routes. Reordered, an IPv4 default route down an
    /// adapter nobody holds stays in use and the machine blackholes.
    #[test]
    fn the_knobs_are_set_before_the_routes() {
        let script = script(1000);
        let at = |what: &str| script.find(what).unwrap_or_else(|| panic!("no {what}"));
        let knobs = [
            at("sysctl -qw net.ipv4.conf.tracewing.ignore_routes_with_linkdown=1"),
            at("sysctl -qw net.ipv6.conf.tracewing.ignore_routes_with_linkdown=1"),
        ];
        let routes = [at("ip route add default"), at("ip -6 route add default")];
        assert!(
            knobs.iter().max() < routes.iter().min(),
            "the knobs must come first:\n{script}"
        );
    }

    /// `sh -c` answers with the last command's status, so without this a
    /// failure anywhere above the last line would look like success.
    #[test]
    fn the_script_stops_at_the_first_failure() {
        assert!(script(1000).starts_with("set -e\n"));
    }

    /// The delete is the only line allowed to fail, and it has to be: the
    /// first run has nothing to delete.
    #[test]
    fn only_the_delete_may_fail() {
        let script = script(1000);
        let forgiving: Vec<&str> = script
            .lines()
            .filter(|line| line.ends_with("|| :") || line.contains("2>/dev/null"))
            .collect();
        assert_eq!(
            forgiving,
            [format!(
                "ip tuntap del dev {DEVICE} mode tun 2>/dev/null || :"
            )]
        );
    }

    /// The claim in [`script`]'s own documentation, as something that can
    /// fail. A whitelist rather than a list of forbidden paths, because the
    /// ways to write a file outnumber the ways to ask the kernel for an
    /// interface: a `tee`, an `nmcli` or a `systemctl` fails here whatever
    /// it is spelled like.
    ///
    /// Three questions, because the first two were each found passing
    /// something the comment above claimed they caught. **Who may be
    /// commanded**, below. **Whether anything is written beside the
    /// command** -- a redirection or a substitution needs no forbidden verb
    /// and no forbidden path, and `ip link set … up > /var/lib/x` was green
    /// until it was asked. **Whether a path under `/etc` appears** at all,
    /// which neither of the others would catch in `sysctl -p
    /// /etc/sysctl.d/…`: an allowed verb, no redirection, a file somebody
    /// else put there.
    ///
    /// **Every command, not every line.** Checking the start of the line
    /// would pass `ip link set tracewing up && nmcli connection add …`,
    /// which begins with an allowed verb and then does the thing this test
    /// exists to forbid. Banning the operators instead is not open either:
    /// the first line of the script carries `||` and a redirection already,
    /// and has to. So the line is split on them and every piece must stand
    /// on its own -- which leaves `2>/dev/null` inside the `ip` piece where
    /// it belongs, and leaves `:` as itself, the one command that is allowed
    /// because it does nothing.
    #[test]
    fn the_step_writes_nothing_that_outlives_the_kernel() {
        let script = script(1000);
        let doing: Vec<&str> = script
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#') && *line != "set -e")
            .collect();
        assert!(!doing.is_empty(), "the script does nothing:\n{script}");
        for line in &doing {
            // Every operator is a separator and the empty pieces an `||`
            // or an `&&` leaves behind are dropped below, so the pair does
            // not have to be told from the single.
            let split = line.replace(['|', ';', '&'], "\n");
            let pieces: Vec<&str> = split
                .lines()
                .map(str::trim)
                .filter(|piece| !piece.is_empty())
                .collect();
            assert!(!pieces.is_empty(), "an empty command in: {line}");
            for piece in pieces {
                assert!(
                    piece.starts_with("ip ") || piece.starts_with("sysctl ") || piece == ":",
                    "only the kernel may be asked for anything: {piece}\n  in: {line}"
                );
            }
        }
        // Who may be commanded is only half of it: a line whose every
        // piece is an `ip` can still write a file beside the command, and
        // nothing about that file has to be under /etc to outlive a boot.
        // Both of these were green once, with `> /var/lib/…` and with
        // `$(touch /var/lib/…)`.
        //
        // Asked of the commands rather than of the whole script, because
        // the comments in it are prose: the one above `sysctl` says
        // `linkdown` in backticks, and reading the script whole turned this
        // into a test that failed on the script it was written for.
        let commands = doing.join("\n");
        assert!(
            !commands.replace("2>/dev/null", "").contains('>'),
            "a redirection writes a file, wherever it points:\n{script}"
        );
        assert!(
            !commands.contains("$(") && !commands.contains('`'),
            "a substitution runs a command the whitelist never sees:\n{script}"
        );
        // And a path under /etc needs neither: `sysctl -p /etc/sysctl.d/…`
        // is an allowed verb reading a file somebody put there.
        assert!(
            !commands.contains("/etc"),
            "a path under /etc outlives the kernel:\n{script}"
        );
    }

    /// The device's name is a component of the two sysctl paths, so a dot in
    /// it would address a different interface, or nothing.
    #[test]
    fn the_name_can_be_a_sysctl_path() {
        assert!(!DEVICE.contains('.'), "{DEVICE} cannot be a sysctl path");
        assert!(DEVICE.len() < 16, "{DEVICE} is too long for IFNAMSIZ");
    }

    /// The uid is the one thing the script is given, and a device owned by
    /// somebody else is a device tracewing cannot open.
    #[test]
    fn the_device_is_given_to_the_user_asked_for() {
        assert!(script(4242).contains("mode tun user 4242"));
        assert!(!script(4242).contains("user 0\n"));
    }

    /// What this machine's kernel said, with the adapter made by the script
    /// above and **nobody holding the device** -- which is the state the
    /// mode is looked at in, and the state in which the kernel is ignoring
    /// those routes. The tables say nothing about that, which is why these
    /// read as "installed".
    mod as_the_kernel_said_it {
        pub const TUN_FLAGS: &str = "0x1801\n";
        pub const FLAGS: &str = "0x1091\n";
        pub const ROUTE: &str = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
tracewing\t00000000\t00000000\t0001\t0\t0\t50\t00000000\t0\t0\t0
tracewing\t00004D0A\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0
";
        pub const IPV6_ROUTE: &str = "\
fd000077000000000000000000000000 40 00000000000000000000000000000000 00 00000000000000000000000000000000 00000100 00000001 00000000 00000001 tracewing
00000000000000000000000000000000 00 00000000000000000000000000000000 00 00000000000000000000000000000000 00000032 00000001 00000000 00000001 tracewing
fd000077000000000000000000000001 80 00000000000000000000000000000000 00 00000000000000000000000000000000 00000000 00000002 00000000 80200001 tracewing
";
        pub const IF_INET6: &str = "fd000077000000000000000000000001 29 40 00 80 tracewing\n";
    }

    /// The lines this machine really had, taken as they came.
    #[test]
    fn what_the_kernel_said_reads_as_an_adapter_that_is_ready() {
        use as_the_kernel_said_it::*;
        assert!(is_tun(TUN_FLAGS), "{TUN_FLAGS}");
        assert!(is_up(FLAGS), "{FLAGS}");
        assert!(has_default_v4(ROUTE), "{ROUTE}");
        assert!(has_default_v6(IPV6_ROUTE), "{IPV6_ROUTE}");
        assert!(has_address_v6(IF_INET6), "{IF_INET6}");
    }

    /// The near misses, each of which would otherwise pass for the adapter
    /// and leave the mode switched on over somebody else's device.
    #[test]
    fn what_is_nearly_the_adapter_is_not_the_adapter() {
        // A device that goes when its holder does: the addresses and routes
        // would go with it, which is the whole shape gone.
        assert!(!is_tun("0x1001\n"), "not persistent");
        // A tap device is not a tun device.
        assert!(!is_tun("0x1802\n"), "IFF_TAP");
        assert!(!is_tun("nonsense"));
        assert!(!is_up("0x1090\n"), "IFF_UP clear");
        // Another interface's default route is not ours.
        assert!(!has_default_v4(
            &as_the_kernel_said_it::ROUTE.replace("tracewing", "wlp8s0")
        ));
        assert!(!has_default_v6(
            &as_the_kernel_said_it::IPV6_ROUTE.replace("tracewing", "wlp8s0")
        ));
        // The route down our adapter that is not a default route: the
        // connected /24 from the address, which is the second line of the
        // real table and must not be mistaken for the first.
        assert!(!has_default_v4(
            "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
tracewing\t00004D0A\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0
"
        ));
        // `0.0.0.0/1` starts at the same address as a default route and is
        // not one -- and it is not a hypothetical: the two `/1` halves are
        // how the Windows side covers everything while leaving the real
        // default alone. The mask is what tells them apart, and the
        // destination alone does not.
        assert!(!has_default_v4(
            "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
tracewing\t00000000\t00000000\t0001\t0\t0\t50\t00000080\t0\t0\t0
"
        ));
        // `::/1` likewise: all zeros with a prefix length that is not 0.
        assert!(!has_default_v6(
            "00000000000000000000000000000000 01 00000000000000000000000000000000 00 00000000000000000000000000000000 00000032 00000001 00000000 00000001 tracewing\n"
        ));
        // `::/0` is a default route; `fd00:77::/64` is not.
        assert!(!has_default_v6(
            "fd000077000000000000000000000000 40 00000000000000000000000000000000 00 00000000000000000000000000000000 00000100 00000001 00000000 00000001 tracewing\n"
        ));
        // A route that is not up is not a route, as `roads.rs` found with
        // the unreachable `::/0` over `lo`.
        assert!(!has_default_v4(
            "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
tracewing\t00000000\t00000000\t0000\t0\t0\t50\t00000000\t0\t0\t0
"
        ));
        // The link-local address the kernel adds once the device is held is
        // not the address we asked for.
        assert!(!has_address_v6(
            "fe80000000000000b70ea8e21e5790e1 29 40 20 80 tracewing\n"
        ));
        // The right address on the wrong interface.
        assert!(!has_address_v6(
            "fd000077000000000000000000000001 29 40 00 80 wlp8s0\n"
        ));
        // The right address with the wrong prefix.
        assert!(!has_address_v6(
            "fd000077000000000000000000000001 29 80 00 80 tracewing\n"
        ));
        // Nothing at all, which is what a machine that has never run the
        // step says.
        assert!(!has_default_v4("") && !has_default_v6("") && !has_address_v6(""));
    }

    /// The two consts and what the kernel spells them as, kept in step: the
    /// checks above compare against these, so a change to either address
    /// that forgot the other would read every adapter as wrong.
    #[test]
    fn the_addresses_are_spelled_as_the_kernel_spells_them() {
        assert_eq!(
            wanted_v4(),
            (
                std::net::Ipv4Addr::new(10, 77, 0, 1),
                std::net::Ipv4Addr::new(255, 255, 255, 0)
            )
        );
        assert_eq!(
            wanted_v6(),
            ("fd000077000000000000000000000001".to_owned(), 64)
        );
    }

    /// The `ioctl` half, on the one interface every machine has. 127.0.0.1
    /// read as 1.0.0.127 is what a byte order wrong here would look like,
    /// so this covers the order as well as the call.
    #[test]
    fn the_address_call_answers_for_a_real_interface() {
        let (address, mask) = address_v4_of("lo").expect("lo has an address");
        assert_eq!(address, std::net::Ipv4Addr::new(127, 0, 0, 1));
        // Loopback is a /8 by convention, but what matters here is that a
        // mask came back and that it is one.
        assert_eq!(mask.octets()[0], 255, "{mask}");
        // A name nothing answers for is `None`, not a panic and not zeros.
        assert_eq!(address_v4_of("no-such-device-0"), None);
    }

    /// Nothing missing is ready, and what is missing is named rather than
    /// counted.
    #[test]
    fn what_is_missing_is_said() {
        let whole = Look {
            device: true,
            ours: true,
            up: true,
            knobs: true,
            addresses: true,
            routes: true,
        };
        assert!(whole.ready());
        assert_eq!(whole.missing(), Vec::<&str>::new());
        assert!(!Look::default().ready());
        // No device at all: its owner is not a second complaint.
        assert_eq!(
            Look::default().missing(),
            [
                "the device",
                "the link",
                "the linkdown knobs",
                "the addresses",
                "the routes"
            ]
        );
        // A device that is somebody else's is the one thing that cannot be
        // fixed by looking harder, and it is named on its own.
        let theirs = Look {
            ours: false,
            ..whole
        };
        assert!(!theirs.ready());
        assert_eq!(theirs.missing(), ["the device's owner"]);
    }

    /// Reads the real adapter, which has to be there already. No privilege
    /// of its own: that is the claim, so the test proves it by needing
    /// none. Make the adapter first -- by hand, or with the test below --
    /// then:
    ///
    /// `cargo test -p tracewing-system -- --ignored the_look_reads`
    #[test]
    #[ignore = "needs the adapter to have been made already"]
    fn the_look_reads_the_real_adapter() {
        let look = look();
        println!("{look:?}");
        assert!(
            look.ready(),
            "missing: {:?}. Make it first: `cargo test -p tracewing-system -- --ignored \
             the_adapter`",
            look.missing()
        );
    }

    /// Runs the real thing: asks for a password, makes the adapter, checks the
    /// kernel agrees, and deletes it again. `#[ignore]`d because it needs a
    /// person at the dialog and changes the machine's routing table, like the
    /// live device test in `tun_linux.rs`.
    ///
    /// `cargo test -p tracewing-system -- --ignored the_adapter`
    #[test]
    #[ignore = "needs administrator rights and a person to grant them"]
    fn the_adapter_is_made_and_the_routes_are_dead_until_it_is_held() {
        assert!(
            !look().ready(),
            "the adapter is there already, so this would prove nothing: {:?}",
            look()
        );
        make_adapter().expect("make the adapter");
        let made = look();
        assert!(
            made.ready(),
            "missing after making it: {:?}",
            made.missing()
        );
        let routes = std::process::Command::new("ip")
            .args(["route", "show", "default"])
            .output()
            .expect("ip route show");
        let routes = String::from_utf8_lossy(&routes.stdout);
        let ours = routes
            .lines()
            .find(|line| line.contains(DEVICE))
            .unwrap_or_else(|| panic!("no default route down {DEVICE}:\n{routes}"));
        assert!(
            ours.contains("dead linkdown"),
            "nobody is holding it, so it must not be in use: {ours}"
        );
        let owner = std::fs::read_to_string(format!("/sys/class/net/{DEVICE}/owner"))
            .expect("read the owner");
        assert_eq!(owner.trim(), users_id().to_string());
        crate::os::run_as_admin(
            "delete the adapter",
            &format!("ip tuntap del dev {DEVICE} mode tun"),
        )
        .expect("delete the adapter");
        // And gone is read as gone, so that the next run asks again.
        let after = look();
        assert!(!after.ready(), "{after:?}");
        assert_eq!(after.missing()[0], "the device");
    }
}
