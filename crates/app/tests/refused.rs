//! A program that turns tracewing's certificate down: its connection is a
//! row that says so, rather than nothing; the program is named, and Set up
//! opens at the answer that deals with it; and leaving the host alone from
//! there lets the program through.
//!
//! This file claims `ports: 20_540..20_550` for the proxy it starts.
//!
//! `cargo test --release -p tracewing --features headless --test refused`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    flows::handshake_note,
    layout::Layout,
    system::{BrowserFix, CaState},
    traffic::{Screen, Traffic},
    wizard::{Target, fix_for},
};
use tracewing_dev::benchkit;
use tracewing_proxy::refused::{Said, said};
use tracewing_store::settings::StorageSettings;
use tracewing_system::{
    BrowserTrust,
    launch::{self, Browser},
};

fn wait_for(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    what: &str,
    done: impl Fn(&Traffic) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let ok = window
            .update(cx, |view, _, cx| {
                view.look(cx);
                done(view)
            })
            .unwrap();
        cx.run_until_parked();
        if ok {
            return;
        }
        assert!(Instant::now() < deadline, "waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn shot(cx: &mut HeadlessAppContext, window: gpui::AnyWindowHandle, name: &str) {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shots");
    std::fs::create_dir_all(&out).unwrap();
    cx.capture_screenshot(window)
        .unwrap()
        .save(out.join(format!("{name}.png")))
        .unwrap();
}

/// curl through the proxy, trusting only the origin's authority, as a
/// program that was never told about tracewing's does.
fn curl(proxy: std::net::SocketAddr, origin_ca: &std::path::Path, origin: u16) -> bool {
    Command::new("curl")
        .args(["-sS", "-o", "/dev/null", "--max-time", "10", "--proxy"])
        .arg(format!("http://{proxy}"))
        .arg("--cacert")
        .arg(origin_ca)
        .arg(format!("https://localhost:{origin}/json?size=20"))
        .output()
        .expect("curl")
        .status
        .success()
}

#[test]
fn a_certificate_turned_down_is_a_row_and_the_host_can_be_left_alone() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-refused-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let origin_ca = dir.join("origin-ca.pem");
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let origin = listener.local_addr().unwrap().port();
    let ca_out = origin_ca.clone();
    runtime.spawn(async move { benchkit::serve_origin(listener, &ca_out).await });
    while !origin_ca.exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_540..20_550,
        extra_roots: vec![origin_ca.clone()],
        key_store: Default::default(),
        // Looked for, so the row knows it is curl.
        find_apps: true,
    })
    .unwrap();

    let text = Arc::new(gpui_wgpu::CosmicTextSystem::new(tracewing::FONT));
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(tracewing::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(tracewing::load_fonts);
    let data_dir = dir.clone();
    let window: WindowHandle<Traffic> = cx
        .open_window(size(px(1440.), px(900.)), move |window, cx| {
            cx.new(|cx| Traffic::new(engine, &data_dir, window, cx))
        })
        .unwrap();
    let proxy = window
        .update(&mut cx, |view, _, _| view.proxy_addr())
        .unwrap();

    // curl does not trust tracewing's authority. On OpenSSL it lets the
    // handshake finish, checks the certificate after, and resets the
    // connection without asking anything, which is what is seen.
    assert!(
        !curl(proxy, &origin_ca, origin),
        "the proxy's certificate was taken"
    );
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "the refusal", |view| view.count() == 1);
    let host = window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            let detail = view.inspector.detail.as_ref().unwrap();
            assert_eq!(detail.method, "CONNECT");
            let error = detail.error.as_deref().unwrap();
            assert_eq!(said(error), Some(Said::HungUp), "{error}");
            assert_eq!(
                handshake_note(error).as_deref(),
                Some("Hung up before a request")
            );
            detail.host.clone()
        })
        .unwrap();
    assert!(host.starts_with("localhost"), "{host}");
    // The program, once it is found: curl, which trusts what tracewing's
    // terminal tells it to.
    wait_for(&mut cx, window, "the program", |view| {
        view.inspector
            .detail
            .as_ref()
            .is_some_and(|detail| detail.app.is_some())
    });
    let app = window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            view.inspector.detail.as_ref().unwrap().app.clone().unwrap()
        })
        .unwrap();
    let fix = fix_for(Some(&app), false).unwrap_or_else(|| panic!("nothing for {app}"));
    assert_eq!(fix.target, Target::CommandLine, "{app}");
    cx.run_until_parked();
    shot(&mut cx, window.into(), "130-certificate-turned-down");

    // Had it been Firefox, with a profile that does not hold the CA, the
    // flow offers Set up's Install itself; once every profile holds it, it
    // says what is left. Nothing is installed here: the browsers are made
    // up, and Install is not pressed, so no store of this computer's is
    // touched.
    wait_for(&mut cx, window, "the certificate looked for", |view| {
        view.system.ca != CaState::Checking
    });
    let firefox = |holding, of| {
        (
            Browser {
                name: "Firefox",
                engine: launch::Engine::Firefox,
                program: PathBuf::new(),
                profile: PathBuf::new(),
                nss: Vec::new(),
            },
            BrowserTrust::Own { holding, of },
        )
    };
    window
        .update(&mut cx, |view, _, cx| {
            view.inspector.detail.as_mut().unwrap().app = Some("firefox".to_owned());
            view.system.browsers = vec![firefox(0, 1)];
            view.system.environment.no_certutil = None;
            assert_eq!(view.browser_fix(true), BrowserFix::Install);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "133-certificate-firefox-install");
    window
        .update(&mut cx, |view, _, cx| {
            view.system.browsers = vec![firefox(1, 1)];
            assert_eq!(view.browser_fix(true), BrowserFix::Trusted);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "134-certificate-firefox-trusted");
    window
        .update(&mut cx, |view, _, cx| view.select_place(0, cx))
        .unwrap();

    // Set up, open at the answer for it.
    window
        .update(&mut cx, |view, _, cx| {
            view.open_setup_for(Some(fix.target), cx);
            assert_eq!(view.screen, Screen::Setup);
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        Layout::load(&dir.join("layout.toml")).capture_target,
        Some(Target::CommandLine)
    );
    shot(&mut cx, window.into(), "132-certificate-set-up");
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();

    // What "Do not decrypt this host" does: the next connection is passed
    // on as it is, and curl, trusting the origin, gets through.
    window
        .update(&mut cx, |view, _, cx| {
            view.change_rules(|rules| rules.leave_alone(&host), cx)
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "131-certificate-left-alone");
    assert!(
        curl(proxy, &origin_ca, origin),
        "left alone, it went through"
    );
    wait_for(&mut cx, window, "the tunnel", |view| view.count() == 2);

    std::fs::remove_dir_all(&dir).ok();
}
