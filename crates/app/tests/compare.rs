//! Two flows through the real proxy, compared: their query, headers and
//! bodies matched, and the same lines of a body folded.
//!
//! `cargo test --release -p tracewing --features headless --test compare`

#![cfg(all(feature = "headless", target_os = "linux"))]

mod common;

use common::key;

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    compare::{Change, Row},
    compare_page::{BodyLoad, ComparePart},
    detail::Side,
    engine::{Engine, EngineOptions},
    input::TextInput,
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

fn send(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    url: &str,
    headers: &str,
    body: &str,
) {
    window
        .update(cx, |view, _, cx| {
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            let fields = [
                (page.method.clone(), "POST"),
                (page.url.clone(), url),
                (page.headers.clone(), headers),
                (page.body.clone(), body),
            ];
            for (input, text) in fields {
                type_in(&input, text, cx);
            }
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(cx, window, "the answer", |view| {
        view.compose_page
            .as_ref()
            .is_some_and(|page| page.answer.is_some() || page.problem.is_some())
    });
}

/// The same in both requests: lines enough to fold.
const ITEMS: &str = r#"["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"]"#;

#[test]
fn two_flows_are_compared() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-compare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let ca = dir.join("origin-ca.pem");
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let ca_out = ca.clone();
    runtime.spawn(async move { benchkit::serve_origin(listener, &ca_out).await });
    while !ca.exists() {
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
        ports: 20_100..20_110,
        extra_roots: vec![ca],
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

    // Each answer is a different JSON order of about 2 KB.
    send(
        &mut cx,
        window,
        &format!("https://localhost:{port}/json?rich=1&page=1&lang=vi"),
        "content-type: application/json\nx-trace: one",
        &format!(r#"{{"user": 42, "items": {ITEMS}, "note": "first"}}"#),
    );
    send(
        &mut cx,
        window,
        &format!("https://localhost:{port}/json?rich=1&page=2"),
        "content-type: application/json\nx-retry: 1",
        &format!(r#"{{"items":{ITEMS},"note":"second","user":42}}"#),
    );
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "two flows", |view| view.count() == 2);

    // Ctrl and a click on the other row.
    let (first, second) = window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            let first = view.inspector.detail.as_ref().unwrap().id;
            view.select_place(1, cx);
            let second = view.inspector.detail.as_ref().unwrap().id;
            view.select_place(0, cx);
            assert!(!view.compare_on_click(first, true, cx), "not with itself");
            assert!(
                !view.compare_on_click(second, false, cx),
                "a plain click selects"
            );
            view.select_place(0, cx);
            assert!(view.compare_on_click(second, true, cx));
            assert_eq!(view.screen, Screen::Compare);
            (first, second)
        })
        .unwrap();
    wait_for(&mut cx, window, "the bodies compared", |view| {
        let page = view.compare_page.as_ref().unwrap();
        matches!(page.body(Side::Response), BodyLoad::Ready(_))
    });
    window
        .update(&mut cx, |view, _, _| {
            let page = view.compare_page.as_ref().unwrap();
            assert_eq!((page.a.id, page.b.id), (first, second));

            // The query: page changed, lang only in the first.
            let query: Vec<(String, Change)> = page
                .query()
                .iter()
                .map(|p| (p.name.clone(), p.change()))
                .collect();
            assert_eq!(
                query,
                [
                    ("rich".to_owned(), Change::Same),
                    ("page".to_owned(), Change::Changed),
                    ("lang".to_owned(), Change::Removed),
                ]
            );
            let headers = page.headers(Side::Request);
            let change_of = |name: &str| {
                headers
                    .iter()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                    .map(|p| p.change())
            };
            assert_eq!(change_of("x-trace"), Some(Change::Removed));
            assert_eq!(change_of("x-retry"), Some(Change::Added));
            assert_eq!(change_of("content-type"), Some(Change::Same));

            // The requests' JSON differs only in the note, whatever its order.
            let BodyLoad::Ready(request) = page.body(Side::Request) else {
                panic!("the request's bodies");
            };
            assert_eq!(request.a.kind, "JSON");
            assert_eq!(request.changed, 1);
            assert_eq!(page.differences(Side::Request, ComparePart::Body), Some(1));
            let BodyLoad::Ready(response) = page.body(Side::Response) else {
                panic!("the responses' bodies");
            };
            assert!(response.changed > 0);
        })
        .unwrap();
    shot(&mut cx, window.into(), "77-compare");

    // A fold of the same lines opens.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_compare_side(Side::Request, cx);
            let page = view.compare_page.as_ref().unwrap();
            let rows = page.rows(Side::Request).len();
            let fold = page.rows(Side::Request).iter().find_map(|row| match row {
                Row::Folded { at, count } => Some((*at, *count)),
                _ => None,
            });
            let (at, count) = fold.expect("the same lines folded");
            view.open_compare_fold(at, cx);
            let page = view.compare_page.as_ref().unwrap();
            assert_eq!(page.rows(Side::Request).len(), rows - 1 + count);

            view.set_compare_part(ComparePart::Query, cx);
            view.swap_compared(cx);
            let page = view.compare_page.as_ref().unwrap();
            assert_eq!((page.a.id, page.b.id), (second, first));
            assert_eq!(page.part, ComparePart::Query, "what was looked at stays");
        })
        .unwrap();
    shot(&mut cx, window.into(), "78-compare-query");

    // "Compare with…", then a click on the other.
    key(&mut cx, window, "escape");
    window
        .update(&mut cx, |view, _, cx| {
            assert_eq!(view.screen, Screen::Traffic);
            view.select_place(0, cx);
            view.start_compare(cx);
            assert_eq!(view.compare_pick, Some(first));
        })
        .unwrap();
    shot(&mut cx, window.into(), "79-compare-pick");
    key(&mut cx, window, "escape");
    window
        .update(&mut cx, |view, _, cx| {
            assert_eq!(view.compare_pick, None, "Esc lets it go");
            view.start_compare(cx);
            assert!(view.compare_on_click(second, false, cx));
            let page = view.compare_page.as_ref().unwrap();
            assert_eq!((page.a.id, page.b.id), (first, second));
            assert_eq!(view.compare_pick, None);
        })
        .unwrap();

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
