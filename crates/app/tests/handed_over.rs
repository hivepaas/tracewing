//! A way in built after the engine started is served by it.
//!
//! The local capture mode is handed over this way rather than gathered at
//! startup, because making its adapter asks for a password and a password
//! asked for before there is a window is one nobody can place. What that
//! leaves untested is the wiring itself: the window has a sender, the proxy
//! has the receiver, and between them is a channel that would be just as
//! quiet if it were connected to nothing.
//!
//! So this hands over a source that does nothing but record that it was
//! started. It does not send a flow: what a flow does once it arrives is the
//! proxy's own business and is covered where that is tested. What is covered
//! here is that the handing over reaches a running proxy at all.
//!
//! `cargo test --release -p tracewing --test handed_over`
//!
//! This file claims `ports: 20_560..20_570` for the proxy it starts, the next
//! free ten above `tests/local_tab.rs`.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tracewing::engine::{Engine, EngineOptions};
use tracewing_proxy::source::{By, Found, Source, Stop};
use tracewing_store::settings::StorageSettings;

/// A way in that records that the proxy started it.
struct Counts(Arc<AtomicUsize>);

impl Source for Counts {
    fn name(&self) -> String {
        "a counted way in".to_owned()
    }

    fn by(&self) -> By {
        By::Given
    }

    fn start(
        self: Box<Self>,
        _proxy: std::net::SocketAddr,
        _found: Found,
        _stop: Stop,
    ) -> anyhow::Result<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn a_way_in_handed_over_after_the_start_is_started() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-handed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_560..20_570,
        extra_roots: Vec::new(),
        key_store: Default::default(),
        find_apps: false,
    })
    .expect("the engine starts");

    let started = Arc::new(AtomicUsize::new(0));
    engine
        .serving
        .blocking_send(Box::new(Counts(Arc::clone(&started))) as Box<dyn Source>)
        .expect("the proxy is listening for a way in that arrives later");

    // The proxy starts it on its own runtime, so this waits for the answer
    // rather than assuming it has already happened. A second is far longer
    // than it takes and short enough that a failure is a failure.
    let until = std::time::Instant::now() + std::time::Duration::from_secs(1);
    while started.load(Ordering::SeqCst) == 0 && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        started.load(Ordering::SeqCst),
        1,
        "the way in reached the running proxy and was started exactly once"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
