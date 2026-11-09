//! The smaller parts of the inspector, for flows through the real proxy: the
//! fields of a query and a form, a picture, a body saved to a file, and the
//! certificates of a server the proxy would not trust.
//!
//! `cargo test --release -p tracewing --features headless --test details`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, px, size};
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
    snippets::Language,
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

/// Sends a request from the compose screen and returns the status it got.
fn send(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    method: &str,
    url: &str,
    headers: &str,
    body: &str,
) -> u16 {
    window
        .update(cx, |view, _, cx| {
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            let fields = [
                (page.method.clone(), method),
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
    window
        .update(cx, |view, _, _| {
            let page = view.compose_page.as_ref().unwrap();
            assert_eq!(page.problem, None);
            page.answer.as_ref().unwrap().status
        })
        .unwrap()
}

fn origin(runtime: &tokio::runtime::Runtime, ca_out: PathBuf) -> u16 {
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let ca = ca_out.clone();
    runtime.spawn(async move { benchkit::serve_origin(listener, &ca).await });
    while !ca_out.exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    port
}

#[test]
fn fields_pictures_saved_bodies_and_refused_certificates() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-details-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let trusted_ca = dir.join("origin-ca.pem");
    let trusted = origin(&runtime, trusted_ca.clone());
    // Its CA is not given to the proxy, which therefore will not go on with it.
    let untrusted = origin(&runtime, dir.join("other-ca.pem"));
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_090..20_100,
        extra_roots: vec![trusted_ca],
        key_store: Default::default(),
        find_apps: true,
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

    let status = send(
        &mut cx,
        window,
        "POST",
        &format!("https://localhost:{trusted}/json?size=20&q=red+shoes&page=2"),
        "content-type: application/x-www-form-urlencoded",
        "user=an%20na&city=H%C3%A0+N%E1%BB%99i&remember=on",
    );
    assert_eq!(status, 200);
    let status = send(
        &mut cx,
        window,
        "GET",
        &format!("https://localhost:{trusted}/picture.png"),
        "",
        "",
    );
    assert_eq!(status, 200);
    let status = send(
        &mut cx,
        window,
        "GET",
        &format!("https://localhost:{untrusted}/json"),
        "",
        "",
    );
    assert_eq!(status, 502);

    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "three flows", |view| view.count() == 3);

    // The query and the form, each field decoded.
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            view.set_side(Side::Request, cx);
            let detail = view.inspector.detail.as_ref().unwrap();
            assert_eq!(detail.form.as_ref().map(Vec::len), Some(3));
            view.set_part(Part::Query, cx);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "50-query");
    window
        .update(&mut cx, |view, _, cx| view.set_part(Part::Form, cx))
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "51-form");
    // A response has no form: the part goes back to the body.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_side(Side::Response, cx);
            assert_eq!(view.inspector.part, Part::Body);
        })
        .unwrap();

    // A picture is drawn, and saved as it came.
    window
        .update(&mut cx, |view, _, cx| view.select_place(1, cx))
        .unwrap();
    wait_for(&mut cx, window, "the picture", |view| {
        matches!(view.inspector.body, BodyState::Image(..))
    });
    shot(&mut cx, window.into(), "52-picture");
    let saved = dir.join("saved.png");
    window
        .update(&mut cx, |view, _, cx| {
            view.save_body(Side::Response, saved.clone(), cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the body saved", |view| {
        !view.is_busy_with_files() && saved.exists()
    });
    assert_eq!(std::fs::read(&saved).unwrap(), benchkit::PICTURE);

    // The server it would not trust: why, and what it showed.
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(2, cx);
            let detail = view.inspector.detail.as_ref().unwrap();
            assert!(
                detail
                    .error
                    .as_deref()
                    .unwrap()
                    .contains("TLS with localhost")
            );
            let tls = detail.connection.as_ref().unwrap().tls.as_ref().unwrap();
            assert!(tls.error.as_deref().unwrap().contains("certificate"));
            assert!(tls.chain[0].names.iter().any(|n| n == "localhost"));
            view.set_part(Part::Connection, cx);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "53-refused");

    // HTML, JavaScript and CSS, each in its colors.
    for path in ["page.html", "app.js", "style.css"] {
        let status = send(
            &mut cx,
            window,
            "GET",
            &format!("https://localhost:{trusted}/{path}"),
            "",
            "",
        );
        assert_eq!(status, 200);
    }
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "six flows", |view| view.count() == 6);
    for (place, name) in [(3, "63-html"), (4, "64-script"), (5, "65-style")] {
        window
            .update(&mut cx, |view, _, cx| {
                view.select_place(place, cx);
                view.set_side(Side::Response, cx);
                view.set_part(Part::Body, cx);
            })
            .unwrap();
        wait_for(&mut cx, window, name, |view| {
            matches!(view.inspector.body, BodyState::Shown(..))
        });
        shot(&mut cx, window.into(), name);
    }

    // The form's POST, as code to paste.
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            view.set_copy_menu(true, cx);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "67-copy-as");
    window
        .update(&mut cx, |view, _, cx| {
            view.copy_as(Language::Python, cx);
            assert!(!view.inspector.copy_menu, "closed once chosen");
        })
        .unwrap();
    let copied = cx
        .update(|cx| cx.read_from_clipboard())
        .and_then(|item| item.text())
        .expect("copied");
    assert!(copied.starts_with("import requests\n"), "{copied}");
    assert!(copied.contains("requests.post("), "{copied}");
    assert!(copied.contains("city=H%C3%A0+N%E1%BB%99i"), "{copied}");

    std::fs::remove_dir_all(&dir).ok();
}
