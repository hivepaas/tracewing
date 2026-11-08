//! Hosts in the sidebar: pinned to the top, hidden from the list and shown again,
//! and kept that way for the next start.
//!
//! `cargo test --release -p tracewing --features headless --test hosts`

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
    layout::Layout,
    traffic::{Screen, Traffic},
};
use tracewing_dev::benchkit;
use tracewing_store::settings::StorageSettings;

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

fn shown(view: &Traffic) -> Vec<(String, u64, bool)> {
    view.host_marks()
        .arrange(view.host_counts())
        .0
        .iter()
        .map(|line| (line.host.to_string(), line.count, line.pinned))
        .collect()
}

#[test]
fn hosts_are_pinned_hidden_and_shown_again() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-hosts-{}", std::process::id()));
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
    let start = |cx: &mut HeadlessAppContext| {
        let engine = Engine::start(EngineOptions {
            data_dir: dir.clone(),
            settings: StorageSettings {
                disk_reserve_mib: 1,
                ..StorageSettings::default()
            },
            rules: Default::default(),
            traffic_rules: Default::default(),
            ports: 20_220..20_230,
            extra_roots: vec![origin_ca.clone()],
            key_store: Default::default(),
            find_apps: true,
        })
        .unwrap();
        while !dir.join("ca.pem").exists() {
            std::thread::sleep(Duration::from_millis(5));
        }
        let data_dir = dir.clone();
        let window: WindowHandle<Traffic> = cx
            .open_window(size(px(1440.), px(900.)), move |window, cx| {
                cx.new(|cx| Traffic::new(engine, &data_dir, window, cx))
            })
            .unwrap();
        window
            .update(cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
            .unwrap();
        window
    };

    let text = Arc::new(gpui_wgpu::CosmicTextSystem::new(tracewing::FONT));
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(tracewing::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(tracewing::load_fonts);
    let window = start(&mut cx);
    let proxy = window
        .update(&mut cx, |view, _, _| view.proxy_addr())
        .unwrap();

    // The same server by two names: two hosts, the busier one first.
    let busy = format!("localhost:{origin}");
    let quiet = format!("127.0.0.1:{origin}");
    for host in [&busy, &busy, &busy, &quiet] {
        let out = Command::new("curl")
            .args(["-sS", "-o", "/dev/null", "--proxy"])
            .arg(format!("http://{proxy}"))
            .arg("--cacert")
            .arg(dir.join("ca.pem"))
            .arg(format!("https://{host}/json?size=5"))
            .output()
            .expect("curl");
        assert!(out.status.success(), "{out:?}");
    }
    let expected = vec![(busy.clone(), 3, false), (quiet.clone(), 1, false)];
    wait_for(&mut cx, window, "both hosts counted", move |view| {
        view.count() == 4 && shown(view) == expected
    });

    // Pinned, the quiet one goes first.
    window
        .update(&mut cx, |view, _, cx| view.toggle_pin(&quiet, cx))
        .unwrap();
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(
                shown(view),
                [(quiet.clone(), 1, true), (busy.clone(), 3, false)]
            );
        })
        .unwrap();

    // Hidden, the busy one's flows leave the list, and it leaves the hosts.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_host(Some(busy.clone()), cx);
            view.toggle_hidden(&busy, cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "its flows gone", |view| {
        !view.is_filtering() && view.count() == 1
    });
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(shown(view), [(quiet.clone(), 1, true)]);
            assert!(view.host_marks().is_hidden(&busy));
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "60-hosts");

    let kept = Layout::load(&dir.join("layout.toml")).hosts;
    assert!(kept.is_pinned(&quiet) && kept.is_hidden(&busy));

    // Kept for next time.
    window
        .update(&mut cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    let window = start(&mut cx);
    window
        .update(&mut cx, |view, _, _| {
            assert!(view.host_marks().is_pinned(&quiet));
            assert!(view.host_marks().is_hidden(&busy));
        })
        .unwrap();

    // Shown again.
    window
        .update(&mut cx, |view, _, cx| view.toggle_hidden(&busy, cx))
        .unwrap();
    window
        .update(&mut cx, |view, _, _| {
            assert!(view.host_marks().hidden.is_empty());
        })
        .unwrap();

    std::fs::remove_dir_all(&dir).ok();
}
