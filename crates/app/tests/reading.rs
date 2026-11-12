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
    inspector::BodyState,
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
