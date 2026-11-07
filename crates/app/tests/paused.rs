//! Breakpoints made in the window: a response held, changed there and sent on,
//! and a request dropped, through the real proxy.
//!
//! `cargo test --release -p tracewing --features headless --test paused`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    input::TextInput,
    rules_page::RuleKind,
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

fn type_in(input: &Entity<TextInput>, text: &str, cx: &mut gpui::Context<Traffic>) {
    input.update(cx, |input, cx| input.set_text(text, 0..0, cx));
}

fn held(view: &Traffic) -> usize {
    view.paused_page.as_ref().map_or(0, |page| page.held.len())
}

#[test]
fn breakpoints_made_in_the_window_hold_traffic() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-paused-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let origin_ca = dir.join("origin-ca.pem");
    let origin_port = {
        let listener = runtime
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let ca_out = origin_ca.clone();
        runtime.spawn(async move { benchkit::serve_origin(listener, &ca_out).await });
        port
    };
    while !origin_ca.exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        key_store: Default::default(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 19_790..19_800,
        extra_roots: vec![origin_ca],
    })
    .unwrap();
    let proxy = format!("http://{}", engine.addr);
    let ca = dir.join("ca.pem");
    while !ca.exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let client = benchkit::client_through_proxy(&proxy, &ca, false).unwrap();
    let send = |path: &str| {
        let client = client.clone();
        let url = format!("https://localhost:{origin_port}{path}");
        runtime.spawn(async move {
            let res = client.get(url).send().await.unwrap();
            let status = res.status().as_u16();
            (status, res.text().await.unwrap())
        })
    };

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
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Rules, cx);
            view.set_rule_kind(RuleKind::PauseResponse, cx);
            view.fill_rule("Hold JSON", "*/json?size=40*", "", "", cx);
            view.save_rule(cx);
            view.set_rule_kind(RuleKind::PauseRequest, cx);
            view.fill_rule("Hold the other", "*/json?size=41*", "", "", cx);
            view.save_rule(cx);
            assert_eq!(view.traffic_rules().rules.len(), 2);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "30-rules-pause");

    // A response, held: the screen comes up by itself.
    let response = send("/json?size=40");
    wait_for(&mut cx, window, "the held response", |view| {
        held(view) == 1 && view.screen == Screen::Paused
    });
    shot(&mut cx, window.into(), "31-paused-response");

    // A header line that cannot be sent is refused, and it stays held.
    window
        .update(&mut cx, |view, _, cx| {
            let page = view.paused_page.as_ref().unwrap();
            let headers = page.headers.clone();
            type_in(&headers, "content-type: text/plain\nno colon here", cx);
            view.send_paused(cx);
            let page = view.paused_page.as_ref().unwrap();
            assert_eq!(page.problem.as_deref(), Some("Header line 2 has no \":\""));
        })
        .unwrap();
    wait_for(&mut cx, window, "still held", |view| held(view) == 1);

    window
        .update(&mut cx, |view, _, cx| {
            let page = view.paused_page.as_ref().unwrap();
            let (status, headers, body) =
                (page.status.clone(), page.headers.clone(), page.body.clone());
            type_in(&status, "201", cx);
            type_in(&headers, "content-type: text/plain", cx);
            type_in(&body, "changed in the window", cx);
            view.send_paused(cx);
        })
        .unwrap();
    assert_eq!(
        runtime.block_on(response).unwrap(),
        (201, "changed in the window".to_owned())
    );
    wait_for(&mut cx, window, "nothing held", |view| held(view) == 0);

    // A request, dropped: the app gets a 502 that says why.
    let request = send("/json?size=41");
    wait_for(&mut cx, window, "the held request", |view| held(view) == 1);
    shot(&mut cx, window.into(), "32-paused-request");
    window
        .update(&mut cx, |view, _, cx| view.drop_paused(cx))
        .unwrap();
    let (status, text) = runtime.block_on(request).unwrap();
    assert_eq!(status, 502);
    assert!(text.contains("dropped at a breakpoint"), "{text}");

    // Two at once, let go together as they came.
    let first = send("/json?size=41");
    let second = send("/json?size=410");
    wait_for(&mut cx, window, "two held", |view| held(view) == 2);
    window
        .update(&mut cx, |view, _, cx| view.release_all_paused(cx))
        .unwrap();
    assert_eq!(runtime.block_on(first).unwrap().1.len(), 41);
    assert_eq!(runtime.block_on(second).unwrap().1.len(), 410);

    std::fs::remove_dir_all(&dir).ok();
}
