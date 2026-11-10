//! Calls to LLM APIs, put back together in the inspector: a streamed answer
//! with its thinking and a tool call, and a whole one.
//!
//! `cargo test --release -p tracewing --features headless --test llm`

#![cfg(all(feature = "headless", target_os = "linux"))]

mod common;

use common::{clipboard, drag, key};

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    inspector::Part,
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

#[test]
fn calls_to_models_are_put_back_together() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-llm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let origin_ca = dir.join("origin-ca.pem");
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let origin = listener.local_addr().unwrap().port();
    let ca_out = origin_ca.clone();
    runtime.spawn(async move { benchkit::serve_origin(listener, &ca_out).await });
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
        ports: 20_250..20_260,
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
    let proxy = window
        .update(&mut cx, |view, _, _| view.proxy_addr())
        .unwrap();

    let send = |path: &str, body: &str| {
        let out = Command::new("curl")
            .args(["-sS", "-o", "/dev/null", "--proxy"])
            .arg(format!("http://{proxy}"))
            .arg("--cacert")
            .arg(dir.join("ca.pem"))
            .args(["-H", "content-type: application/json", "--data", body])
            .arg(format!("https://localhost:{origin}{path}"))
            .output()
            .expect("curl");
        assert!(out.status.success(), "{out:?}");
    };
    send(
        "/v1/messages",
        r#"{"model":"claude-sonnet-4-5","max_tokens":1024,"stream":true,
            "system":"You book restaurants in Hanoi.",
            "messages":[{"role":"user","content":"A table for two tonight, please."}],
            "tools":[{"name":"find_table","input_schema":{}},{"name":"book","input_schema":{}}]}"#,
    );
    send(
        "/v1/chat/completions",
        r#"{"model":"gpt-4.1-mini","messages":[{"role":"user","content":"Any tables at 7?"}]}"#,
    );
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "two flows", |view| view.count() == 2);

    // The stream: chosen, it opens on its LLM part once its bodies are read.
    window
        .update(&mut cx, |view, _, cx| view.select_place(0, cx))
        .unwrap();
    wait_for(&mut cx, window, "the call read", |view| {
        view.inspector.llm.is_some()
    });
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(view.inspector.part, Part::Llm);
            let (_, call) = view.inspector.llm.as_ref().unwrap();
            assert_eq!(call.model.as_deref(), Some("claude-sonnet-4-5"));
            assert_eq!(call.thinking, "The user wants a table for two tonight.");
            assert_eq!(call.text, "I will look for a table near the Old Quarter.");
            assert_eq!(call.tool_calls[0].name, "find_table");
            assert_eq!(
                call.tool_calls[0].arguments,
                "{\n  \"area\": \"Hoan Kiem\",\n  \"people\": 2\n}"
            );
            assert_eq!(call.messages[0].text, "You book restaurants in Hanoi.");
            assert_eq!(call.tools, ["find_table", "book"]);
            assert_eq!(
                (call.input_tokens, call.output_tokens),
                (Some(412), Some(58))
            );
        })
        .unwrap();
    shot(&mut cx, window.into(), "69-llm-stream");

    // The call reads as text when copied: the answer, the tool it calls, and
    // what was asked.
    let first = window
        .update(&mut cx, |view, _, _| {
            view.inspector.select.drawn_at(0).unwrap()
        })
        .unwrap();
    drag(
        &mut cx,
        window,
        gpui::point(first.left() + px(1.), first.center().y),
        gpui::point(first.right() + px(40.), first.center().y),
    );
    key(&mut cx, window, "ctrl-c");
    assert_eq!(clipboard(&mut cx), "ANSWER");
    key(&mut cx, window, "ctrl-a");
    key(&mut cx, window, "ctrl-c");
    let all = clipboard(&mut cx);
    for said in [
        "I will look for a table near the Old Quarter.",
        "Calls find_table",
        "\"area\": \"Hoan Kiem\"",
        "ASKED",
        "You book restaurants in Hanoi.",
    ] {
        assert!(all.contains(said), "{said} in {all}");
    }
    shot(&mut cx, window.into(), "82-llm-selected");

    // A whole answer, the next one along.
    window
        .update(&mut cx, |view, _, cx| view.select_place(1, cx))
        .unwrap();
    wait_for(&mut cx, window, "the second call read", |view| {
        view.inspector
            .llm
            .as_ref()
            .is_some_and(|(_, call)| call.text == "Two tables are free at 7pm.")
    });
    shot(&mut cx, window.into(), "70-llm-whole");

    std::fs::remove_dir_all(&dir).ok();
}
