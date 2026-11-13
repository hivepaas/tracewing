//! The sidebar's lists scroll by themselves, each under a heading that stays:
//! forty hosts are more than fit, and the hosts' heading, its filter and All
//! hosts stay where they are as the list goes by under them.
//!
//! `cargo test --release -p tracewing --features headless --test sidebar`

#![cfg(all(feature = "headless", target_os = "linux"))]

mod common;

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, point, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    input::TextInput,
    traffic::{Screen, Traffic},
};
use tracewing_store::settings::StorageSettings;

fn wait_for(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    what: &str,
    done: impl Fn(&Traffic, &gpui::App) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let ok = window
            .update(cx, |view, _, cx| {
                view.look(cx);
                done(view, cx)
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

fn type_in(input: &Entity<TextInput>, text: &str, cx: &mut gpui::Context<Traffic>) {
    input.update(cx, |input, cx| input.set_text(text, 0..0, cx));
}

fn get(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, url: &str) {
    window
        .update(cx, |view, _, cx| {
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            type_in(&page.url.clone(), url, cx);
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(cx, window, "the answer", |view, _| {
        view.compose_page
            .as_ref()
            .is_some_and(|page| page.answer.is_some() || page.problem.is_some())
    });
}

#[test]
fn the_hosts_scroll_under_their_heading() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-sidebar-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    // Forty servers: forty hosts, a port each.
    let ports: Vec<u16> = (0..40)
        .map(|_| {
            let listener = runtime
                .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
                .unwrap();
            let port = listener.local_addr().unwrap().port();
            runtime.spawn(tracewing_dev::playground::serve(listener));
            port
        })
        .collect();
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_480..20_490,
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
    for port in &ports {
        get(&mut cx, window, &format!("http://localhost:{port}/json"));
    }
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "the forty flows", |view, _| {
        view.count() == 40
    });
    wait_for(&mut cx, window, "the forty hosts", |view, _| {
        view.host_counts().len() == 40
    });
    for _ in 0..3 {
        window.update(&mut cx, |_, w, _| w.refresh()).unwrap();
        cx.run_until_parked();
    }
    let hosts = window
        .update(&mut cx, |view, _, _| view.sidebar_scrolls[2].clone())
        .unwrap();
    shot(&mut cx, window.into(), "128-sidebar-hosts");
    let max = hosts.handle.max_offset().y;
    assert!(max > px(0.), "forty hosts are more than fit");
    let list = hosts.handle.bounds();

    // The bar, dragged to its end: the list goes to its last host, and what
    // is over it (the heading, the filter, All hosts) stays where it was.
    let x = list.right() - px(tracewing::scrollbar::BAR_W / 2.);
    common::drag(
        &mut cx,
        window,
        point(x, list.top() + px(10.)),
        point(x, list.bottom() + px(200.)),
    );
    assert_eq!(-hosts.handle.offset().y, max, "at the last host");
    for _ in 0..2 {
        window.update(&mut cx, |_, w, _| w.refresh()).unwrap();
        cx.run_until_parked();
    }
    assert_eq!(
        hosts.handle.bounds().top(),
        list.top(),
        "the list starts where it did: what is over it stayed"
    );
    shot(&mut cx, window.into(), "129-sidebar-hosts-scrolled");

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
