//! The local way in, assembled on a real machine.
//!
//! Everything else about this way in is tested where it can be: the sieve's
//! rules, the switch's states, the tab's pictures. What no Linux test reaches
//! is the assembly itself on Windows -- `Apps::spawn`, the adapter coming up
//! with its address and its two `/1` halves, the sieve and the bind and the
//! echo going into one `Tun`, and the whole of it coming down again.
//!
//! `#[ignore]`d because it needs an administrator and `wintun.dll`, and
//! because it puts an adapter on the machine for as long as it runs:
//!
//! ```powershell
//! $env:WINTUN_DLL = "$HOME\wintun-dist\wintun\bin\amd64\wintun.dll"
//! cargo test -p tracewing --release --test local_live -- --ignored --nocapture
//! ```

#![cfg(windows)]

use tracewing::{
    local_apps::{Local, Picked},
    local_source,
};
use tracewing_proxy::roads;

/// How many ways out of ours the table holds at `bits`, in either family.
fn ours_at(luid_rows: &[roads::Road], bits: u8) -> usize {
    luid_rows.iter().filter(|road| road.bits == bits).count()
}

/// Rows belonging to an interface whose LUID is not any of `theirs`, which is
/// how the adapter's own are found without being told its LUID.
fn new_rows(before: &[roads::Road], now: &[roads::Road]) -> Vec<roads::Road> {
    let was: Vec<u64> = before.iter().map(|road| road.luid).collect();
    now.iter()
        .filter(|road| !was.contains(&road.luid))
        .copied()
        .collect()
}

/// The whole assembly: it comes up, it is on the machine, and it goes away.
#[test]
#[ignore = "needs Administrator and wintun.dll, and puts an adapter on this machine"]
fn the_local_way_in_comes_up_and_goes_away_again() {
    let dir = std::env::temp_dir().join(format!("tracewing-local-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let before = roads::roads_now().expect("this machine's routes");

    let local = Local {
        on: true,
        // A program that is on every Windows since 1803, so the list is not
        // empty -- an empty one keeps nothing, which would test less.
        programs: vec![Picked::at(r"C:\Windows\System32\curl.exe".into())],
    };
    let started = local_source::start(&local, vec![9090], &dir)
        .expect("the way in assembles")
        .expect("the switch is on, so there is a source");

    let now = roads::roads_now().expect("this machine's routes");
    let ours = new_rows(&before, &now);
    println!(
        "the adapter brought {} row(s), {} of them /1",
        ours.len(),
        ours_at(&ours, 1)
    );
    for road in &ours {
        println!(
            "  if {} bits {} metric {} v6 {} connected {}",
            road.interface, road.bits, road.metric, road.v6, road.connected
        );
    }
    assert_eq!(
        ours_at(&ours, 1),
        4,
        "both halves of each address space: that is what makes the machine route into it"
    );
    // And it is never the way out, which is the loop this mode has to avoid.
    let luid = ours.first().expect("rows of our own").luid;
    for v6 in [false, true] {
        let out = roads::out_by(&now, luid, v6);
        println!("way out for {}: {out:?}", if v6 { "IPv6" } else { "IPv4" });
        if let Some(out) = out {
            assert!(
                !ours.iter().any(|road| road.interface == out),
                "the way out is our own adapter, interface {out}"
            );
        }
    }

    // Ending it the way the window does.
    started.stopper.stop();
    drop(started);
    let after = roads::roads_now().expect("this machine's routes");
    assert!(
        new_rows(&before, &after).is_empty(),
        "the adapter's rows are still in the table after it was stopped and dropped"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
