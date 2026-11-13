//! Scroll bars: the inspector's parts that scroll by themselves (Headers
//! here) draw one when there is more than fits, and its thumb drags them.
//!
//! `cargo test --release -p tracewing --features headless --test scrollbars`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, point, px, size};

mod common;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    detail::Side,
    engine::{Engine, EngineOptions},
    input::TextInput,
    inspector::Part,
    layout::Sides,
    traffic::{Screen, Traffic},
};
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

fn get(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, url: &str, headers: &str) {
    window
        .update(cx, |view, _, cx| {
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            type_in(&page.url.clone(), url, cx);
            type_in(&page.headers.clone(), headers, cx);
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(cx, window, "the answer", |view| {
        view.compose_page
            .as_ref()
            .is_some_and(|page| page.answer.is_some() || page.problem.is_some())
    });
}

#[test]
fn a_part_longer_than_its_pane_has_a_bar_that_drags_it() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-scrollbars-{}", std::process::id()));
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
        ports: 20_430..20_440,
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
    let main: WindowHandle<Traffic> = cx
        .open_window(size(px(1440.), px(900.)), move |window, cx| {
            cx.new(|cx| Traffic::new(engine, &data_dir, window, cx))
        })
        .unwrap();

    // Sixty headers: far more than the pane shows.
    let headers: String = (1..=60)
        .map(|i| format!("x-header-{i:02}: value number {i}\n"))
        .collect();
    get(
        &mut cx,
        main,
        &format!("http://localhost:{port}/json_big?mb=1"),
        &headers,
    );
    main.update(&mut cx, |view, _, cx| {
        view.set_screen(Screen::Traffic, cx);
        view.set_sides(Sides::Tabs, cx);
        view.select_place(0, cx);
    })
    .unwrap();
    wait_for(&mut cx, main, "the flow", |view| {
        view.inspector.detail.is_some()
    });
    main.update(&mut cx, |view, _, cx| {
        view.set_side(Side::Request, cx);
        view.set_part(Part::Headers, cx);
    })
    .unwrap();
    cx.run_until_parked();
    // Drawn twice: the bar knows how far the part goes once it is laid out.
    main.update(&mut cx, |_, window, _| window.refresh())
        .unwrap();
    cx.run_until_parked();
    let scrolled = main
        .update(&mut cx, |view, _, _| {
            view.inspector.scrolled(Part::Headers, Side::Request)
        })
        .unwrap();
    let max = scrolled.handle.max_offset().y;
    assert!(max > px(0.), "sixty headers are more than fit");
    assert_eq!(scrolled.handle.offset().y, px(0.));
    shot(&mut cx, main.into(), "110-headers-scrollbar");

    // The thumb, at the top of the bar over the pane's right edge, dragged
    // down: the headers scroll with it.
    let pane = scrolled.handle.bounds();
    let x = pane.right() - px(tracewing::scrollbar::BAR_W / 2.);
    let from = point(x, pane.top() + px(10.));
    let to = point(x, pane.top() + px(10.) + pane.size.height / 2.);
    common::drag(&mut cx, main, from, to);
    let down = -scrolled.handle.offset().y;
    assert!(down > px(0.), "the drag scrolled the headers");
    // Down to the end, and no further.
    let end = point(x, pane.bottom() + px(400.));
    common::drag(&mut cx, main, to, end);
    assert_eq!(-scrolled.handle.offset().y, max, "at the end");
    shot(&mut cx, main.into(), "111-headers-scrolled");

    // The body's bar, twice as thick as it was.
    main.update(&mut cx, |view, _, cx| {
        view.set_side(Side::Response, cx);
        view.set_part(Part::Body, cx);
    })
    .unwrap();
    cx.run_until_parked();
    shot(&mut cx, main.into(), "112-body-scrollbar");

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
