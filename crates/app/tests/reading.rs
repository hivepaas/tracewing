//! A value read where it stands: what a right click offers for the selection,
//! and the reading chosen drawn beside the value.
//!
//! `cargo test --release -p tracewing --features headless --test reading`

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
    inspector::{BodyState, Part},
    select::Spot,
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

/// The line a field sits on, and where its value starts and ends in it.
fn value_of(text: &str, field: &str) -> Option<(usize, usize)> {
    let at = text.find(&format!("\"{field}\":"))? + field.len() + 3;
    let rest = text[at..].trim_start();
    let from = at + text[at..].len() - rest.len();
    let to = from
        + rest
            .find([',', '}'])
            .unwrap_or(rest.len())
            .min(rest.trim_end().len());
    Some((from, to))
}

#[test]
fn a_value_is_read_and_drawn_where_it_stands() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-reading-{}", std::process::id()));
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
        ports: 20_350..20_360,
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

    get(&mut cx, window, &format!("http://localhost:{port}/json"));
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Traffic, cx);
            view.select_place(0, cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the body laid out", |view, cx| {
        let BodyState::Shown(body) = &view.inspector.body else {
            return false;
        };
        body.read(cx).body().lines() > 3
    });

    // The `total` of the first order: a number, which is a size and a mask of
    // bits as well as a number.
    let (line, from, to) = window
        .update(&mut cx, |view, _, cx| {
            let BodyState::Shown(body) = &view.inspector.body else {
                unreachable!()
            };
            let body = body.read(cx);
            let body = body.body();
            let lines = body.read_whole(0..body.lines().min(400));
            lines
                .iter()
                .enumerate()
                .find_map(|(line, text)| {
                    let (from, to) = value_of(text, "total")?;
                    Some((line as u64, from, to))
                })
                .expect("the orders carry a total")
        })
        .unwrap();

    let readings = window
        .update(&mut cx, |view, _, cx| {
            let BodyState::Shown(body) = &view.inspector.body else {
                unreachable!()
            };
            body.update(cx, |body, cx| {
                body.selection().select(
                    Spot {
                        piece: line,
                        at: from,
                    },
                    Spot {
                        piece: line,
                        at: to,
                    },
                );
                let readings = body.read_selection();
                if let Some(best) = readings.first() {
                    body.draw_reading(best.clone(), cx);
                }
                readings
            })
        })
        .unwrap();

    assert!(
        !readings.is_empty(),
        "a number read as nothing at all: {readings:?}"
    );
    assert!(
        readings.iter().any(|r| r.id == "size" || r.id == "radix"),
        "what a number is read as: {:?}",
        readings.iter().map(|r| r.id).collect::<Vec<_>>()
    );
    // The field's name is known, so the reading says so.
    assert!(
        readings.iter().any(|r| r.label.contains("size")
            || r.beside().contains("0x")
            || r.beside().contains(", ")),
        "a reading says something: {:?}",
        readings.iter().map(|r| r.beside()).collect::<Vec<_>>()
    );

    // Drawn beside the value, and the window draws with it there.
    let drawn = window
        .update(&mut cx, |view, _, cx| {
            let BodyState::Shown(body) = &view.inspector.body else {
                unreachable!()
            };
            body.read(cx).readings_drawn()
        })
        .unwrap();
    assert_eq!(drawn, 1);
    cx.run_until_parked();
    shot(&mut cx, window.into(), "83-reading-drawn");

    // And taken away again.
    window
        .update(&mut cx, |view, _, cx| {
            let BodyState::Shown(body) = &view.inspector.body else {
                unreachable!()
            };
            body.update(cx, |body, cx| body.clear_readings(cx));
        })
        .unwrap();
    cx.run_until_parked();
    let drawn = window
        .update(&mut cx, |view, _, cx| {
            let BodyState::Shown(body) = &view.inspector.body else {
                unreachable!()
            };
            body.read(cx).readings_drawn()
        })
        .unwrap();
    assert_eq!(drawn, 0);
}

/// A header's value read and drawn under it, where the field's own name is
/// what the converters are told.
#[test]
fn a_header_is_read_under_its_own_name() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir =
        std::env::temp_dir().join(format!("tracewing-reading-headers-{}", std::process::id()));
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
        ports: 20_360..20_370,
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

    get(&mut cx, window, &format!("http://localhost:{port}/json"));
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Traffic, cx);
            view.select_place(0, cx);
            view.set_part(Part::Headers, cx);
        })
        .unwrap();
    cx.run_until_parked();

    // The headers are drawn as a name and then its value, so the first value
    // is the second piece. `date` carries an HTTP-date, which is certain.
    let (piece, len, field) = window
        .update(&mut cx, |view, _, _| {
            let sel = &view.inspector.select;
            (0..12u64).find_map(|piece| {
                let field = sel.field_of(piece)?;
                let text = sel.text_of(piece)?;
                (field.eq_ignore_ascii_case("date")).then_some((piece, text.len(), field))
            })
        })
        .unwrap()
        .expect("the answer carries a Date header");
    assert_eq!(field.to_lowercase(), "date");

    let readings = window
        .update(&mut cx, |view, _, cx| {
            view.inspector
                .select
                .select(Spot { piece, at: 0 }, Spot { piece, at: len });
            view.inspector.read_here(&view.inspector.select.clone());
            let readings = view.inspector.readings.clone();
            if let Some(best) = readings.first() {
                view.inspector.draw_reading(best.clone());
            }
            cx.notify();
            readings
        })
        .unwrap();

    assert_eq!(
        readings.first().map(|r| r.id),
        Some("http-date"),
        "a Date header reads as an HTTP-date: {:?}",
        readings.iter().map(|r| r.id).collect::<Vec<_>>()
    );
    let drawn = window
        .update(&mut cx, |view, _, _| view.inspector.readings_drawn())
        .unwrap();
    assert_eq!(drawn, 1);
    cx.run_until_parked();
    shot(&mut cx, window.into(), "84-reading-under-a-header");

    window
        .update(&mut cx, |view, _, cx| {
            view.inspector.clear_readings();
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    let drawn = window
        .update(&mut cx, |view, _, _| view.inspector.readings_drawn())
        .unwrap();
    assert_eq!(drawn, 0);
}

/// A rule remembered for a field reads the next flow's value by itself, in
/// the headers and in a body, and is on the disk for the next run.
#[test]
fn what_is_remembered_reads_itself_next_time() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-reading-rules-{}", std::process::id()));
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
        ports: 20_370..20_380,
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

    // Two flows from the same host: a rule for the first reads the second.
    get(&mut cx, window, &format!("http://localhost:{port}/json"));
    get(&mut cx, window, &format!("http://localhost:{port}/json"));
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Traffic, cx);
            view.select_place(0, cx);
            view.set_part(Part::Headers, cx);
        })
        .unwrap();
    cx.run_until_parked();

    // Nothing is read until a rule says so.
    let date = window
        .update(&mut cx, |view, _, _| {
            let sel = &view.inspector.select;
            (0..12u64).find_map(|piece| {
                let field = sel.field_of(piece)?;
                field
                    .eq_ignore_ascii_case("date")
                    .then(|| sel.text_of(piece))?
            })
        })
        .unwrap()
        .expect("the answer carries a Date header");
    let before = window
        .update(&mut cx, |view, _, _| {
            view.inspector.reading_under("date", &date).is_some()
        })
        .unwrap();
    assert!(!before, "nothing is read before a rule says to");
    // It is underlined, though: that is how a reader learns there is
    // something here to read at all.
    let marked = window
        .update(&mut cx, |view, _, _| {
            view.inspector.marks_of("date", &date, false).len()
        })
        .unwrap();
    assert_eq!(marked, 1, "a Date header says it can be read");
    let plain = window
        .update(&mut cx, |view, _, _| {
            view.inspector
                .marks_of("content-type", "application/json", false)
                .len()
        })
        .unwrap();
    assert_eq!(plain, 0, "and a content type does not");
    cx.run_until_parked();
    shot(&mut cx, window.into(), "87-reading-underlined");

    // Remember it, as the menu's `Always read date this way` does.
    window
        .update(&mut cx, |view, _, cx| {
            let host = view.inspector.host();
            view.inspector
                .remembered
                .borrow_mut()
                .remember(&host, "date", "http-date")
                .unwrap();
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();

    // The other flow's own Date, which was never chosen, reads itself.
    window
        .update(&mut cx, |view, _, cx| view.select_place(1, cx))
        .unwrap();
    cx.run_until_parked();
    let (other_date, reading) = window
        .update(&mut cx, |view, _, _| {
            let sel = &view.inspector.select;
            let date = (0..12u64)
                .find_map(|piece| {
                    let field = sel.field_of(piece)?;
                    field
                        .eq_ignore_ascii_case("date")
                        .then(|| sel.text_of(piece))?
                })
                .expect("the second answer carries a Date header too");
            let reading = view.inspector.reading_under("date", &date);
            (date, reading)
        })
        .unwrap();
    let reading = reading.expect("a remembered field reads itself");
    assert_eq!(reading.id, "http-date");
    assert_eq!(
        window
            .update(&mut cx, |view, _, _| view.inspector.readings_drawn())
            .unwrap(),
        0,
        "it is the rule that read it, not a choice"
    );
    cx.run_until_parked();
    shot(&mut cx, window.into(), "85-reading-remembered");

    // A body's field is read by a rule too.
    window
        .update(&mut cx, |view, _, cx| view.set_part(Part::Body, cx))
        .unwrap();
    wait_for(&mut cx, window, "the body laid out", |view, cx| {
        let BodyState::Shown(body) = &view.inspector.body else {
            return false;
        };
        body.read(cx).body().lines() > 3
    });
    let (line, text) = window
        .update(&mut cx, |view, _, cx| {
            let BodyState::Shown(body) = &view.inspector.body else {
                unreachable!()
            };
            let body = body.read(cx).body();
            let lines = body.read_whole(0..body.lines().min(400));
            lines
                .iter()
                .enumerate()
                .find_map(|(line, text)| {
                    text.contains("\"total\":")
                        .then(|| (line as u64, text.clone()))
                })
                .expect("the orders carry a total")
        })
        .unwrap();
    let before = window
        .update(&mut cx, |view, _, cx| {
            let BodyState::Shown(body) = &view.inspector.body else {
                unreachable!()
            };
            body.read(cx).reading_at(line, &text).is_some()
        })
        .unwrap();
    assert!(!before, "nothing is read in a body before a rule says to");
    window
        .update(&mut cx, |view, _, cx| {
            let host = view.inspector.host();
            view.inspector
                .remembered
                .borrow_mut()
                .remember(&host, "total", "size")
                .unwrap();
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    let after = window
        .update(&mut cx, |view, _, cx| {
            let BodyState::Shown(body) = &view.inspector.body else {
                unreachable!()
            };
            body.read(cx).reading_at(line, &text)
        })
        .unwrap()
        .expect("a remembered field is read in a body as well");
    assert_eq!(after.id, "size");
    cx.run_until_parked();
    shot(&mut cx, window.into(), "86-reading-remembered-in-a-body");

    // And it is on the disk for the next run.
    let kept = std::fs::read_to_string(dir.join("readings.toml")).unwrap();
    assert!(kept.contains("read_as = \"http-date\""), "{kept}");
    assert!(kept.contains("field = \"date\""), "{kept}");
    assert!(
        !other_date.is_empty() && kept.contains("read_as = \"size\""),
        "{kept}"
    );
}

/// A reading that holds a body — JSON out of base64 — opens as one, with
/// Pretty, Raw and Hex and everything else a body has.
#[test]
fn a_reading_that_holds_a_body_opens_as_one() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-reading-opened-{}", std::process::id()));
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
        ports: 20_380..20_390,
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

    // A query field holding base64 of `{"a":1,"b":2}`.
    get(
        &mut cx,
        window,
        &format!("http://localhost:{port}/json?blob=eyJhIjoxLCJiIjoyfQ"),
    );
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Traffic, cx);
            view.select_place(0, cx);
            view.set_part(Part::Query, cx);
        })
        .unwrap();
    cx.run_until_parked();

    let piece = window
        .update(&mut cx, |view, _, _| {
            let sel = &view.inspector.select;
            (0..12u64).find(|piece| sel.field_of(*piece).is_some_and(|field| field == "blob"))
        })
        .unwrap()
        .expect("the query carries the blob");

    let opened = window
        .update(&mut cx, |view, _, cx| {
            let sel = view.inspector.select.clone();
            let len = sel.text_of(piece).map(|text| text.len()).unwrap_or(0);
            sel.select(Spot { piece, at: 0 }, Spot { piece, at: len });
            view.inspector.read_here(&sel);
            // Two readings of the same value: the bytes base64 held, and
            // the JSON inside them. The chain is the one to open.
            let plain = view
                .inspector
                .readings
                .iter()
                .find(|reading| reading.id == "base64")
                .expect("base64 is read");
            assert_eq!(plain.chain, ["base64"]);
            let reading = view
                .inspector
                .readings
                .iter()
                .find(|reading| reading.id == "json")
                .cloned()
                .expect("the JSON inside the base64 is read too");
            assert_eq!(reading.chain, ["base64", "json"]);
            view.open_reading(&reading, "blob", cx)
        })
        .unwrap();
    assert!(opened, "a reading holding a body opens");
    cx.run_until_parked();

    // It is a body like any other: laid out, with lines of its own.
    let (part, label, lines) = window
        .update(&mut cx, |view, _, cx| {
            let opened = view.inspector.opened.as_ref().expect("it stays open");
            (
                view.inspector.part,
                opened.label.to_string(),
                opened.view.read(cx).body().lines(),
            )
        })
        .unwrap();
    assert_eq!(part, Part::Body, "it is shown where bodies are shown");
    assert_eq!(label, "base64 → json", "it says what read it");
    assert!(
        lines >= 4,
        "{{\"a\":1,\"b\":2}} laid out is four lines: {lines}"
    );
    cx.run_until_parked();
    shot(&mut cx, window.into(), "88-reading-opened");

    // The switch above the pane is about what is in it: Hex shows the bytes
    // the reading held, not the bytes of the body behind it.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_body_mode(tracewing::bodyview::BodyMode::Hex, cx)
        })
        .unwrap();
    cx.run_until_parked();
    let (opened_mode, body_mode) = window
        .update(&mut cx, |view, _, cx| {
            let opened = view.inspector.opened.as_ref().unwrap();
            let BodyState::Shown(body) = &view.inspector.body else {
                unreachable!()
            };
            (opened.view.read(cx).mode(), body.read(cx).mode())
        })
        .unwrap();
    assert_eq!(opened_mode, tracewing::bodyview::BodyMode::Hex);
    assert_ne!(
        body_mode,
        tracewing::bodyview::BodyMode::Hex,
        "the body behind it is left as it was"
    );
    window
        .update(&mut cx, |view, _, cx| {
            view.set_body_mode(tracewing::bodyview::BodyMode::Pretty, cx)
        })
        .unwrap();

    // And the way back leaves the body itself.
    window
        .update(&mut cx, |view, _, cx| {
            assert!(view.close_opened(cx));
            assert!(view.inspector.opened.is_none());
        })
        .unwrap();
    cx.run_until_parked();
}
