//! S7, the block on UDP 443: while it is held, a QUIC client's datagrams are
//! refused; let go, or its holder killed outright, they get through again.
//! An echo server on 127.0.0.1:443 stands for the internet's HTTP/3 servers:
//! the rule refuses UDP to port 443 on every interface, loopback included.
//!
//! Changes this computer's firewall. See `tracewing_e2e`.

use std::{net::UdpSocket, time::Duration};
use tracewing_e2e::{
    helper::Helper, only_where_allowed, only_where_someone_answers, scratch, wait_for,
};

/// Answers whatever comes to 127.0.0.1:443, until the test ends.
fn echo_on_443() {
    // macOS lets an ordinary user, which `--attended` runs this as, have a
    // port under 1024 only on the wildcard address: 127.0.0.1:443 is refused
    // (EACCES), 0.0.0.0:443 is not. A container lets one have either, by
    // ip_unprivileged_port_start.
    let at = if cfg!(target_os = "macos") {
        "0.0.0.0:443"
    } else {
        "127.0.0.1:443"
    };
    let server = UdpSocket::bind(at).unwrap_or_else(|e| panic!("bind UDP {at}: {e}"));
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
    // macOS's pf takes an administrator, which it asks for.
    only_where_someone_answers!();
    let dir = scratch("quic-block");
    echo_on_443();
    assert!(
        answered(),
        "the echo does not answer even with nothing blocked"
    );

    // tracewing holds it, as the window does, and lets it go.
    let block = Helper::quic_block(&dir);
    assert!(!answered(), "a datagram to 443 got through the block");
    let ended = block.let_go();
    assert!(ended.success(), "letting go failed: {ended:?}");
    assert!(answered(), "still refused after the block was let go");

    // Killed outright instead: what holds the rule goes with it.
    let block = Helper::quic_block(&dir);
    assert!(
        !answered(),
        "a datagram to 443 got through the second block"
    );
    block.kill();
    wait_for(
        "the block to go with its holder",
        Duration::from_secs(10),
        answered,
    );
    std::fs::remove_dir_all(&dir).ok();
}
