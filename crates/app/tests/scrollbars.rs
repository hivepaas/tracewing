//! Scroll bars: the inspector's parts that scroll by themselves (Headers
//! here) draw one when there is more than fits, and its thumb drags them.
//!
//! `cargo test --release -p tracewing --features headless --test scrollbars`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{
    AppContext as _, Entity, HeadlessAppContext, Modifiers, PlatformInput, ScrollDelta,
    ScrollWheelEvent, TouchPhase, WindowHandle, point, px, size,
};

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
    inspector::{BodyState, Part},
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

    // Lines wider than the pane go sideways: the wheel with Shift held, or the
    // bar along the bottom. A press still finds the character under it.
    get(
        &mut cx,
        main,
        &format!("http://localhost:{port}/json_long"),
        "",
    );
    main.update(&mut cx, |view, _, cx| {
        view.set_screen(Screen::Traffic, cx);
        view.select_place(1, cx);
    })
    .unwrap();
    wait_for(&mut cx, main, "the long JSON", |view| {
        view.inspector
            .detail
            .as_ref()
            .is_some_and(|d| d.url.contains("json_long"))
    });
    main.update(&mut cx, |view, _, cx| {
        view.set_side(Side::Response, cx);
        view.set_part(Part::Body, cx);
    })
    .unwrap();
    wait_for(&mut cx, main, "its body", |view| {
        matches!(&view.inspector.body, BodyState::Shown(_))
    });
    let body = main
        .update(&mut cx, |view, _, _| match &view.inspector.body {
            BodyState::Shown(body) => body.clone(),
            _ => panic!("a body is shown"),
        })
        .unwrap();
    let line = |cx: &mut HeadlessAppContext| {
        main.update(cx, |_, _, cx| body.read(cx).selection().drawn_at(1))
            .unwrap()
            .expect("the long line is drawn")
    };
    let before = line(&mut cx);
    let across_of = |cx: &mut HeadlessAppContext| {
        main.update(cx, |_, _, cx| body.read(cx).scroll_x())
            .unwrap()
    };
    assert_eq!(across_of(&mut cx), 0.);
    common::mouse(
        &mut cx,
        main,
        PlatformInput::ScrollWheel(ScrollWheelEvent {
            position: before.center(),
            delta: ScrollDelta::Pixels(point(px(0.), px(-300.))),
            modifiers: Modifiers {
                shift: true,
                ..Default::default()
            },
            touch_phase: TouchPhase::Moved,
        }),
    );
    let across = across_of(&mut cx);
    assert!(across > 0., "Shift and the wheel went sideways");
    let after = line(&mut cx);
    assert!(
        (after.left() - (before.left() - px(across))).abs() < px(1.),
        "the text is laid out further left by as much: {before:?} {after:?} {across}"
    );
    // A press at the far end of the bottom bar goes to the end.
    let list = main
        .update(&mut cx, |_, _, cx| body.read(cx).drawn_bounds())
        .unwrap();
    let end = point(
        list.right() - px(tracewing::scrollbar::BAR_W + 4.),
        list.bottom() - px(tracewing::scrollbar::BAR_W / 2.),
    );
    common::drag(&mut cx, main, end, end);
    let further = across_of(&mut cx);
    assert!(
        further > across,
        "the bar's end went further: {further} after {across}"
    );
    shot(&mut cx, main.into(), "117-body-sideways");

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The flow list's bar is the one every area has, and a drag on it stops the
/// list following the newest flow, as a wheel does. The list once drew a bar
/// of its own for exactly this: a bar that moved the handle straight would
/// leave the list following, and the next flow to arrive would pull it back
/// to the bottom out from under the hand that had just moved it.
///
/// Claims `ports: 20_720..20_730` for its proxy, the next ten free.
#[test]
fn dragging_the_lists_bar_stops_it_following_the_newest_flow() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-list-bar-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let origin = listener.local_addr().unwrap().port();
    runtime.spawn(tracewing_dev::playground::serve(listener));
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_720..20_730,
        extra_roots: Vec::new(),
        key_store: Default::default(),
        find_apps: false,
    })
    .unwrap();
    let proxy = engine.addr;
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
    main.update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://{proxy}")).unwrap())
        .build()
        .unwrap();
    let send = |from: usize, to: usize| {
        runtime.block_on(async {
            for i in from..to {
                client
                    .get(format!("http://localhost:{origin}/json?n={i}"))
                    .send()
                    .await
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap();
            }
        })
    };
    send(0, 80);
    wait_for(&mut cx, main, "eighty flows", |view| view.count() >= 80);
    // Drawn again: the list knows its height, and the bar its thumb, once
    // the list has been laid out.
    main.update(&mut cx, |_, window, _| window.refresh())
        .unwrap();
    cx.run_until_parked();
    let at = |cx: &mut HeadlessAppContext| {
        main.update(cx, |view, _, _| {
            let (at, list) = view.list_at();
            (at, list, view.max_scroll(), view.following())
        })
        .unwrap()
    };
    let (start, list, max, following) = at(&mut cx);
    assert!(max > 0., "eighty flows are more than fit");
    assert!(following, "a list nobody has moved follows the newest flow");
    assert!(
        (start - max).abs() < 1.,
        "so it starts at its bottom: {start} of {max}"
    );

    // The thumb, at the bottom of the bar while the list follows, taken
    // hold of and dragged to the top.
    let x = list.right() - px(tracewing::scrollbar::BAR_W / 2.);
    common::drag(
        &mut cx,
        main,
        point(x, list.bottom() - px(8.)),
        point(x, list.top() + px(8.)),
    );
    let (moved, _, _, following) = at(&mut cx);
    assert!(!following, "the drag stopped the list following");
    assert!(
        moved < max / 2.,
        "the drag took the list up: {moved} of {max}"
    );
    // Still under the pointer, let go: the thumb as wide as it grows.
    shot(&mut cx, main.into(), "145-list-bar-under-the-pointer");

    // Another flow: the list stays where the hand left it.
    send(80, 81);
    wait_for(&mut cx, main, "the eighty-first flow", |view| {
        view.count() >= 81
    });
    let (after, _, _, _) = at(&mut cx);
    assert_eq!(after, moved, "a new flow pulled the list back down");
    let _ = std::fs::remove_dir_all(&dir);
}
