//! The list's columns: sorted by a header, hidden and shown from the menu, made
//! wider, and kept that way for the next start.
//!
//! `cargo test --release -p tracewing --features headless --test columns`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    columns::Column,
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

/// The `size` asked for by the flows in the list, top to bottom.
fn sizes(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>) -> Vec<String> {
    window
        .update(cx, |view, _, cx| {
            (0..view.count())
                .map(|place| {
                    view.select_place(place, cx);
                    let url = &view.inspector.detail.as_ref().unwrap().url;
                    url.rsplit_once('=').unwrap().1.to_owned()
                })
                .collect()
        })
        .unwrap()
}

#[test]
fn columns_sort_hide_and_widen() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-columns-{}", std::process::id()));
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
        ports: 20_210..20_220,
        extra_roots: vec![origin_ca],
        key_store: Default::default(),
        find_apps: true,
    })
    .unwrap();
    while !dir.join("ca.pem").exists() {
        std::thread::sleep(Duration::from_millis(5));
    }

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

    // One after another, so they come in this order.
    for size in [200, 5, 5000] {
        let out = Command::new("curl")
            .args(["-sS", "-o", "/dev/null", "--proxy"])
            .arg(format!("http://{proxy}"))
            .arg("--cacert")
            .arg(dir.join("ca.pem"))
            .arg(format!("https://localhost:{origin}/json?size={size}"))
            .output()
            .expect("curl");
        assert!(out.status.success(), "{out:?}");
    }
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "three flows", |view| view.count() == 3);
    assert_eq!(sizes(&mut cx, window), ["200", "5", "5000"]);

    // Up, down, then back to the order they came.
    let sorted = |view: &Traffic| !view.is_filtering() && view.count() == 3;
    window
        .update(&mut cx, |view, _, cx| view.sort_by(Column::Size, cx))
        .unwrap();
    wait_for(&mut cx, window, "smallest first", sorted);
    assert_eq!(sizes(&mut cx, window), ["5", "200", "5000"]);
    window
        .update(&mut cx, |view, _, cx| view.sort_by(Column::Size, cx))
        .unwrap();
    wait_for(&mut cx, window, "largest first", sorted);
    assert_eq!(sizes(&mut cx, window), ["5000", "200", "5"]);

    // A filter keeps the sort.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_filter_text("status:200", cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "filtered and sorted", sorted);
    assert_eq!(sizes(&mut cx, window), ["5000", "200", "5"]);

    // Hidden, made wider, and the menu open.
    window
        .update(&mut cx, |view, _, cx| {
            view.toggle_column(Column::App, cx);
            view.toggle_column(Column::Number, cx);
            view.set_column_width(Column::Host, 260., cx);
            view.set_columns_menu(true, cx);
            assert!(!view.columns().is_shown(Column::App));
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "58-columns");
    let kept = Layout::load(&dir.join("layout.toml")).columns;
    assert!(!kept.is_shown(Column::App), "kept for next time");
    assert!(!kept.is_shown(Column::Number));
    assert_eq!(kept.width(Column::Host), 260.);

    // Hiding the column sorted by stops the sort.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_columns_menu(false, cx);
            view.set_filter_text("", cx);
            view.toggle_column(Column::Size, cx);
            assert_eq!(view.sort(), None);
        })
        .unwrap();
    wait_for(&mut cx, window, "the order they came", |view| {
        !view.is_filtering() && view.count() == 3
    });
    assert_eq!(sizes(&mut cx, window), ["200", "5", "5000"]);
    shot(&mut cx, window.into(), "59-columns-hidden");

    window
        .update(&mut cx, |view, _, cx| {
            view.reset_columns(cx);
            assert!(view.columns().is_default());
        })
        .unwrap();
    assert!(Layout::load(&dir.join("layout.toml")).columns.is_default());

    std::fs::remove_dir_all(&dir).ok();
}
