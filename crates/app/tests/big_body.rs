//! A body past 50 MB in the window, which features.md says it keeps up with:
//! 60 MB of JSON recorded, shown, scanned to its end, laid out pretty, found
//! in, and shown at its last line. The times are printed and bounded loosely,
//! so that a body which never finishes, or takes minutes, fails rather than
//! passes slowly; they are not a benchmark.
//!
//! `cargo test --release -p tracewing --features headless --test big_body -- --nocapture`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    bodyview::{BodyMode, BodyView},
    engine::{Engine, EngineOptions},
    input::TextInput,
    inspector::{BodyState, Part},
    traffic::{Screen, Traffic},
};
use tracewing_store::settings::StorageSettings;

/// How much: past the 50 MB features.md names.
const MB: u64 = 60;

fn wait_for(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    what: &str,
    limit: Duration,
    done: impl Fn(&Traffic, &gpui::App) -> bool,
) -> Duration {
    let started = Instant::now();
    loop {
        let ok = window
            .update(cx, |view, _, cx| {
                view.look(cx);
                done(view, cx)
            })
            .unwrap();
        cx.run_until_parked();
        if ok {
            return started.elapsed();
        }
        assert!(started.elapsed() < limit, "waited {limit:?} for {what}");
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

fn body(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>) -> Entity<BodyView> {
    window
        .update(cx, |view, _, _| match &view.inspector.body {
            BodyState::Shown(body) => body.clone(),
            _ => panic!("a body is shown"),
        })
        .unwrap()
}

#[test]
fn a_body_past_50_mb_is_shown_scanned_and_found_in() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-big-body-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    runtime.spawn(tracewing_dev::playground::serve(listener));
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_660..20_670,
        extra_roots: Vec::new(),
        key_store: Default::default(),
        find_apps: false,
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

    // Recorded through the proxy, as an app's answer is.
    let proxy = window
        .update(&mut cx, |view, _, _| view.proxy_addr())
        .unwrap();
    let out = std::process::Command::new("curl")
        .args([
            "-sS",
            "-o",
            "/dev/null",
            "-w",
            "%{size_download}",
            "--proxy",
        ])
        .arg(format!("http://{proxy}"))
        .arg(format!("http://localhost:{port}/json_big?mb={MB}"))
        .output()
        .expect("curl");
    assert!(out.status.success(), "{out:?}");
    let got: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap();
    // The playground closes its JSON past the size asked, by a few bytes.
    assert!(got >= MB * 1024 * 1024, "only {got} bytes came back");
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(
        &mut cx,
        window,
        "the flow",
        Duration::from_secs(60),
        |view, _| view.count() == 1,
    );

    // Shown, then scanned to its end, then pretty.
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            view.set_side(tracewing::detail::Side::Response, cx);
            view.set_part(Part::Body, cx);
        })
        .unwrap();
    let shown = wait_for(
        &mut cx,
        window,
        "the body shown",
        Duration::from_secs(60),
        |view, _| matches!(view.inspector.body, BodyState::Shown(_)),
    );
    let scanned = wait_for(
        &mut cx,
        window,
        "the body scanned to its end",
        Duration::from_secs(120),
        |view, cx| match &view.inspector.body {
            BodyState::Shown(body) => body.read(cx).body().is_scanned(),
            _ => false,
        },
    );
    let view = body(&mut cx, window);
    let (lines, mode, size, scan) = window
        .update(&mut cx, |_, _, cx| {
            let b = view.read(cx);
            (
                b.body().lines(),
                b.mode(),
                b.body().size(),
                b.body().scan_time(),
            )
        })
        .unwrap();
    eprintln!(
        "{MB} MB: shown in {shown:?}, scanned in {scanned:?} ({scan:?} by its own clock), {lines} lines"
    );
    assert_eq!(size, got, "the body kept whole");
    assert_eq!(mode, BodyMode::Pretty, "laid out as JSON, not as text");
    assert!(lines > 1_000_000, "{lines} lines for {MB} MB of orders");
    shot(&mut cx, window.into(), "143-body-60mb");

    // Found in: a word on every order stops at Find's cap, and says so.
    window
        .update(&mut cx, |view, window, cx| view.open_find(window, cx))
        .unwrap();
    window
        .update(&mut cx, |view, _, cx| {
            let input: Entity<TextInput> = view.inspector.find.as_ref().unwrap().input.clone();
            input.update(cx, |input, cx| input.set_text("Lyon", 4..4, cx));
        })
        .unwrap();
    let found = wait_for(
        &mut cx,
        window,
        "Find to finish",
        Duration::from_secs(60),
        |_, cx| view.read(cx).find_count().is_some_and(|c| c.done),
    );
    let count = window
        .update(&mut cx, |_, _, cx| view.read(cx).find_count().unwrap())
        .unwrap();
    eprintln!("found {} in {found:?}", count.found);
    assert!(count.found > 0, "{count:?}");

    // And its last line, from the end key, drawn.
    window
        .update(&mut cx, |view, window, cx| view.close_find(window, cx))
        .unwrap();
    window
        .update(&mut cx, |_, _, cx| {
            view.update(cx, |b, cx| {
                b.set_position(f64::MAX);
                cx.notify();
            });
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "144-body-60mb-end");

    std::fs::remove_dir_all(&dir).ok();
}
