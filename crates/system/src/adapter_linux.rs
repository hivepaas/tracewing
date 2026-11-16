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

/// Makes the adapter, asking for administrator rights.
///
/// Every call asks, because the script is the whole answer rather than a
/// difference against what is there: reading back whether the addresses, the
/// routes and the knobs are all as they should be is a separate piece, and
/// until it exists this is called when somebody switches the mode on and not
/// on the way to anything else.
pub fn make_adapter() -> Result<()> {
    crate::os::run_as_admin(&script(users_id()))
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

    /// Runs the real thing: asks for a password, makes the adapter, checks the
    /// kernel agrees, and deletes it again. `#[ignore]`d because it needs a
    /// person at the dialog and changes the machine's routing table, like the
    /// live device test in `tun_linux.rs`.
    ///
    /// `cargo test -p tracewing-system -- --ignored the_adapter`
    #[test]
    #[ignore = "needs administrator rights and a person to grant them"]
    fn the_adapter_is_made_and_the_routes_are_dead_until_it_is_held() {
        make_adapter().expect("make the adapter");
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
        crate::os::run_as_admin(&format!("ip tuntap del dev {DEVICE} mode tun"))
            .expect("delete the adapter");
    }
}
