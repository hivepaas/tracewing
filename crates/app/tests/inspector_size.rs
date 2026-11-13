//! The inspector's − and + buttons: smaller or larger by a tenth of the window
//! (at least 100 px) at a time, taller below the list and wider beside it, never under 100 px and never so large
//! that the list has less than its own least.
//!
//! `cargo test --release -p tracewing --features headless --test inspector_size`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{path::PathBuf, sync::Arc};
use tracewing::{
    engine::{Engine, EngineOptions},
    layout::{LIST_MIN, Layout, Place},
    traffic::{PANE_MIN, Screen, Traffic},
};
use tracewing_store::settings::StorageSettings;

fn shot(cx: &mut HeadlessAppContext, window: gpui::AnyWindowHandle, name: &str) {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shots");
    std::fs::create_dir_all(&out).unwrap();
    cx.capture_screenshot(window)
        .unwrap()
        .save(out.join(format!("{name}.png")))
        .unwrap();
}

/// Presses − or + `times` times, and says the layout as it was kept.
fn press(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    dir: &std::path::Path,
    larger: bool,
    times: usize,
) -> Layout {
    for _ in 0..times {
        window
            .update(cx, |view, window, cx| {
                view.resize_inspector(larger, window, cx)
            })
            .unwrap();
        cx.run_until_parked();
    }
    Layout::load(&dir.join("layout.toml"))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn minus_and_plus_resize_the_inspector_by_a_tenth_of_the_window() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-inspector-size-{}", std::process::id()));
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
        ports: 20_440..20_450,
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

    // Sizes are kept at the design's (13 px text); the window draws at the
    // default 14, so a pixel on the screen is 13/14 of one kept.
    let kept = |px: f32| px * 13. / f32::from(tracewing::text_size::DEFAULT);

    // Below the list: − and + change its height, by a tenth of the window's
    // but at least 100 px: the window is 900 high, so 100.
    main.update(&mut cx, |view, _, cx| {
        view.set_screen(Screen::Traffic, cx);
        view.set_place(Place::Below, cx);
    })
    .unwrap();
    cx.run_until_parked();
    let start = Layout::default();
    let shorter = press(&mut cx, main, &dir, false, 1);
    assert!(
        close(shorter.inspector_h, start.inspector_h - kept(100.)),
        "{}",
        shorter.inspector_h
    );
    assert!(
        close(shorter.inspector_w, start.inspector_w),
        "the width is left alone"
    );
    let taller = press(&mut cx, main, &dir, true, 1);
    assert!(
        close(taller.inspector_h, start.inspector_h),
        "{}",
        taller.inspector_h
    );
    shot(&mut cx, main.into(), "118-inspector-size-below");
    // Never under 100 px.
    let least = press(&mut cx, main, &dir, false, 40);
    assert!(
        close(least.inspector_h, kept(PANE_MIN)),
        "{}",
        least.inspector_h
    );
    // Nor so tall that the list has less than its least: the window is 900
    // high, and the inspector stops short of it.
    let most = press(&mut cx, main, &dir, true, 60);
    assert!(most.inspector_h < 900. - LIST_MIN, "{}", most.inspector_h);
    assert_eq!(
        press(&mut cx, main, &dir, true, 1).inspector_h,
        most.inspector_h,
        "no further"
    );

    // Beside it: the width instead, by a tenth of the window's: 144 of 1440.
    main.update(&mut cx, |view, _, cx| view.set_place(Place::Beside, cx))
        .unwrap();
    cx.run_until_parked();
    let before = Layout::load(&dir.join("layout.toml"));
    let narrower = press(&mut cx, main, &dir, false, 1);
    assert!(
        close(narrower.inspector_w, before.inspector_w - kept(144.)),
        "{}",
        narrower.inspector_w
    );
    assert!(
        close(narrower.inspector_h, before.inspector_h),
        "the height is left alone"
    );
    let wider = press(&mut cx, main, &dir, true, 2);
    assert!(
        close(wider.inspector_w, before.inspector_w + kept(144.)),
        "{}",
        wider.inspector_w
    );
    shot(&mut cx, main.into(), "119-inspector-size-beside");
    let narrowest = press(&mut cx, main, &dir, false, 40).inspector_w;
    assert!(close(narrowest, kept(PANE_MIN)), "{narrowest}");
    let widest = press(&mut cx, main, &dir, true, 60);
    assert!(
        widest.inspector_w < 1440. - LIST_MIN,
        "{}",
        widest.inspector_w
    );

    // Made as wide as it goes with the sidebar hidden, then the sidebar shown
    // again: the inspector gives way rather than going past the window's
    // edge, its body and scroll bars with it; the width chosen is kept.
    main.update(&mut cx, |view, _, cx| {
        view.new_compose(cx);
        let page = view.compose_page.as_ref().unwrap();
        let url = format!("http://localhost:{port}/json");
        page.url
            .clone()
            .update(cx, |input, cx| input.set_text(&url, 0..0, cx));
        view.send_composed(cx);
    })
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !main
        .update(&mut cx, |view, _, _| {
            view.compose_page
                .as_ref()
                .is_some_and(|p| p.answer.is_some())
        })
        .unwrap()
    {
        assert!(
            std::time::Instant::now() < deadline,
            "waiting for the answer"
        );
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    main.update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    while !main
        .update(&mut cx, |view, _, cx| {
            view.look(cx);
            view.count() == 1
        })
        .unwrap()
    {
        assert!(std::time::Instant::now() < deadline, "waiting for the flow");
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    main.update(&mut cx, |view, _, cx| {
        view.select_place(0, cx);
        view.set_side(tracewing::detail::Side::Response, cx);
        view.set_part(tracewing::inspector::Part::Body, cx);
    })
    .unwrap();
    let toggle_sidebar = |cx: &mut HeadlessAppContext| {
        main.update(cx, |view, window, cx| {
            view.do_key(tracewing::shortcuts::Action::Sidebar, window, cx);
        })
        .unwrap();
        cx.run_until_parked();
    };
    while !main
        .update(&mut cx, |view, _, cx| {
            view.look(cx);
            matches!(
                view.inspector.body,
                tracewing::inspector::BodyState::Shown(_)
            )
        })
        .unwrap()
    {
        assert!(std::time::Instant::now() < deadline, "waiting for the body");
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    toggle_sidebar(&mut cx);
    let chosen = press(&mut cx, main, &dir, true, 60).inspector_w;
    toggle_sidebar(&mut cx);
    for _ in 0..3 {
        main.update(&mut cx, |_, window, _| window.refresh())
            .unwrap();
        cx.run_until_parked();
    }
    let body = main
        .update(&mut cx, |view, _, _| match &view.inspector.body {
            tracewing::inspector::BodyState::Shown(body) => Some(body.clone()),
            _ => None,
        })
        .unwrap()
        .expect("the flow's body is shown");
    let right = main
        .update(&mut cx, |_, _, cx| body.read(cx).drawn_bounds().right())
        .unwrap();
    assert!(
        right <= px(1440.),
        "the body ends inside the window: {right:?}"
    );
    assert_eq!(
        Layout::load(&dir.join("layout.toml")).inspector_w,
        chosen,
        "the width chosen is kept for when there is room"
    );
    shot(&mut cx, main.into(), "125-inspector-gives-way");

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
