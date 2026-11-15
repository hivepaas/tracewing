//! Run by hand on Windows, with Administrator and a real wintun.dll. Not part
//! of the suite and not committed: it is here to check the three assumptions
//! `wintun.rs` makes that no amount of compiling can confirm.
#![cfg(windows)]

use std::{path::PathBuf, sync::Arc, sync::mpsc, thread, time::Duration};
use tracewing_proxy::{tun::Device, wintun::WintunAdapter};

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
