//! The WireGuard tunnel the settings ask for: served when the switch is on,
//! left shut when it is off, and — when something else holds the port — not
//! served, not fatal, and said in words Settings can show. Plus the one
//! thing a phone cannot do without: the configuration it is given.
//!
//! The protocol itself is tested in `tracewing-proxy`, against `boringtun`
//! playing the phone. What is tested here is only the wiring from
//! `settings.toml` to a port that answers and a configuration that names it.
//!
//! `cargo test --release -p tracewing --features headless --test wireguard`
//!
//! This file claims `ports: 20_520..20_530` for the proxies it starts, the
//! next free ten above the top of the ranges `tests/socks.rs` lists.

#![cfg(all(feature = "headless", target_os = "linux"))]

use std::{
    net::{SocketAddr, UdpSocket},
    path::PathBuf,
    time::{Duration, Instant},
};
use tracewing::engine::{Engine, EngineOptions};
use tracewing_store::settings::StorageSettings;

fn data_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tracewing-wg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A port nothing is listening on: bound to find a free number, then let go.
/// UDP, because that is what the tunnel binds, and a free TCP number says
/// nothing about the UDP one.
fn a_free_port() -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap().port()
}

fn start(name: &str, settings: StorageSettings, ports: std::ops::Range<u16>) -> (Engine, PathBuf) {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = data_dir(name);
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings,
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports,
        extra_roots: Vec::new(),
        // Not the user's keychain: the keys of a test belong in its own
        // directory, which `KeyStore::File` is for.
        key_store: Default::default(),
        find_apps: false,
    })
    .unwrap();
    (engine, dir)
}

#[test]
fn the_switch_serves_a_tunnel_on_the_number_asked_for() {
    let port = a_free_port();
    let settings = StorageSettings {
        wireguard: true,
        wireguard_port: u64::from(port),
        ..StorageSettings::default()
    };
    let (mut engine, dir) = start("on", settings, 20_520..20_530);
    let at = engine.wireguard.expect("a tunnel was served");
    assert_eq!(
        at,
        SocketAddr::from(([0, 0, 0, 0], port)),
        "every address, because the phone is on another one"
    );
    assert_eq!(engine.wireguard_problem, None);

    // Something is really listening: a datagram to it is taken, not refused.
    // What it answers is the protocol's business and is tested there; here it
    // must only not be a closed port.
    let scanner = UdpSocket::bind("127.0.0.1:0").unwrap();
    scanner
        .send_to(b"not a handshake", SocketAddr::from(([127, 0, 0, 1], port)))
        .expect("the port takes datagrams");

    engine.stop();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn switched_off_there_is_no_tunnel_and_no_keys() {
    let (mut engine, dir) = start("off", StorageSettings::default(), 20_520..20_530);
    assert_eq!(engine.wireguard, None);
    assert_eq!(engine.wireguard_problem, None, "none was asked for");
    assert_eq!(
        engine.wireguard_config(),
        None,
        "nothing to tell a phone, and no keys were made for one"
    );
    assert!(
        !dir.join("wireguard.key").exists(),
        "a switch left off writes no key material"
    );
    engine.stop();
    let _ = std::fs::remove_dir_all(dir);
}

/// A port something else holds must not stop tracewing: everything that can
/// use the proxy port still can.
#[test]
fn a_port_already_in_use_is_said_and_not_fatal() {
    let held = UdpSocket::bind("0.0.0.0:0").unwrap();
    let port = held.local_addr().unwrap().port();
    let settings = StorageSettings {
        wireguard: true,
        wireguard_port: u64::from(port),
        ..StorageSettings::default()
    };
    let (mut engine, dir) = start("taken", settings, 20_520..20_530);
    assert_eq!(engine.wireguard, None, "it could not be served");
    let why = engine.wireguard_problem.clone().expect("it says why");
    assert!(why.contains(&port.to_string()), "which port: {why}");
    assert_eq!(
        engine.wireguard_config(),
        None,
        "and nothing is offered to a phone that could not connect"
    );
    // The proxy itself is up, which is the point of it not being fatal.
    assert!(
        std::net::TcpStream::connect(engine.addr).is_ok(),
        "the proxy port"
    );
    engine.stop();
    drop(held);
    let _ = std::fs::remove_dir_all(dir);
}

/// The configuration a phone is given, and the reason it is written when it
/// is asked for rather than when the proxy starts: the addresses other
/// devices reach this machine at are bound by a task inside the proxy's
/// thread, so at the moment `Engine::start` returns there are none. A
/// configuration built then would have no address in it, every time.
#[test]
fn the_phones_configuration_names_an_address_it_can_reach() {
    let port = a_free_port();
    let settings = StorageSettings {
        wireguard: true,
        wireguard_port: u64::from(port),
        // The tunnel's address has to be one a phone could reach, and this
        // is what finds those.
        other_devices: true,
        ..StorageSettings::default()
    };
    let (mut engine, dir) = start("config", settings, 20_520..20_530);

    // Waited for rather than assumed: see above. Without the wait this test
    // passes or fails by how fast the machine is.
    let deadline = Instant::now() + Duration::from_secs(20);
    let config = loop {
        if let Some(config) = engine.wireguard_config() {
            break config;
        }
        assert!(
            Instant::now() < deadline,
            "waiting for an address other devices could reach"
        );
        std::thread::sleep(Duration::from_millis(20));
    };

    assert!(
        config.contains(&format!(":{port}")),
        "the endpoint names the tunnel's port: {config}"
    );
    assert!(
        config.contains("[Interface]") && config.contains("[Peer]"),
        "a WireGuard configuration, as its app imports one: {config}"
    );
    assert!(
        config.contains("AllowedIPs = 0.0.0.0/0, ::/0"),
        "everything goes through it, which is the point: {config}"
    );
    assert!(
        !config.contains("0.0.0.0:"),
        "a phone cannot reach `0.0.0.0`, whatever the socket is bound to: {config}"
    );

    // Asked twice, answered the same: the keys are kept, so a phone set up
    // yesterday still matches the configuration shown today.
    assert_eq!(
        engine.wireguard_config().as_deref(),
        Some(config.as_str()),
        "the same keys and the same address"
    );

    engine.stop();
    let _ = std::fs::remove_dir_all(dir);
}
