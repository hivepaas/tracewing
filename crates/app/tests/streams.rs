//! A WebSocket and server-sent events in the window, live, with no display.
//!
//! `cargo test --release -p tracewing --features headless --test streams`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use hudsucker::{
    futures::{SinkExt, StreamExt},
    tokio_tungstenite::tungstenite::Message,
};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    inspector::Part,
    traffic::Traffic,
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

fn messages(view: &Traffic) -> usize {
    view.inspector.messages.as_ref().map_or(0, |m| m.list.len())
}

#[test]
fn streams_show_as_they_come() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-streams-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let (origin_ca, ws_ca) = (dir.join("origin-ca.pem"), dir.join("ws-ca.pem"));
    let origin = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let origin_port = origin.local_addr().unwrap().port();
    let ca_out = origin_ca.clone();
    runtime.spawn(async move { benchkit::serve_origin(origin, &ca_out).await });
    let ws_origin = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let ws_port = ws_origin.local_addr().unwrap().port();
    let ca_out = ws_ca.clone();
    runtime.spawn(async move { benchkit::serve_ws_origin(ws_origin, &ca_out).await });
    while !origin_ca.exists() || !ws_ca.exists() {
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
        ports: 19_590..19_600,
        extra_roots: vec![origin_ca, ws_ca],
    })
    .unwrap();
    let addr = engine.addr;
    let proxy_ca = dir.join("ca.pem");
    while !proxy_ca.exists() {
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
    cx.run_until_parked();

    // A WebSocket: listed from its upgrade, and not as an error.
    let mut ws = runtime
        .block_on(benchkit::websocket_through_proxy(
            addr,
            &proxy_ca,
            "localhost",
            ws_port,
            "/chat?room=1",
        ))
        .unwrap();
    wait_for(&mut cx, window, "its row", |view| view.count() == 1);
    window
        .update(&mut cx, |view, _, cx| view.select_place(0, cx))
        .unwrap();
    window
        .update(&mut cx, |view, _, _| {
            let detail = view.inspector.detail.as_ref().expect("chosen");
            assert_eq!(detail.kind, "websocket");
            assert!(detail.open);
            assert_eq!(detail.status, Some(101));
            assert_eq!(view.inspector.part, Part::Messages, "what it is there for");
        })
        .unwrap();

    // Messages arrive in the open inspector with nothing else moving.
    runtime
        .block_on(async {
            ws.send(Message::text(r#"{"order":"đặt hàng","qty":2}"#))
                .await?;
            ws.next().await.unwrap()?;
            ws.send(Message::binary(vec![0u8, 1, 2, 3])).await?;
            ws.next().await.unwrap()
        })
        .unwrap();
    wait_for(&mut cx, window, "the first messages", |view| {
        messages(view) >= 4
    });
    runtime
        .block_on(async {
            for i in 0..60 {
                ws.send(Message::text(format!("message {i}"))).await?;
                ws.next().await.unwrap()?;
            }
            Ok::<_, hudsucker::tokio_tungstenite::tungstenite::Error>(())
        })
        .unwrap();
    wait_for(&mut cx, window, "more messages", |view| {
        messages(view) >= 124
    });
    shot(&mut cx, window.into(), "19-websocket-open");

    window
        .update(&mut cx, |view, _, cx| view.choose_message(0, cx))
        .unwrap();
    wait_for(&mut cx, window, "the message", |view| {
        view.inspector
            .messages
            .as_ref()
            .is_some_and(|m| m.view.is_some())
    });
    cx.run_until_parked();
    shot(&mut cx, window.into(), "20-websocket-message");

    // Closed: the row and the inspector say so.
    runtime
        .block_on(async {
            ws.close(None).await?;
            while ws.next().await.is_some() {}
            Ok::<_, hudsucker::tokio_tungstenite::tungstenite::Error>(())
        })
        .unwrap();
    wait_for(&mut cx, window, "it to close", |view| {
        view.inspector.detail.as_ref().is_some_and(|d| !d.open)
    });

    // Server-sent events: the same, from the response.
    let client =
        benchkit::client_through_proxy(&format!("http://{addr}"), &proxy_ca, false).unwrap();
    let url = format!("https://localhost:{origin_port}/stream?events=5&gap_ms=120");
    let events = runtime.spawn(async move {
        let bytes = client.get(url).send().await?.bytes().await?;
        Ok::<_, reqwest::Error>(bytes.len())
    });
    wait_for(&mut cx, window, "the events' row", |view| view.count() == 2);
    window
        .update(&mut cx, |view, _, cx| view.select_place(1, cx))
        .unwrap();
    let was_open = window
        .update(&mut cx, |view, _, _| {
            let detail = view.inspector.detail.as_ref().expect("chosen");
            assert_eq!(detail.kind, "sse");
            detail.open
        })
        .unwrap();
    assert!(was_open, "listed while it was still going");
    wait_for(&mut cx, window, "the events", |view| messages(view) == 5);
    assert!(runtime.block_on(events).unwrap().unwrap() > 0);
    wait_for(&mut cx, window, "them to end", |view| {
        view.inspector.detail.as_ref().is_some_and(|d| !d.open)
    });
    shot(&mut cx, window.into(), "21-events");

    drop(cx);
    runtime.shutdown_background();
    std::fs::remove_dir_all(&dir).unwrap();
}
