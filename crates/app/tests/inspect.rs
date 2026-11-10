//! GraphQL operations in the list, cookies and tokens in the inspector, for
//! flows that went through the real proxy.
//!
//! `cargo test --release -p tracewing --features headless --test inspect`

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
    detail::Side,
    engine::{Engine, EngineOptions},
    filter::Kind,
    input::TextInput,
    inspector::Part,
    traffic::{Screen, Traffic},
};
use tracewing_dev::benchkit;
use tracewing_store::settings::StorageSettings;

/// RS256, for user 7, expired at the start of 2026.
const EXPIRED_JWT: &str = "eyJhbGciOiJSUzI1NiIsImtpZCI6ImsxIn0.eyJzdWIiOiI3Iiwic2NvcGUiOiJyZWFkIiwiZXhwIjoxNzY3MjI1NjAwfQ.c2lnbmF0dXJl";

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
    method: &str,
    url: &str,
    headers: &str,
    body: &str,
) {
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
            assert_eq!(page.answer.as_ref().unwrap().status, 200);
        })
        .unwrap();
}

#[test]
fn graphql_cookies_and_tokens_are_read() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-inspect-{}", std::process::id()));
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
        ports: 19_990..20_000,
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
    let origin = format!("https://localhost:{origin_port}");

    send(
        &mut cx,
        window,
        "GET",
        &format!("{origin}/json?size=40"),
        "",
        "",
    );
    send(
        &mut cx,
        window,
        "POST",
        &format!("{origin}/json?size=60&cookies=1"),
        &format!(
            "content-type: application/json\n\
             authorization: Bearer {EXPIRED_JWT}\n\
             cookie: sid=abc123; token={}",
            benchkit::SESSION_JWT
        ),
        r#"{"query":"query GetUser($id: ID!) { user(id: $id) { name } }","operationName":"GetUser","variables":{"id":"42"}}"#,
    );
    send(
        &mut cx,
        window,
        "GET",
        &format!("{origin}/json?size=30&query=mutation%20Like%20%7B%20like%20%7D"),
        "",
        "",
    );

    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "three flows", |view| view.count() == 3);
    shot(&mut cx, window.into(), "40-graphql-list");

    // The chip and the box both find them by their operation.
    window
        .update(&mut cx, |view, _, cx| view.set_kind(Kind::GraphQl, cx))
        .unwrap();
    wait_for(&mut cx, window, "GraphQL flows", |view| view.count() == 2);
    window
        .update(&mut cx, |view, _, cx| {
            view.set_kind(Kind::All, cx);
            view.set_filter_text("gql:GetUser", cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the GetUser flow", |view| {
        view.count() == 1
    });
    window
        .update(&mut cx, |view, _, cx| {
            view.set_filter_text("gql:mutation", cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the mutation", |view| view.count() == 1);
    window
        .update(&mut cx, |view, _, cx| {
            view.set_filter_text("gql:GetUser", cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "GetUser again", |view| view.count() == 1);

    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            let detail = view.inspector.detail.as_ref().unwrap();
            assert_eq!(detail.method, "POST");
            assert_eq!(detail.graphql.as_deref(), Some("query GetUser"));
            view.set_side(Side::Request, cx);
            view.set_part(Part::Headers, cx);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "41-jwt-in-headers");

    // A drag over the first header takes its name and value, as a line of HTTP.
    let (name_at, value_at, name, value) = window
        .update(&mut cx, |view, _, _| {
            let sel = &view.inspector.select;
            let detail = view.inspector.detail.as_ref().unwrap();
            let (name, value) = detail.headers(Side::Request).unwrap()[0].clone();
            (
                sel.drawn_at(0).unwrap(),
                sel.drawn_at(1).unwrap(),
                name,
                value,
            )
        })
        .unwrap();
    drag(
        &mut cx,
        window,
        gpui::point(name_at.left() + px(1.), name_at.center().y),
        gpui::point(value_at.right() + px(40.), value_at.top() + px(4.)),
    );
    key(&mut cx, window, "ctrl-c");
    assert_eq!(clipboard(&mut cx), format!("{name}: {value}"));
    shot(&mut cx, window.into(), "77-header-selected");
    // All of them, the token with them; the menu offers the same.
    key(&mut cx, window, "ctrl-a");
    key(&mut cx, window, "ctrl-c");
    let all = clipboard(&mut cx);
    assert!(all.contains(&format!(": Bearer {EXPIRED_JWT}")), "{all}");
    assert!(all.contains(&format!("{name}: {value}\n")), "{all}");
    right_click(&mut cx, window, value_at.center());
    assert!(
        window
            .update(&mut cx, |view, _, _| view.inspector.select.menu().is_some())
            .unwrap()
    );
    shot(&mut cx, window.into(), "78-copy-menu");
    key(&mut cx, window, "escape");
    assert!(
        window
            .update(&mut cx, |view, _, _| view.inspector.select.menu().is_none())
            .unwrap()
    );

    window
        .update(&mut cx, |view, _, cx| view.set_part(Part::Cookies, cx))
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "42-cookies-sent");

    window
        .update(&mut cx, |view, _, cx| view.set_side(Side::Response, cx))
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "43-cookies-set");

    // A flow with no cookies does not stay on a part it does not have.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_filter_text("", cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "all flows", |view| view.count() == 3);
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            assert_eq!(view.inspector.detail.as_ref().unwrap().method, "GET");
            assert_eq!(view.inspector.part, Part::Body);
        })
        .unwrap();

    // The first flow made the connection to the origin, and paid for each step.
    window
        .update(&mut cx, |view, _, cx| {
            let detail = view.inspector.detail.as_ref().unwrap();
            let labels: Vec<_> = detail.phases().iter().map(|p| p.label).collect();
            assert!(
                labels.ends_with(&["DNS", "Connect", "TLS", "Waiting", "Download"]),
                "{labels:?}"
            );
            let connection = detail.connection.as_ref().expect("its facts are kept");
            assert!(connection.remote.contains(&origin_port.to_string()));
            let tls = connection.tls.as_ref().expect("over TLS");
            assert_eq!(tls.alpn.as_deref(), Some("h2"));
            assert!(tls.version.starts_with("TLS 1."), "{}", tls.version);
            assert!(!tls.chain.is_empty());
            assert!(tls.chain[0].names.iter().any(|n| n == "localhost"));
            view.set_part(Part::Connection, cx);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "44-connection");

    // The next went over the same connection: nothing to connect, the same facts.
    let first_conn = window
        .update(&mut cx, |view, _, _| {
            view.inspector.detail.as_ref().unwrap().conn_id
        })
        .unwrap();
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(1, cx);
            let detail = view.inspector.detail.as_ref().unwrap();
            assert_eq!(detail.conn_id, first_conn);
            assert_eq!(detail.connect_us, None);
            let labels: Vec<_> = detail.phases().iter().map(|p| p.label).collect();
            assert_eq!(labels, ["Waiting", "Download"]);
            assert!(detail.connection.is_some(), "found with the first flow");
            assert_eq!(view.inspector.part, Part::Connection);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "45-connection-reused");

    // The list goes out as a HAR, without secrets when asked...
    let har_path = dir.join("out.har");
    window
        .update(&mut cx, |view, _, cx| {
            view.set_har_without_secrets(true, cx);
            let live = view.session_dir().to_owned();
            view.export_har(live, None, har_path.clone(), cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the HAR written", |view| {
        !view.is_busy_with_files()
    });
    let har: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&har_path).unwrap()).unwrap();
    let entries = har["log"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    let posted = &entries[1];
    assert_eq!(posted["request"]["method"], "POST");
    assert!(
        posted["request"]["postData"]["text"]
            .as_str()
            .unwrap()
            .contains("GetUser")
    );
    assert_eq!(posted["response"]["cookies"], serde_json::json!([]));
    let text = har.to_string();
    assert!(!text.contains(EXPIRED_JWT), "the bearer token went out");
    assert!(!text.contains("sid=abc123"), "a cookie went out");
    // The first flow made the connection: its steps, and the server's address.
    assert!(entries[0]["timings"]["ssl"].as_f64().unwrap() > 0.0);
    assert_eq!(entries[0]["serverIPAddress"], "127.0.0.1");
    assert_eq!(entries[1]["timings"]["connect"], -1.0);

    // ...and comes back in as a session of its own, named after the file.
    window
        .update(&mut cx, |view, _, cx| {
            view.import_file(har_path.clone(), cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the HAR read", |view| {
        !view.is_busy_with_files() && view.is_viewing() && view.count() == 3
    });
    window
        .update(&mut cx, |view, _, cx| {
            assert_ne!(view.session_dir(), view.engine.session);
            assert_eq!(
                tracewing_store::session::name_of(view.session_dir()).as_deref(),
                Some("out")
            );
            view.select_place(1, cx);
            let detail = view.inspector.detail.as_ref().unwrap();
            assert_eq!(detail.graphql.as_deref(), Some("query GetUser"));
            assert_eq!(detail.status, Some(200));
            view.set_part(Part::Body, cx);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "46-har-imported");
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Sessions, cx))
        .unwrap();
    wait_for(&mut cx, window, "the sessions", |view| {
        view.sessions_page.sessions.len() == 2
    });
    shot(&mut cx, window.into(), "47-sessions-har");

    std::fs::remove_dir_all(&dir).ok();
}
