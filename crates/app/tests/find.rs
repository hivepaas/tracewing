//! Find in the inspector: what is typed found in the body, plain, with case,
//! as whole words or a regular expression, stepped through with Enter, and
//! in a CSV table's cells; Ctrl+F finds in the inspector when it was pressed
//! in last, and filters the list otherwise.
//!
//! `cargo test --release -p tracewing --features headless --test find`

#![cfg(all(feature = "headless", target_os = "linux"))]

mod common;

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
    inspector::{BodyState, FindSwitch},
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

/// The body shown.
fn body(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>) -> Entity<BodyView> {
    window
        .update(cx, |view, _, _| match &view.inspector.body {
            BodyState::Shown(body) => body.clone(),
            _ => panic!("a body is shown"),
        })
        .unwrap()
}

/// The body shown's mode, and its table's rows if it has one.
fn shown(view: &Traffic, cx: &gpui::App) -> Option<(BodyMode, Option<usize>)> {
    let BodyState::Shown(body) = &view.inspector.body else {
        return None;
    };
    let body = body.read(cx);
    Some((body.mode(), body.table().map(|t| t.rows())))
}

fn find_count(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
) -> Option<tracewing::bodyview::FindCount> {
    let view = body(cx, window);
    window
        .update(cx, |_, _, cx| view.read(cx).find_count())
        .unwrap()
}

/// Types `text` into Find, and waits until the whole body is searched.
fn find(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, text: &str) -> usize {
    window
        .update(cx, |view, _, cx| {
            let input = view
                .inspector
                .find
                .as_ref()
                .expect("Find is open")
                .input
                .clone();
            input.update(cx, |input, cx| {
                input.set_text(text, text.len()..text.len(), cx)
            });
        })
        .unwrap();
    wait_for(cx, window, "the search", |view, cx| {
        match &view.inspector.body {
            BodyState::Shown(body) => body.read(cx).find_count().is_some_and(|c| c.done),
            _ => false,
        }
    });
    find_count(cx, window).map_or(0, |c| c.found)
}

#[test]
fn find_in_the_body_and_in_a_table() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-find-{}", std::process::id()));
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
        ports: 20_450..20_460,
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

    let base = format!("http://localhost:{port}");
    get(&mut cx, window, &format!("{base}/json_big?mb=1"));
    get(&mut cx, window, &format!("{base}/csv"));
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Traffic, cx);
            view.select_place(0, cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the JSON", |view, cx| {
        matches!(shown(view, cx), Some((BodyMode::Pretty, None)))
    });

    // Opened from the inspector; the first found is shown at once.
    window
        .update(&mut cx, |view, window, cx| view.open_find(window, cx))
        .unwrap();
    let plain = find(&mut cx, window, "Lyon");
    assert!(plain > 10, "a megabyte of orders has many in Lyon: {plain}");
    let count = find_count(&mut cx, window).unwrap();
    assert_eq!(count.now, Some(0));
    // Enter steps to the next, which is drawn where it is.
    common::key(&mut cx, window, "enter");
    let count = find_count(&mut cx, window).unwrap();
    assert_eq!(count.now, Some(1));
    shot(&mut cx, window.into(), "121-find-body");

    // Case: LYON is not Lyon.
    assert_eq!(find(&mut cx, window, "LYON"), plain, "any case by default");
    window
        .update(&mut cx, |view, _, cx| view.flip_find(FindSwitch::Case, cx))
        .unwrap();
    assert_eq!(find(&mut cx, window, "LYON"), 0, "with case, none");
    window
        .update(&mut cx, |view, _, cx| view.flip_find(FindSwitch::Case, cx))
        .unwrap();

    // Whole words: "id" is in "paid" too, until words are asked for.
    let any = find(&mut cx, window, "id");
    window
        .update(&mut cx, |view, _, cx| view.flip_find(FindSwitch::Word, cx))
        .unwrap();
    let words = find(&mut cx, window, "id");
    assert!(words > 0 && words < any, "whole words {words} of {any}");
    window
        .update(&mut cx, |view, _, cx| view.flip_find(FindSwitch::Word, cx))
        .unwrap();

    // A regular expression, and one that does not work, which says why.
    window
        .update(&mut cx, |view, _, cx| view.flip_find(FindSwitch::Regex, cx))
        .unwrap();
    assert!(find(&mut cx, window, r#""id":\d{3},"#) > 0);
    window
        .update(&mut cx, |view, _, cx| {
            let input = view.inspector.find.as_ref().unwrap().input.clone();
            input.update(cx, |input, cx| input.set_text("(", 1..1, cx));
        })
        .unwrap();
    cx.run_until_parked();
    let problem = window
        .update(&mut cx, |view, _, _| {
            view.inspector.find.as_ref().unwrap().problem.clone()
        })
        .unwrap();
    assert!(problem.is_some(), "a bad expression says why");
    window
        .update(&mut cx, |view, _, cx| view.flip_find(FindSwitch::Regex, cx))
        .unwrap();

    // Esc closes it, and nothing is marked any more.
    common::key(&mut cx, window, "escape");
    let open = window
        .update(&mut cx, |view, _, _| view.inspector.find.is_some())
        .unwrap();
    assert!(!open, "Esc closes Find");
    assert!(find_count(&mut cx, window).is_none());

    // Ctrl+F: after a press in the inspector, Find; after one in the list,
    // the list's filter.
    let in_body = {
        let view = body(&mut cx, window);
        window
            .update(&mut cx, |_, _, cx| view.read(cx).drawn_bounds().center())
            .unwrap()
    };
    common::drag(&mut cx, window, in_body, in_body);
    common::key(&mut cx, window, "ctrl-f");
    let open = window
        .update(&mut cx, |view, _, _| view.inspector.find.is_some())
        .unwrap();
    assert!(open, "Ctrl+F after a press in the inspector opens Find");

    // In a CSV table, its cells.
    window
        .update(&mut cx, |view, _, cx| view.select_place(1, cx))
        .unwrap();
    wait_for(&mut cx, window, "the CSV as a table", |view, cx| {
        matches!(shown(view, cx), Some((BodyMode::Table, Some(_))))
    });
    let cells = find(&mut cx, window, "Lyon");
    assert!(cells > 0, "Lyon is in the table's cells");
    shot(&mut cx, window.into(), "122-find-table");

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
