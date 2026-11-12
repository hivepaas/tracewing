//! Requests composed in the window, and flows sent again, through the real proxy.
//!
//! `cargo test --release -p tracewing --features headless --test compose`

#![cfg(all(feature = "headless", target_os = "linux"))]

mod common;

use common::{clipboard, drag, key, right_click};

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    input::TextInput,
    rpc_view::Shown,
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

/// The lines of the inspector's JSON-RPC part, as label and value.
fn lines(shown: &Shown) -> Vec<(&str, &str)> {
    shown
        .sections
        .iter()
        .flat_map(|section| {
            section
                .lines
                .iter()
                .map(|line| (line.label.as_str(), line.value.as_str()))
        })
        .collect()
}

fn answer(view: &Traffic) -> Option<(u16, usize)> {
    let page = view.compose_page.as_ref()?;
    page.answer.as_ref().map(|a| (a.status, a.body.len()))
}

#[test]
fn requests_composed_in_the_window_go_through() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-compose-{}", std::process::id()));
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
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 19_890..19_900,
        extra_roots: vec![origin_ca],
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
    let url = format!("https://localhost:{origin_port}/json?size=50");

    // What cannot be sent says why, and nothing goes.
    window
        .update(&mut cx, |view, _, cx| {
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            let url_field = page.url.clone();
            type_in(&url_field, "/no/host", cx);
            view.send_composed(cx);
            let page = view.compose_page.as_ref().unwrap();
            assert_eq!(page.problem.as_deref(), Some("\"/no/host\" needs a host"));
            assert!(!page.sending);
        })
        .unwrap();

    window
        .update(&mut cx, |view, _, cx| {
            let page = view.compose_page.as_ref().unwrap();
            let (method, url_field, headers, body) = (
                page.method.clone(),
                page.url.clone(),
                page.headers.clone(),
                page.body.clone(),
            );
            type_in(&method, "POST", cx);
            type_in(&url_field, &url, cx);
            type_in(
                &headers,
                "content-type: application/json\nx-from: compose",
                cx,
            );
            type_in(&body, r#"{"made":"here"}"#, cx);
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the answer", |view| answer(view).is_some());
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(answer(view), Some((200, 50)))
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "35-compose");

    // The answer's text is selected and copied like the inspector's...
    let (summary, line) = window
        .update(&mut cx, |view, _, _| {
            let page = view.compose_page.as_ref().unwrap();
            (
                tracewing::compose::summary(page.answer.as_ref().unwrap()),
                page.select.drawn_at(0).unwrap(),
            )
        })
        .unwrap();
    drag(
        &mut cx,
        window,
        gpui::point(line.left() + px(1.), line.center().y),
        gpui::point(line.right() + px(40.), line.center().y),
    );
    key(&mut cx, window, "ctrl-c");
    assert_eq!(clipboard(&mut cx), summary);
    key(&mut cx, window, "ctrl-a");
    key(&mut cx, window, "ctrl-c");
    let all = clipboard(&mut cx);
    assert!(all.starts_with(&format!("{summary}\n\n")), "{all}");
    assert!(all.contains("content-type: application/json"), "{all}");
    assert!(all.trim_end().ends_with('}'), "the body last: {all}");
    shot(&mut cx, window.into(), "80-compose-selected");
    // ...and a field has its own menu: cut, copy, paste, select all.
    let url_field = window
        .update(&mut cx, |view, _, _| {
            view.compose_page.as_ref().unwrap().url.clone()
        })
        .unwrap();
    let field = cx.read_entity(&url_field, |input, _| input.drawn_at().unwrap());
    right_click(&mut cx, window, field.center());
    assert!(cx.read_entity(&url_field, |input, _| input.menu_open()));
    shot(&mut cx, window.into(), "81-field-menu");
    key(&mut cx, window, "escape");
    assert!(!cx.read_entity(&url_field, |input, _| input.menu_open()));

    // It went through the proxy, so it is in Traffic; sent again from there.
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "the composed flow", |view| {
        view.count() == 1
    });
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            let detail = view.inspector.detail.as_ref().expect("chosen");
            assert_eq!(detail.method, "POST");
            view.send_flow_again(cx);
            assert_eq!(view.screen, Screen::Compose);
        })
        .unwrap();
    wait_for(&mut cx, window, "the second answer", |view| {
        answer(view).is_some()
    });
    window
        .update(&mut cx, |view, _, cx| {
            assert_eq!(answer(view), Some((200, 50)));
            view.set_screen(Screen::Traffic, cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "both flows", |view| view.count() == 2);

    // A call written out in the body is sent as the calldata it stands for,
    // and the inspector reads that calldata back as the same call.
    let rpc = format!("https://localhost:{origin_port}/rpc");
    let whose = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Compose, cx);
            let page = view.compose_page.as_ref().unwrap();
            let (method, url_field, headers, body) = (
                page.method.clone(),
                page.url.clone(),
                page.headers.clone(),
                page.body.clone(),
            );
            type_in(&method, "POST", cx);
            type_in(&url_field, &rpc, cx);
            type_in(&headers, "content-type: application/json", cx);
            type_in(
                &body,
                &format!(
                    r#"{{"jsonrpc":"2.0","id":9,"method":"eth_call","params":[{{"to":"{whose}","data":"balanceOf({whose})"}},"latest"]}}"#
                ),
                cx,
            );
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the call's answer", |view| {
        answer(view).is_some()
    });
    window
        .update(&mut cx, |view, _, _| {
            let page = view.compose_page.as_ref().unwrap();
            assert_eq!(page.calls, ["balanceOf(address)"], "worked out on sending");
            assert_eq!(page.problem, None);
            assert_eq!(
                page.left_out,
                Vec::<String>::new(),
                "nothing written that the client works out"
            );
        })
        .unwrap();
    shot(&mut cx, window.into(), "36-compose-calldata");

    // What went on the wire: the proxy recorded it, and the inspector reads
    // the calldata back as the call that was written.
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "the call's flow", |view| view.count() == 3);
    window
        .update(&mut cx, |view, _, cx| view.select_place(2, cx))
        .unwrap();
    wait_for(&mut cx, window, "the call read back", |view| {
        view.inspector
            .rpc
            .as_ref()
            .is_some_and(|shown| lines(shown).contains(&("function", "balanceOf(address)")))
    });
    window
        .update(&mut cx, |view, _, _| {
            let shown = view.inspector.rpc.as_ref().unwrap();
            assert!(
                lines(shown).contains(&("address", whose)),
                "the value as it was written: {:?}",
                lines(shown)
            );
        })
        .unwrap();

    // A send can be stopped, and then nothing of it reaches the page. The
    // test executor runs background work on this thread, so the stop has to
    // be asked for before the executor gets its turn; that a stop lands part
    // way through a real send is `compose::tests::a_send_stops_where_it_stands`.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Compose, cx);
            let page = view.compose_page.as_ref().unwrap();
            let (method, url_field, body) =
                (page.method.clone(), page.url.clone(), page.body.clone());
            type_in(&method, "GET", cx);
            // Twenty seconds of answer, a little at a time.
            type_in(
                &url_field,
                &format!("https://localhost:{origin_port}/stream?events=40&gap_ms=500"),
                cx,
            );
            type_in(&body, "", cx);
            view.send_composed(cx);
            assert!(view.compose_page.as_ref().unwrap().sending);

            view.stop_composed(cx);
            let page = view.compose_page.as_ref().unwrap();
            assert!(!page.sending, "the page is free at once");
            assert!(page.stopped, "and says the send was stopped");
            assert!(page.answer.is_none());
        })
        .unwrap();
    // The send that was let go of brings nothing back, however long it is given.
    let after = Instant::now();
    while after.elapsed() < Duration::from_secs(2) {
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(20));
    }
    window
        .update(&mut cx, |view, _, _| {
            let page = view.compose_page.as_ref().unwrap();
            assert_eq!(answer(view), None, "no late answer");
            assert_eq!(page.problem, None, "and it is not an error");
            assert!(page.stopped);
            assert!(!page.sending);
        })
        .unwrap();
    shot(&mut cx, window.into(), "36-compose-stopped");

    std::fs::remove_dir_all(&dir).ok();
}
