//! The SOCKS5 port the settings ask for: opened when the switch is on, left
//! shut when it is off, and — when something else already holds the port —
//! not opened, not fatal, and said in words Settings can show.
//!
//! The protocol itself is tested in `tracewing-proxy`; what is tested here is
//! only the wiring from `settings.toml` to a port that answers.
//!
//! `cargo test --release -p tracewing --features headless --test socks`

#![cfg(all(feature = "headless", target_os = "linux"))]

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    time::Duration,
};
use tracewing::engine::{Engine, EngineOptions};
use tracewing_store::settings::StorageSettings;

fn data_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tracewing-socks-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A port nothing is listening on: bound to find a free number, then let go.
fn a_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// Each test here starts a proxy of its own, so each needs a port range of
/// its own: the app takes the first free port in the range, and two test
/// binaries given the same range will mostly miss each other and sometimes
/// not. Every other file under `tests/` claims one range of ten, and the
/// ranges in use end at `sidebar.rs`'s 20_480..20_490, so these three carry
/// on from there. A new test file takes the next free ten rather than any
/// round number.
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
        key_store: Default::default(),
        find_apps: false,
    })
    .unwrap();
    (engine, dir)
}

/// The handshake, far enough to prove a SOCKS5 server is behind the port and
/// not something else listening on it.
fn greets_as_socks5(at: SocketAddr) {
    let mut client = TcpStream::connect(at).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    // Version 5, one way in offered, and that way is no authentication.
    client.write_all(&[5, 1, 0]).unwrap();
    let mut answer = [0u8; 2];
    client.read_exact(&mut answer).unwrap();
    assert_eq!(answer, [5, 0], "version 5, no authentication");
}

#[test]
fn the_switch_opens_a_socks5_port_on_the_number_asked_for() {
    let port = a_free_port();
    let settings = StorageSettings {
        socks: true,
        socks_port: u64::from(port),
        ..StorageSettings::default()
    };
    let (mut engine, dir) = start("on", settings, 20_490..20_500);
    let at = engine.socks.expect("a port was opened");
    assert_eq!(at, SocketAddr::from(([127, 0, 0, 1], port)));
    assert_eq!(engine.socks_problem, None);
    greets_as_socks5(at);
    engine.stop();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn switched_off_there_is_no_socks5_port() {
    let (mut engine, dir) = start("off", StorageSettings::default(), 20_500..20_510);
    assert_eq!(engine.socks, None);
    assert_eq!(engine.socks_problem, None, "none was asked for");
    engine.stop();
    let _ = std::fs::remove_dir_all(dir);
}

/// A port something else holds is the one failure that is likely, and it must
/// not stop tracewing: everything that can use the proxy port still can.
#[test]
fn a_port_already_in_use_is_said_and_not_fatal() {
    let held = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = held.local_addr().unwrap().port();
    let settings = StorageSettings {
        socks: true,
        socks_port: u64::from(port),
        ..StorageSettings::default()
    };
    let (mut engine, dir) = start("taken", settings, 20_510..20_520);
    assert_eq!(engine.socks, None, "it could not be opened");
    let why = engine.socks_problem.clone().expect("it says why");
    assert!(why.contains(&port.to_string()), "which port: {why}");
    // The proxy itself is up, which is the point of it not being fatal.
    assert!(TcpStream::connect(engine.addr).is_ok(), "the proxy port");
    engine.stop();
    drop(held);
    let _ = std::fs::remove_dir_all(dir);
}
