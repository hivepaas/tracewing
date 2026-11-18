//! A refused QUIC attempt names the program that sent it.
//!
//! This is the one thing no unit test stands in for. The socket a datagram
//! left is looked for in the operating system's own table of UDP sockets --
//! a different table from the one a connection is found in, and on Windows
//! one that holds no far end at all -- so what shows it works is a real
//! socket, held by this program, that really sent a datagram, and the row
//! the proxy keeps about it.
//!
//! It goes through the handed-over seam rather than a TUN adapter, because
//! what is being checked is the look-up and the row, and making an adapter
//! asks for a password. What a refused attempt is read off the wire by is
//! covered in `tracewing_proxy::stack`.
//!
//! `cargo test --release -p tracewing --test quic_sender`
//!
//! This file claims `ports: 20_570..20_580` for the proxy it starts, the next
//! free ten above `tests/handed_over.rs`.

use std::{
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};
use tracewing::engine::{Engine, EngineOptions};
use tracewing_proxy::{
    quic::{Attempt, Hello},
    source::{By, Found, Source, Stop},
};
use tracewing_store::{session, settings::StorageSettings};

/// A way in that tells the proxy about one QUIC attempt and nothing else.
struct ToldOnce(Attempt);

impl Source for ToldOnce {
    fn name(&self) -> String {
        "a told attempt".to_owned()
    }

    fn by(&self) -> By {
        By::Given
    }

    fn start(self: Box<Self>, _proxy: SocketAddr, found: Found, _stop: Stop) -> anyhow::Result<()> {
        found.quic(self.0);
        Ok(())
    }
}

/// What the row says, once there is one: its kind, host, and the program.
fn row(session: &std::path::Path) -> Option<(String, String, Option<String>, Option<u32>)> {
    let db = rusqlite::Connection::open(session::flows_db(session)).ok()?;
    db.query_row(
        "SELECT kind, host, app, app_pid FROM flows ORDER BY id DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .ok()
}

#[test]
fn a_refused_quic_attempt_names_the_program_that_sent_it() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-quic-sender-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // A socket of this program's that really sends a datagram, and is held
    // open while the proxy asks the system who holds it -- which is what it
    // would be on a machine where a browser is the one asking for HTTP/3.
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let client = socket.local_addr().unwrap();
    // Nothing listens there; the datagram leaving is the point.
    let _ = socket.send_to(b"not really a QUIC Initial", "127.0.0.1:9");

    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_570..20_580,
        extra_roots: Vec::new(),
        key_store: Default::default(),
        // The whole point of this test: without it every flow is as if it
        // came from another device, which is how the rest of the tests get one.
        find_apps: true,
    })
    .expect("the engine starts");

    engine
        .serving
        .blocking_send(Box::new(ToldOnce(Attempt {
            client,
            to: "93.184.216.34:443".parse().unwrap(),
            version: 1,
            hello: Some(Hello {
                server_name: Some("example.com".to_owned()),
                alpn: vec!["h3".to_owned()],
            }),
        })) as Box<dyn Source>)
        .expect("the proxy is listening for a way in that arrives later");

    // The row is written on another thread, after the look-up answers.
    let until = Instant::now() + Duration::from_secs(20);
    let found = loop {
        if let Some(found) = row(&engine.session)
            && found.3.is_some()
        {
            break found;
        }
        assert!(
            Instant::now() < until,
            "waiting for the row and its program"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    let (kind, host, app, pid) = found;
    assert_eq!(kind, "quic");
    assert_eq!(host, "example.com");
    assert_eq!(
        pid,
        Some(std::process::id()),
        "the program that holds the socket the datagram left"
    );
    let app = app.expect("a name as well as a number");
    let exe = std::env::current_exe().unwrap();
    let file = exe.file_name().unwrap().to_string_lossy();
    assert_eq!(
        app,
        file.strip_suffix(".exe").unwrap_or(&file),
        "what this test program is called, which is what a row would say"
    );
    drop(socket);
    let _ = std::fs::remove_dir_all(&dir);
}
