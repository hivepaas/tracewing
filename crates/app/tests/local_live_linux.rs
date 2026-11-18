//! The local way in, assembled on a real Linux kernel, in a namespace of its
//! own.
//!
//! Everything else about this way in is tested where it can be: the sieve's
//! rules, the switch's states, the tab's pictures, the script's ordering.
//! What no ordinary test reaches is the assembly -- the privileged step
//! really running, the device really opening, `Apps::spawn`, the sieve and
//! the bind and the echo going into one `Tun`, and the whole of it letting go
//! again.
//!
//! **It ends differently from the Windows one, and that is the point.** There,
//! stopping the capture takes the adapter's rows out of the routing table and
//! the test asserts that none are left. Here they **stay**: the device is
//! persistent, the kernel keeps its addresses and routes when the holder lets
//! go, and marks the routes `dead linkdown` rather than using them. That is
//! what makes holding the device the switch, and what makes a hard kill safe.
//! So this asserts the opposite ending, and then asserts the thing that makes
//! the rows harmless -- that the way out is not ours.
//!
//! # Why it no longer needs a password, or this machine's network
//!
//! It used to ask polkit for rights and make its adapter on the machine it
//! ran on, which meant it could not run at all while anybody else was using
//! the network: for as long as it held the device, this machine's traffic
//! went into a TUN nobody was pumping.
//!
//! It re-executes itself under `unshare --user --net --mount
//! --map-root-user` instead. The kernel grants the capabilities inside a
//! user namespace of our own, so `run_as_admin` takes its `id -u` is `0`
//! road and runs the script directly -- the same nine lines, by the same
//! call, with no dialog -- and the device, the addresses and the routes live
//! in a network namespace that goes when the process does. The trick is
//! TraceW 3's, from `local_live_http3.rs`; the traps are written down in
//! [http3.md](../../../docs/design/http3.md) and not repeated here.
//!
//! # The way out is made, not borrowed, and that is what makes two of these
//! assertions mean anything
//!
//! A fresh network namespace holds `lo` and nothing else, so `out_by` would
//! answer nowhere -- and `assert_ne!(None, Some(ours))` passes while saying
//! nothing at all. Both of the assertions this test exists for are of that
//! shape, so this makes a veth pair to be the way out and asserts the way
//! out **is that pair**, which the old version could not: on a real machine
//! it could only say the answer was not ours.
//!
//! It is also a firmer statement of the thing being tested. The adapter's
//! default route is at metric 50 and the pair's at 600, pinned here rather
//! than inherited from whatever this machine's Wi-Fi happens to be, so the
//! answer is the way out **although ours has the better metric** -- which is
//! the whole of why `connected` has to carry it.
//!
//! **The pair is a stand-in for the route shape, not for reachability.**
//! Both ends are in this namespace, so traffic to the peer's address goes
//! through the local table on `lo` and never out of the pair. For a test
//! that reads `/sys` and `/proc/net/route` that costs nothing. For anything
//! that sends traffic through the way out it is a trap, and the symptom is
//! everything answering 502.
//!
//! ```sh
//! cargo test -p tracewing --release --test local_live_linux -- --nocapture
//! ```

#![cfg(target_os = "linux")]

use tracewing::{
    local_apps::{Local, Picked},
    local_source,
};
use tracewing_proxy::roads;
use tracewing_system::adapter;

/// Set on the inner process, so that it does the work rather than unsharing
/// again for ever.
const IN_OURS: &str = "TRACEWING_LOCAL_LIVE_NS";

/// Where the inner run says it got to the end, which is the only thing that
/// tells the outer one it ran at all.
const WITNESS: &str = "TRACEWING_LOCAL_LIVE_WITNESS";

/// The interface standing in for this machine's real way out.
const WAY_OUT: &str = "wayout";

/// Our own rows, by the index the kernel gives the adapter's name.
fn ours_in(roads: &[roads::Road], ours: u64) -> Vec<roads::Road> {
    roads
        .iter()
        .filter(|road| road.luid == ours)
        .copied()
        .collect()
}

fn run(program: &str, args: &[&str]) {
    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("{program}: {e}"));
    assert!(status.success(), "{program} {args:?} failed: {status}");
}

/// Whether this process is the one that does the work.
///
/// `false` means it was the outer one: it has just run the inner one inside
/// a namespace and asserted that it passed, and the caller should stop. The
/// inner run is asked for by name and `--exact`, so a second test in this
/// file would get a namespace of its own rather than share this one.
///
/// **`--ignored`, and a witness file.** This test is `#[ignore]`d, so
/// without `--ignored` the inner run skips it -- and a run that executed no
/// test exits `0`, so the outer one reported a pass having measured
/// nothing. That is why the status is not trusted on its own: the inner run
/// writes a file as its last act, and not finding it is a failure here. An
/// exit code cannot tell "it passed" from "there was nothing to do".
fn in_our_own_namespace(test: &str) -> bool {
    if std::env::var_os(IN_OURS).is_some() {
        return true;
    }
    let exe = std::env::current_exe().expect("this test binary");
    let witness = std::env::temp_dir().join(format!("tracewing-live-ns-{}", std::process::id()));
    let _ = std::fs::remove_file(&witness);
    let status = std::process::Command::new("unshare")
        .args(["--user", "--net", "--mount", "--map-root-user"])
        .arg(&exe)
        .args([
            test,
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(IN_OURS, "1")
        .env(WITNESS, &witness)
        .status()
        .expect(
            "`unshare` is in util-linux. If this fails on a machine rather than for want of \
             the program, the kernel is refusing unprivileged user namespaces: read \
             /proc/sys/user/max_user_namespaces",
        );
    assert!(
        status.success(),
        "the run inside the namespace failed; its output is above"
    );
    assert!(
        witness.exists(),
        "the run inside the namespace exited {status} without reaching the end of the test, \
         so nothing was measured"
    );
    let _ = std::fs::remove_file(&witness);
    false
}

/// The inner run's last act, read by the outer one. See
/// [`in_our_own_namespace`].
fn reached_the_end() {
    if let Some(path) = std::env::var_os(WITNESS) {
        std::fs::write(path, "ran").expect("say that this run reached the end");
    }
}

/// This namespace's own `/sys`, its loopback, and a way out to be chosen.
///
/// The `sysfs` is the trap worth naming twice: without it `/sys` is still
/// the one the outer namespace mounted, so `Road::connected` and
/// `adapter::look` read **this machine's** interfaces and cannot see the
/// device `ip` has just made.
///
/// The pair's default route is deliberately **worse** than the adapter's,
/// which the script puts at metric 50.
fn the_namespace() {
    run("mount", &["-t", "sysfs", "sysfs", "/sys"]);
    run("ip", &["link", "set", "lo", "up"]);
    run(
        "ip",
        &[
            "link", "add", WAY_OUT, "type", "veth", "peer", "name", "beyond",
        ],
    );
    run("ip", &["addr", "add", "10.99.0.2/24", "dev", WAY_OUT]);
    run("ip", &["addr", "add", "10.99.0.3/24", "dev", "beyond"]);
    // Both ends up, or the pair reads as disconnected and `connected` drops
    // it exactly as it drops the adapter.
    run("ip", &["link", "set", WAY_OUT, "up"]);
    run("ip", &["link", "set", "beyond", "up"]);
    run("ip", &["addr", "add", "fd00:99::2/64", "dev", WAY_OUT]);
    run("ip", &["addr", "add", "fd00:99::3/64", "dev", "beyond"]);
    // Device routes rather than `via` the peer: both ends are in this
    // namespace, so the peer's address is a local address and the kernel
    // refuses it as a gateway for IPv6 ("Gateway can not be a local
    // address"). It leaves the two default rows differing in exactly the
    // two things this test is about, carrier and metric.
    run(
        "ip",
        &["route", "add", "default", "dev", WAY_OUT, "metric", "600"],
    );
    run(
        "ip",
        &[
            "-6", "route", "add", "default", "dev", WAY_OUT, "metric", "600",
        ],
    );
}

/// The whole assembly: it comes up, it is on the machine, and letting go
/// leaves it harmless.
///
/// **Still `#[ignore]`d, and the reason is measured.** It no longer wants a
/// password or this machine's network, so running it is cheap and safe --
/// but on Ubuntu 24.04 and later it runs only where AppArmor lets it:
///
/// ```text
/// unshare: write failed /proc/self/uid_map: Operation not permitted
/// ```
///
/// `kernel.apparmor_restrict_unprivileged_userns` is the cause, measured
/// both ways on the CI runner: with it at 1 the line above, with
/// `sysctl -w …=0` the same command exits 0, and set back to 1 it fails
/// again. The mechanism is in `/etc/apparmor.d/unprivileged_userns`, the
/// profile an unconfined process is **transitioned into** when it makes an
/// unprivileged user namespace: it says `allow userns` and
/// `audit deny capability`, so the namespace is made and then the uid_map
/// write is refused. That is exactly what is seen -- `unshare --user` alone
/// exits 0 and `--map-root-user` does not.
///
/// **It is not only CI, which is why this is ignored rather than gated on
/// the runner.** The same failure reproduces on a developer machine under
/// `aa-exec -p unconfined`. What made it pass while this was written is that
/// the shell was VS Code's, and Ubuntu ships `/etc/apparmor.d/code`, whose
/// profile grants `userns` -- so it runs from an editor and fails from a
/// terminal on the same machine. Run it under a profile that allows it, or
/// with the sysctl at 0.
#[test]
#[ignore = "wants AppArmor to allow an unprivileged user namespace: see this test's doc"]
fn the_local_way_in_comes_up_and_lets_go_again() {
    if !in_our_own_namespace("the_local_way_in_comes_up_and_lets_go_again") {
        return;
    }
    the_namespace();
    let out = u64::from(roads::index_of(WAY_OUT).expect("the kernel numbers the way out"));

    let dir = std::env::temp_dir().join(format!("tracewing-local-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let local = Local {
        on: true,
        // A program every distribution has, so the list is not empty -- an
        // empty one keeps nothing, which would test less.
        programs: vec![Picked::at("/usr/bin/curl".into())],
    };
    // The real path, not a copy of the script: this reaches `make_adapter`,
    // which looks before it asks, and `run_as_admin`, which takes its
    // already-root road here and would have raised a dialog outside.
    let started = local_source::start(&local, vec![9090], &dir, &local_source::Http3::off())
        .expect("the way in assembles")
        .expect("the switch is on, so there is a source");

    // The one privileged step, read back the way the arm reads it.
    let look = adapter::look();
    println!("the adapter: {look:?}");
    assert!(look.ready(), "still missing {:?}", look.missing());
    let ours = u64::from(roads::index_of(adapter::DEVICE).expect("the kernel numbers it"));

    let now = roads::roads_now().expect("this namespace's routes");
    let mine = ours_in(&now, ours);
    for road in &now {
        println!(
            "  if {} bits {} metric {} v6 {} connected {}",
            road.interface, road.bits, road.metric, road.v6, road.connected
        );
    }
    assert_eq!(
        mine.iter().filter(|road| road.bits == 0).count(),
        2,
        "a default route in each family, which is what makes the machine route into it"
    );
    assert!(
        mine.iter().all(|road| road.connected),
        "held, so the carrier is up: that is the switch"
    );
    // And it is the way out that is chosen, not our own adapter, which is
    // the loop this mode has to avoid. Named rather than merely denied: on a
    // machine with no way out at all, `assert_ne!` here passes saying
    // nothing.
    for v6 in [false, true] {
        let chosen = roads::out_by(&now, ours, v6);
        println!(
            "way out for {}: {chosen:?}",
            if v6 { "IPv6" } else { "IPv4" }
        );
        assert_eq!(
            chosen.map(u64::from),
            Some(out),
            "the way out is not the one made for it"
        );
    }

    // Ending it the way the window does, and then letting the last handle go,
    // which is what takes the carrier down.
    started.stopper.stop();
    drop(started);

    // The kernel is told by the descriptor closing, so this is the one thing
    // here worth waiting for rather than reading once.
    let mut after = Vec::new();
    for _ in 0..50 {
        after = ours_in(&roads::roads_now().expect("this namespace's routes"), ours);
        if after.iter().all(|road| !road.connected) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(
        !after.is_empty(),
        "the rows are gone: on Linux they stay, and the kernel ignores them instead"
    );
    assert!(
        after.iter().all(|road| !road.connected),
        "the carrier is still up with nobody holding the device"
    );
    for v6 in [false, true] {
        let chosen = roads::out_by(
            &roads::roads_now().expect("this namespace's routes"),
            // Not told which is ours this time: anything may be the way out
            // except a dead interface, and ours is now dead. Passing the
            // real `ours` would use the value the code really uses and prove
            // nothing -- it is `connected` alone that has to carry this.
            0,
            v6,
        );
        println!(
            "after letting go, way out for {}: {chosen:?}",
            if v6 { "IPv6" } else { "IPv4" }
        );
        assert_eq!(
            chosen.map(u64::from),
            Some(out),
            "a dead adapter of ours is still being chosen, although nothing excluded it"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
    reached_the_end();
}
