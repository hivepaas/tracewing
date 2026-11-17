//! The local way in, assembled on a real Linux machine.
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
//! `#[ignore]`d because it asks polkit for rights, and because for as long as
//! it holds the device **this machine's traffic goes into a TUN nobody is
//! pumping**: the source is never started here, so the packets are dropped.
//! Seconds, and only while it runs, but it is not a test to leave in a loop:
//!
//! ```sh
//! cargo test -p tracewing --release --test local_live_linux -- --ignored --nocapture
//! ```

#![cfg(target_os = "linux")]

use tracewing::{
    local_apps::{Local, Picked},
    local_source,
};
use tracewing_proxy::roads;
use tracewing_system::adapter;

/// Our own rows, by the index the kernel gives the adapter's name.
fn ours_in(roads: &[roads::Road], ours: u64) -> Vec<roads::Road> {
    roads
        .iter()
        .filter(|road| road.luid == ours)
        .copied()
        .collect()
}

/// The whole assembly: it comes up, it is on the machine, and letting go
/// leaves it harmless.
#[test]
#[ignore = "asks polkit for rights, and blackholes this machine's traffic while it holds the device"]
fn the_local_way_in_comes_up_and_lets_go_again() {
    let dir = std::env::temp_dir().join(format!("tracewing-local-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let local = Local {
        on: true,
        // A program every distribution has, so the list is not empty -- an
        // empty one keeps nothing, which would test less.
        programs: vec![Picked::at("/usr/bin/curl".into())],
    };
    let started = local_source::start(&local, vec![9090], &dir)
        .expect("the way in assembles")
        .expect("the switch is on, so there is a source");

    // The one privileged step, read back the way the arm reads it.
    let look = adapter::look();
    println!("the adapter: {look:?}");
    assert!(look.ready(), "still missing {:?}", look.missing());
    let ours = u64::from(roads::index_of(adapter::DEVICE).expect("the kernel numbers it"));

    let now = roads::roads_now().expect("this machine's routes");
    let mine = ours_in(&now, ours);
    for road in &mine {
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
    // And it is never the way out, which is the loop this mode has to avoid.
    for v6 in [false, true] {
        let out = roads::out_by(&now, ours, v6);
        println!("way out for {}: {out:?}", if v6 { "IPv6" } else { "IPv4" });
        assert_ne!(
            out.map(u64::from),
            Some(ours),
            "the way out is our own adapter"
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
        after = ours_in(&roads::roads_now().expect("this machine's routes"), ours);
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
        let out = roads::out_by(
            &roads::roads_now().expect("this machine's routes"),
            // Not told which is ours this time: anything may be the way out
            // except a dead interface, and ours is now dead.
            0,
            v6,
        );
        println!(
            "after letting go, way out for {}: {out:?}",
            if v6 { "IPv6" } else { "IPv4" }
        );
        assert_ne!(
            out.map(u64::from),
            Some(ours),
            "a dead interface of ours is still being chosen as the way out"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
