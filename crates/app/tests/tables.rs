//! CSV and TSV bodies shown as tables, through the real proxy from the
//! playground's server.
//!
//! `cargo test --release -p tracewing --features headless --test tables`

#![cfg(all(feature = "headless", target_os = "linux"))]

mod common;

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, point, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    bodyview::{BodyMode, BodyView},
    engine::{Engine, EngineOptions},
    input::TextInput,
    inspector::BodyState,
    select::Copied,
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

#[test]
fn csv_and_tsv_are_tables() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-tables-{}", std::process::id()));
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
        ports: 20_260..20_270,
        extra_roots: Vec::new(),
        key_store: Default::default(),
        find_apps: false,
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

    let base = format!("http://localhost:{port}");
    get(&mut cx, window, &format!("{base}/csv"));
    get(&mut cx, window, &format!("{base}/tsv_big?mb=1"));
    get(&mut cx, window, &format!("{base}/json"));
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Traffic, cx);
            view.select_place(0, cx);
        })
        .unwrap();
    // The playground's five orders and their head.
    wait_for(&mut cx, window, "the CSV as a table", |view, cx| {
        shown(view, cx) == Some((BodyMode::Table, Some(6)))
    });
    shot(&mut cx, window.into(), "81-csv-table");

    // A megabyte of TSV, read as a table as fast.
    window
        .update(&mut cx, |view, _, cx| view.select_place(1, cx))
        .unwrap();
    wait_for(
        &mut cx,
        window,
        "the TSV as a table",
        |view, cx| matches!(shown(view, cx), Some((BodyMode::Table, Some(rows))) if rows > 5_000),
    );

    // As text, and the next table shows as text too, until Table is chosen.
    window
        .update(&mut cx, |view, _, cx| view.set_body_mode(BodyMode::Raw, cx))
        .unwrap();
    window
        .update(&mut cx, |view, _, cx| view.select_place(0, cx))
        .unwrap();
    wait_for(&mut cx, window, "the CSV as text", |view, cx| {
        matches!(shown(view, cx), Some((BodyMode::Raw, Some(6))))
    });
    window
        .update(&mut cx, |view, _, cx| {
            view.set_body_mode(BodyMode::Table, cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the CSV as a table again", |view, cx| {
        shown(view, cx) == Some((BodyMode::Table, Some(6)))
    });

    // Cells are selected as text is, a drag from one to another, and copy
    // as TSV: a tab between cells, a line between rows.
    let table = body(&mut cx, window);
    let (from, to, first, second) = window
        .update(&mut cx, |_, _, cx| {
            let body = table.read(cx);
            let t = body.table().unwrap();
            let cols = t.widths.len() as u64;
            let at = |key: u64| body.selection().drawn_at(key).expect("the cell is drawn");
            let (a, b) = (at(cols), at(2 * cols + 1));
            (
                point(a.left() + px(1.), a.center().y),
                point(b.right(), b.center().y),
                t.cells(1),
                t.cells(2),
            )
        })
        .unwrap();
    common::drag(&mut cx, window, from, to);
    let copied = window
        .update(&mut cx, |_, _, cx| table.read(cx).selected_text())
        .unwrap();
    let Copied::Text(copied) = copied else {
        panic!("cells are selected: {copied:?}");
    };
    let lines: Vec<&str> = copied.lines().collect();
    assert_eq!(lines.len(), 2, "two rows: {copied:?}");
    assert!(
        lines[0].starts_with(&format!("{}\t", first[0])),
        "{copied:?}"
    );
    assert!(
        lines[1].starts_with(&format!("{}\t", second[0])),
        "{copied:?}"
    );
    shot(&mut cx, window.into(), "113-table-selected");

    // A right click opens the menu: Copy, Copy all, Copy row, Copy column.
    common::right_click(&mut cx, window, from);
    let open = window
        .update(&mut cx, |_, _, cx| {
            table.read(cx).selection().menu().is_some()
        })
        .unwrap();
    assert!(open, "the table's menu opens");
    shot(&mut cx, window.into(), "114-table-menu");

    // A table longer than its pane has a bar, and its thumb scrolls it.
    window
        .update(&mut cx, |view, _, cx| view.select_place(1, cx))
        .unwrap();
    wait_for(
        &mut cx,
        window,
        "the TSV as a table again",
        |view, cx| matches!(shown(view, cx), Some((BodyMode::Table, Some(rows))) if rows > 5_000),
    );
    window
        .update(&mut cx, |_, window, _| window.refresh())
        .unwrap();
    cx.run_until_parked();
    let bar = body(&mut cx, window).read_with(&cx, |b, _| b.table_bar().clone());
    assert!(
        bar.handle.max_offset().y > px(0.),
        "the table is longer than its pane"
    );
    let pane = bar.handle.bounds();
    let x = pane.right() - px(tracewing::scrollbar::BAR_W / 2.);
    common::drag(
        &mut cx,
        window,
        point(x, pane.top() + px(10.)),
        point(x, pane.top() + pane.size.height / 2.),
    );
    assert!(
        bar.handle.offset().y < px(0.),
        "the thumb scrolled the table"
    );
    shot(&mut cx, window.into(), "115-table-scrolled");

    // JSON has no table, and Table chosen does not change it.
    window
        .update(&mut cx, |view, _, cx| view.select_place(2, cx))
        .unwrap();
    wait_for(&mut cx, window, "the JSON", |view, cx| {
        shown(view, cx) == Some((BodyMode::Pretty, None))
    });

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
