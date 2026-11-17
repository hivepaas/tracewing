//! Run by hand on Windows, with Administrator and a real wintun.dll. Not part
//! of the suite and not committed: it is here to check the three assumptions
//! `wintun.rs` makes that no amount of compiling can confirm.
#![cfg(windows)]

use std::{path::PathBuf, sync::Arc, sync::mpsc, thread, time::Duration};
use tracewing_proxy::{
    roads,
    tun::Device,
    wintun::{Held, WintunAdapter},
};

/// **Run these one at a time.** They share one adapter, by a fixed GUID, so
/// `cargo test`'s default threads have several of them calling
/// `Adapter::create` at once and all but one fails -- with
/// *"Failed to create adapter. Wintun needs Administrator"*, which is the
/// message for every creation failure and sends the reader after elevation
/// they already have. Measured: three of six failed that way in an elevated
/// session at High integrity, and all six passed in the same session with
/// `--test-threads=1`.
///
/// ```powershell
/// $env:WINTUN_DLL = "$HOME\wintun-dist\wintun\bin\amd64\wintun.dll"
/// cargo test -p tracewing-proxy --release --test wintun_live -- --ignored --test-threads=1
/// ```
fn dll() -> PathBuf {
    PathBuf::from(std::env::var("WINTUN_DLL").expect("WINTUN_DLL: the path to wintun.dll"))
}

/// 1. The adapter opens at all: the DLL loads from a path, `Adapter::create`
///    takes our fixed GUID, and `start_session` accepts a 4 MiB ring.
#[test]
#[ignore = "needs Administrator and wintun.dll"]
fn the_adapter_opens() {
    let adapter = WintunAdapter::open(&dll()).expect("open the adapter");
    assert_eq!(adapter.name(), "wintun");
}

/// 2. A packet can be written: `allocate_send_packet`, `bytes_mut` and
///    `send_packet` are used correctly. Whether the packet is one Windows
///    likes is not the point — with no address or route on the adapter it
///    will be dropped, and the call still has to succeed.
#[test]
#[ignore = "needs Administrator and wintun.dll"]
fn a_packet_can_be_written() {
    let adapter = WintunAdapter::open(&dll()).expect("open the adapter");
    let mut packet = vec![
        0x45, 0x00, 0x00, 0x1c, // version, IHL, DSCP, total length 28
        0x00, 0x00, 0x00, 0x00, // id, flags
        0x40, 0x11, 0x00, 0x00, // TTL 64, UDP, checksum left at zero
        10, 0, 0, 2, // from
        10, 0, 0, 1, // to
    ];
    packet.extend_from_slice(&[0x30, 0x39, 0x00, 0x35, 0x00, 0x08, 0x00, 0x00]);
    adapter.write(&packet).expect("the adapter took the packet");
}

/// 3. **The one the stop rests on.** `tun.rs` ends its reading thread by
///    asking the device to shut down, because nothing else can interrupt a
///    blocked read. If the real driver does not make `read` fail after a
///    shutdown, tracewing will not quit -- and no test that avoids the
///    driver can say so.
///
///    The first version of this test asserted that nothing arrives on a
///    fresh adapter, and it **failed**: Windows begins sending out of a new
///    adapter at once, before it has an address or a route. So this reads in
///    a loop, exactly as the pump does, and asserts the only thing the pump
///    needs -- that the loop *ends*, with an error, once the shutdown is
///    asked for.
#[test]
#[ignore = "needs Administrator and wintun.dll"]
fn a_shutdown_ends_the_reading_loop() {
    let adapter = Arc::new(WintunAdapter::open(&dll()).expect("open the adapter"));
    let reading = Arc::clone(&adapter);
    let (ended, done) = mpsc::channel();
    thread::spawn(move || {
        let mut packets: Vec<Vec<u8>> = Vec::new();
        loop {
            match reading.read() {
                Ok(packet) => packets.push(packet),
                Err(e) => {
                    let _ = ended.send((packets, e.to_string()));
                    return;
                }
            }
        }
    });
    // It must not end on its own: without a shutdown there is no error to
    // end it, however much the adapter has to say.
    thread::sleep(Duration::from_millis(500));
    assert!(
        done.try_recv().is_err(),
        "the reading loop ended before any shutdown was asked for"
    );
    adapter.shutdown();
    let (packets, said) = done
        .recv_timeout(Duration::from_secs(5))
        .expect("the reading loop ended after the shutdown");
    println!("the loop ended saying: {said}");
    println!("{} packet(s) arrived before the shutdown", packets.len());
    for packet in packets.iter().take(5) {
        println!("  {} bytes, IP version {}", packet.len(), packet[0] >> 4);
    }
}

/// 4. **The adapter has a LUID, and is never the way out.**
///
///    What this checks today is the first half: that `luid()` works against
///    a real adapter — the driver's index call and the conversion — and that
///    it is not zero, since a zero would exclude nothing and the exclusion
///    is the whole point.
///
///    The second half is **nearly free today and will not stay that way**,
///    and the reason is not the one a first draft of this comment gave. It
///    said the adapter is not in the routing table. It is: a fresh Wintun
///    adapter with no address of ours already has five routes, which a probe
///    on the VM printed as IPv4 `/4` and a loopback `/32`, and IPv6 `/64`,
///    `/8` and a loopback `/128`. What makes the exclusion idle today is
///    that none of those reaches further than `WIDE_ENOUGH`, so they are
///    filtered out as ways out before the question of whose they are arises.
///
///    Which is exactly why this will bite later rather than never. The
///    moment the adapter is given the two halves of a default route, it
///    gains rows at `/1` — candidates, and ones that **reach further than
///    another interface's `/0`**, so they would win. On that day the only
///    thing standing between tracewing and a loop is that these rows are
///    recognised as ours, and this test is what says they are.
#[test]
#[ignore = "needs Administrator and wintun.dll"]
fn the_adapter_has_a_luid_and_is_never_the_way_out() {
    let adapter = WintunAdapter::open(&dll()).expect("open the adapter");
    let ours = adapter.luid().expect("the adapter's LUID");
    assert_ne!(ours, 0, "a LUID of zero would leave our own adapter in");

    let roads = roads::roads_now().expect("this machine's routes");
    let mine: Vec<u32> = roads
        .iter()
        .filter(|road| road.luid == ours)
        .map(|road| road.interface)
        .collect();
    for v6 in [false, true] {
        let family = match v6 {
            true => "IPv6",
            false => "IPv4",
        };
        if let Some(out) = roads::out_by(&roads, ours, v6) {
            assert!(
                !mine.contains(&out),
                "the way out for {family} is our own adapter, interface {out}"
            );
        }
    }
    println!(
        "ours = {ours:#018x}, with {} row(s) in the table",
        mine.len()
    );
    println!(
        "out by: IPv4 {:?}, IPv6 {:?}",
        roads::out_by(&roads, ours, false),
        roads::out_by(&roads, ours, true)
    );
}

/// The adapter **up**, which is what the capture mode holds: an address of
/// each family and the two halves of each address space, and all of it gone
/// when it is dropped.
///
/// This is the case the comment on
/// `the_adapter_has_a_luid_and_is_never_the_way_out` says will bite later: at
/// `/1` our own rows reach **further** than another interface's `/0`, so they
/// would win `out_by` on reach, and the only thing standing between tracewing
/// and routing its own connections back into itself is that they are
/// recognised as ours. Here they exist, so the exclusion is tested against
/// the arrangement that needs it rather than against one where it is idle.
#[test]
#[ignore = "needs Administrator and wintun.dll"]
fn the_adapter_comes_up_with_both_halves_and_goes_down_again() {
    let luid = {
        let held = Held::up(&dll()).expect("bring the adapter up");
        let luid = held.luid().expect("the adapter's LUID");
        assert_ne!(luid, 0);
        assert_eq!(
            rows_become(luid, 1, false, 2),
            2,
            "both halves of the IPv4 address space"
        );
        assert_eq!(rows_become(luid, 1, true, 2), 2, "and both of IPv6's");

        let roads = roads::roads_now().expect("this machine's routes");
        let mine: Vec<u32> = roads
            .iter()
            .filter(|road| road.luid == luid)
            .map(|road| road.interface)
            .collect();
        for v6 in [false, true] {
            let family = match v6 {
                true => "IPv6",
                false => "IPv4",
            };
            let out = roads::out_by(&roads, luid, v6);
            println!("with the halves up, {family} goes out by {out:?}; ours are {mine:?}");
            if let Some(out) = out {
                assert!(
                    !mine.contains(&out),
                    "the way out for {family} is our own adapter, interface {out} -- \
                     at /1 our rows reach further than any /0, so this is the \
                     arrangement where the exclusion is the only thing holding"
                );
            }
        }
        luid
    };
    // The handle gone takes the adapter, and the adapter takes its rows.
    assert_eq!(
        rows_become(luid, 1, false, 0),
        0,
        "the halves go when it is dropped"
    );
    assert_eq!(rows_become(luid, 1, true, 0), 0);
}

/// How many routes the table holds for `luid` at exactly `bits`, in the
/// family asked for.
fn rows(luid: u64, bits: u8, v6: bool) -> usize {
    roads::roads_now()
        .expect("this machine's routes")
        .iter()
        .filter(|road| road.luid == luid && road.bits == bits && road.v6 == v6)
        .count()
}

/// Waits for that count to become `want`. Windows does not promise to have
/// finished with an address by the time the call returns.
fn rows_become(luid: u64, bits: u8, v6: bool, want: usize) -> usize {
    let until = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let now = rows(luid, bits, v6);
        if now == want || std::time::Instant::now() > until {
            return now;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

/// 5. **An address, and Windows acting on it.**
///
///    The address going on is only half of it, and the weaker half: a call
///    that returns `NO_ERROR` says Windows accepted the row, not that
///    anything came of it. So this asserts the consequence instead —
///    Windows makes an **on-link route** for a prefix an interface has been
///    given, and a route is what routing acts on.
///
///    It also asserts the route goes away again, which is what makes
///    [`Address`]'s `Drop` worth having rather than assumed: an address left
///    behind on a disposable test VM is invisible, and on a real machine it
///    is a route somebody's traffic can fall into.
#[test]
#[ignore = "needs Administrator and wintun.dll"]
fn an_address_brings_an_on_link_route_and_takes_it_away_again() {
    let adapter = Arc::new(WintunAdapter::open(&dll()).expect("open the adapter"));
    let ours = adapter.luid().expect("the adapter's LUID");
    // A private prefix of its own, so that a machine running this does not
    // lose a route it was using.
    let addr: std::net::IpAddr = "10.6.6.1".parse().expect("an address");
    const BITS: u8 = 24;

    assert_eq!(
        rows(ours, BITS, false),
        0,
        "nothing of ours at /{BITS} before the address"
    );
    {
        let _address = adapter
            .address(addr, BITS)
            .expect("give the adapter its address");
        assert_eq!(
            rows_become(ours, BITS, false, 1),
            1,
            "an on-link route for the prefix appeared"
        );
        println!("with the address, rows of ours at /{BITS}: 1");
    }
    assert_eq!(
        rows_become(ours, BITS, false, 0),
        0,
        "and went again when the address was dropped"
    );
    println!("after dropping it: 0");
}

/// 6. **A route in, and the machine still reachable afterwards.**
///
///    The prefix is `203.0.113.0/26` and both halves of that are chosen, not
///    convenient. `203.0.113.0/24` is the documentation range: reserved,
///    never routed, so a route into it cannot take traffic away from
///    anything real. And `/26` because [`roads::Road`] carries a prefix
///    *length* and not a prefix, so a `/24` here would be
///    indistinguishable from the on-link route the address test's
///    `10.6.6.1/24` brings with it. Nothing else uses `/26`.
///
///    **What this deliberately does not do is route the two default
///    halves.** `0.0.0.0/1` and `128.0.0.0/1` are what the real thing will
///    add, and adding them here — to an adapter whose packets nothing is
///    reading, in a test running over SSH — would black-hole the connection
///    the test is being watched through. The FFI and its failure modes are
///    the same either way; only the blast radius differs.
#[test]
#[ignore = "needs Administrator and wintun.dll"]
fn a_route_goes_into_the_adapter_and_comes_out_again() {
    let adapter = Arc::new(WintunAdapter::open(&dll()).expect("open the adapter"));
    let ours = adapter.luid().expect("the adapter's LUID");
    let to: std::net::IpAddr = "203.0.113.0".parse().expect("an address");
    const BITS: u8 = 26;

    assert_eq!(
        rows(ours, BITS, false),
        0,
        "nothing of ours at /{BITS} before the route"
    );
    {
        let _route = adapter
            .route(to, BITS)
            .expect("route the prefix into the adapter");
        assert_eq!(
            rows_become(ours, BITS, false, 1),
            1,
            "the route is in the table, against our adapter"
        );
        println!("with the route, rows of ours at /{BITS}: 1");
    }
    assert_eq!(
        rows_become(ours, BITS, false, 0),
        0,
        "and gone again when the route was dropped"
    );
    println!("after dropping it: 0");
}
