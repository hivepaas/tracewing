//! S7, the block on UDP 443: while it is held, a QUIC client's datagrams are
//! refused; let go, or its holder killed outright, they get through again.
//! An echo server on 127.0.0.1:443 stands for the internet's HTTP/3 servers:
//! the rule refuses UDP to port 443 on every interface, loopback included.
//!
//! Changes this computer's firewall. See `tracewing_e2e`.

use std::{net::UdpSocket, path::PathBuf, time::Duration};
use tracewing_e2e::{
    helper::{Helper, asked, hold_until_killed},
    only_where_allowed, scratch, wait_for,
};

const HOLDER: &str = "a_program_that_holds_the_block";

/// Not a test: what holds the block, run by the test below as a process of
/// its own. Asked its data folder.
#[test]
#[ignore = "run by the other tests here, as the holder they kill"]
fn a_program_that_holds_the_block() {
    let Some(dir) = asked() else { return };
    let block = tracewing_system::block_quic(&PathBuf::from(dir)).unwrap();
    std::mem::forget(block);
    hold_until_killed();
}

/// Answers whatever comes to 127.0.0.1:443, until the test ends.
fn echo_on_443() {
    let server = UdpSocket::bind("127.0.0.1:443")
        .expect("bind UDP 443: a container lets an ordinary user, by ip_unprivileged_port_start");
    std::thread::spawn(move || {
        let mut buf = [0u8; 64];
        while let Ok((n, from)) = server.recv_from(&mut buf) {
            let _ = server.send_to(&buf[..n], from);
        }
    });
}

/// Whether a datagram to 127.0.0.1:443 is answered.
fn answered() -> bool {
    let client = UdpSocket::bind("127.0.0.1:0").unwrap();
    client.connect("127.0.0.1:443").unwrap();
    client
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let mut buf = [0u8; 64];
    client.send(b"quic?").is_ok() && client.recv(&mut buf).is_ok_and(|n| &buf[..n] == b"quic?")
}

#[test]
fn udp_443_is_refused_while_blocked_and_free_again_after() {
    only_where_allowed!();
    let dir = scratch("quic-block");
    echo_on_443();
    assert!(
        answered(),
        "the echo does not answer even with nothing blocked"
    );

    let block = tracewing_system::block_quic(&dir).unwrap();
    assert!(!answered(), "a datagram to 443 got through the block");
    block.release().unwrap();
    assert!(answered(), "still refused after the block was let go");

    // Its holder killed outright: the kernel takes the table away with it.
    let holder = Helper::start(HOLDER, &dir.display().to_string());
    assert!(
        !answered(),
        "a datagram to 443 got through the helper's block"
    );
    holder.kill();
    wait_for(
        "the block to go with its holder",
        Duration::from_secs(5),
        answered,
    );
    std::fs::remove_dir_all(&dir).ok();
}
