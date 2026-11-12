//! The MCP server, as an AI tool sees it: sessions, flows found with the box's
//! filter, one flow whole, and tokens and cookies left out until Settings let
//! them in. Then the same through the program's own stdin and stdout. And,
//! once Settings allow it, rules added and requests sent.
//!
//! `cargo test --release -p tracewing --features headless --test mcp`

#![cfg(all(feature = "headless", target_os = "linux"))]

use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    mcp::Server,
};
use tracewing_dev::benchkit;
use tracewing_store::settings::StorageSettings;

/// A tool's result, as the structured value it carries.
fn call(server: &Server, name: &str, arguments: Value) -> Value {
    let message = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                          "params": { "name": name, "arguments": arguments } });
    let reply = server.handle_line(&message.to_string()).remove(0);
    let result = &reply["result"];
    assert_eq!(result["isError"], false, "{reply}");
    result["structuredContent"].clone()
}

fn until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done() {
        assert!(Instant::now() < deadline, "waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn header<'a>(headers: &'a Value, name: &str) -> Option<&'a str> {
    headers.as_array()?.iter().find(|pair| {
        pair[0]
            .as_str()
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    })?[1]
        .as_str()
}

#[test]
fn ai_tools_read_the_traffic() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-mcp-{}", std::process::id()));
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
        ports: 20_240..20_250,
        extra_roots: vec![origin_ca],
        key_store: Default::default(),
        find_apps: true,
    })
    .unwrap();
    while !dir.join("ca.pem").exists() {
        std::thread::sleep(Duration::from_millis(5));
    }

    let proxy = engine.addr;
    let curl = |args: &[&str]| {
        let out = Command::new("curl")
            .args(["-sS", "-o", "/dev/null", "--proxy"])
            .arg(format!("http://{proxy}"))
            .arg("--cacert")
            .arg(dir.join("ca.pem"))
            .args(args)
            .output()
            .expect("curl");
        assert!(out.status.success(), "{out:?}");
    };
    curl(&[
        "-H",
        "authorization: Bearer secret-token",
        &format!("https://localhost:{origin}/json?size=5&cookies=1"),
    ]);
    curl(&[
        "-H",
        "content-type: application/json",
        "--data",
        r#"{"q":"find-me-please"}"#,
        &format!("https://localhost:{origin}/json?size=3"),
    ]);

    let server = Server::new(&dir);
    until("two flows", || {
        call(&server, "list_flows", json!({}))["flows"]
            .as_array()
            .is_some_and(|f| f.len() == 2)
    });
    let sessions = call(&server, "list_sessions", json!({}));
    assert_eq!(sessions["sessions"][0]["newest"], true);
    assert_eq!(sessions["sessions"][0]["flows"], 2);

    // The newest first, and the box's filter.
    let all = call(&server, "list_flows", json!({}));
    assert_eq!(all["flows"][0]["method"], "POST");
    assert_eq!(all["flows"][1]["method"], "GET");
    let posts = call(&server, "list_flows", json!({ "filter": "method:POST" }));
    assert_eq!(posts["flows"].as_array().unwrap().len(), 1);
    until("the body's words in the index", || {
        call(&server, "list_flows", json!({ "filter": "find-me-please" }))["flows"]
            .as_array()
            .is_some_and(|f| f.len() == 1)
    });
    let paged = call(&server, "list_flows", json!({ "limit": 1 }));
    assert_eq!(paged["more"], true);
    let older = call(
        &server,
        "list_flows",
        json!({ "limit": 1, "before": paged["next_before"] }),
    );
    assert_eq!(older["flows"][0]["method"], "GET");

    // One flow whole, its secrets left out.
    let get_id = all["flows"][1]["id"].clone();
    let flow = call(&server, "get_flow", json!({ "id": get_id }));
    assert_eq!(flow["status"], 200);
    assert_eq!(flow["secrets_removed"], true);
    assert_eq!(
        header(&flow["request"]["headers"], "authorization"),
        Some("(removed)")
    );
    assert_eq!(
        header(&flow["response"]["headers"], "set-cookie"),
        Some("(removed)")
    );
    assert!(flow["response"]["body"].as_str().unwrap().starts_with('{'));
    let post = call(&server, "get_flow", json!({ "id": all["flows"][0]["id"] }));
    assert_eq!(post["request"]["body"], r#"{"q":"find-me-please"}"#);

    // Settings let them in, from the next call.
    let settings = StorageSettings {
        ai_secrets: true,
        ..StorageSettings::default()
    };
    std::fs::write(dir.join("settings.toml"), settings.to_toml().unwrap()).unwrap();
    let flow = call(&server, "get_flow", json!({ "id": get_id }));
    assert_eq!(
        header(&flow["request"]["headers"], "authorization"),
        Some("Bearer secret-token")
    );

    // The program itself, as an AI tool starts it.
    let mut child = Command::new(env!("CARGO_BIN_EXE_tracewing"))
        .args(["mcp", "--data-dir"])
        .arg(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    for message in [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "protocolVersion": "2025-06-18", "capabilities": {},
                            "clientInfo": { "name": "test", "version": "1" } } }),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "name": "list_flows", "arguments": { "filter": "method:GET" } } }),
    ] {
        writeln!(stdin, "{message}").unwrap();
    }
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    let hello: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(hello["result"]["protocolVersion"], "2025-06-18");
    line.clear();
    stdout.read_line(&mut line).unwrap();
    let found: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(found["id"], 2, "the notification got no answer");
    assert_eq!(
        found["result"]["structuredContent"]["flows"][0]["method"],
        "GET"
    );
    drop(stdin);
    assert!(child.wait().unwrap().success());

    drop(engine);
    std::fs::remove_dir_all(&dir).ok();
}

/// compare_flows: only what differs between two calls, a body's JSON by the
/// paths of its fields, and the call before when no second is given.
#[test]
fn ai_tools_compare_two_flows() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-mcp-compare-{}", std::process::id()));
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
        ports: 20_290..20_300,
        extra_roots: Vec::new(),
        key_store: Default::default(),
        find_apps: false,
    })
    .unwrap();
    let proxy = engine.addr;
    let post = |query: &str, token: &str, body: &str| {
        let out = Command::new("curl")
            .args(["-sS", "-o", "/dev/null", "--noproxy", "", "--proxy"])
            .arg(format!("http://{proxy}"))
            .args(["-H", &format!("authorization: Bearer {token}")])
            .args(["-H", "content-type: application/json", "--data", body])
            .arg(format!("http://localhost:{port}/echo?{query}"))
            .output()
            .expect("curl");
        assert!(out.status.success(), "{out:?}");
    };
    post(
        "page=1&lang=vi",
        "one",
        r#"{"user":42,"items":[{"id":1,"p":1}],"note":"first"}"#,
    );
    post(
        "page=2",
        "two",
        r#"{"note":"second","items":[{"id":1,"p":2}],"user":42}"#,
    );
    let server = Server::new(&dir);
    until("two flows", || {
        call(&server, "list_flows", json!({}))["flows"]
            .as_array()
            .is_some_and(|f| f.len() == 2)
    });
    let flows = call(&server, "list_flows", json!({}));
    let (newest, oldest) = (
        flows["flows"][0]["id"].clone(),
        flows["flows"][1]["id"].clone(),
    );

    // No `b`: the call before, to the same method, host and path.
    let compared = call(&server, "compare_flows", json!({ "a": newest }));
    assert_eq!(compared["b_is_the_call_before"], true);
    assert_eq!(compared["b"], oldest);
    let fields: Vec<&str> = compared["differs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["field"].as_str().unwrap())
        .collect();
    assert!(fields.contains(&"url"), "{fields:?}");
    assert!(!fields.contains(&"method"), "the same method is not said");
    let query: Vec<(&str, &str)> = compared["query"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["change"].as_str().unwrap()))
        .collect();
    assert_eq!(query, [("page", "changed"), ("lang", "only_b")]);

    // A token's value stays out, though that it changed is said.
    let headers = compared["request"]["headers"].as_array().unwrap();
    let token = headers
        .iter()
        .find(|h| h["name"] == "authorization")
        .expect("the token changed");
    assert_eq!(
        (token["a"].as_str(), token["b"].as_str()),
        (Some("(removed)"), Some("(removed)"))
    );
    // In headers, as get_flow leaves it out; the echo's body says it, as
    // get_flow's would.
    assert!(
        !compared["request"]["headers"]
            .to_string()
            .contains("Bearer")
    );

    // The JSON by field, whatever its keys' order.
    let body = &compared["request"]["body"];
    assert_eq!(body["fields_differ"], 2);
    let paths: Vec<(&str, &str, &str)> = body["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["path"].as_str().unwrap(),
                f["a"].as_str().unwrap(),
                f["b"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        paths,
        [
            ("items[id: 1].p", "2", "1"),
            ("note", "\"second\"", "\"first\"")
        ]
    );
    assert_eq!(compared["response"]["body"]["same"], false);

    // The first has no call before it.
    let message = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                          "params": { "name": "compare_flows", "arguments": { "a": oldest } } });
    let reply = server.handle_line(&message.to_string()).remove(0);
    assert_eq!(reply["result"]["isError"], true, "{reply}");

    drop(engine);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A tool's result when it fails: what it says.
fn refused(server: &Server, name: &str, arguments: Value) -> String {
    let message = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                          "params": { "name": name, "arguments": arguments } });
    let reply = server.handle_line(&message.to_string()).remove(0);
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Rules added and taken away, and requests sent through the proxy, only once
/// Settings allow it; the window takes in a rule an AI tool added.
#[test]
fn ai_tools_add_rules_and_send_requests_when_allowed() {
    use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
    use std::sync::Arc;
    use tracewing::traffic::Traffic;
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-mcp-act-{}", std::process::id()));
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
        ports: 20_330..20_340,
        extra_roots: Vec::new(),
        key_store: Default::default(),
        find_apps: false,
    })
    .unwrap();
    while !dir.join("ca.pem").exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let proxy = engine.addr;
    let text = Arc::new(gpui_wgpu::CosmicTextSystem::new(tracewing::FONT));
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(tracewing::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(tracewing::load_fonts);
    let data_dir = dir.clone();
    let window: WindowHandle<Traffic> = cx
        .open_window(size(px(1200.), px(800.)), move |window, cx| {
            cx.new(|cx| Traffic::new(engine, &data_dir, window, cx))
        })
        .unwrap();
    let curl = |path: &str| -> u16 {
        let out = Command::new("curl")
            .args(["-sS", "-o", "/dev/null", "-w", "%{http_code}", "--proxy"])
            .arg(format!("http://{proxy}"))
            .arg(format!("http://localhost:{port}{path}"))
            .output()
            .expect("curl");
        String::from_utf8_lossy(&out.stdout).parse().unwrap()
    };
    assert_eq!(curl("/echo?first"), 200);

    // Not allowed yet: each says where to allow it.
    let server = Server::new(&dir);
    let block = json!({ "name": "teapot", "url": "localhost:*/echo*",
                        "action": { "action": "block", "status": 418 } });
    assert!(refused(&server, "add_rule", block.clone()).contains("Settings"));
    assert!(
        refused(
            &server,
            "send_request",
            json!({ "url": "http://localhost/" })
        )
        .contains("Settings")
    );
    // An unknown action is refused too, once allowed.
    let settings = StorageSettings {
        ai_actions: true,
        ..StorageSettings::default()
    };
    std::fs::write(dir.join("settings.toml"), settings.to_toml().unwrap()).unwrap();
    assert!(
        refused(
            &server,
            "add_rule",
            json!({ "name": "x", "url": "a/*", "action": { "action": "explode" } })
        )
        .contains("action")
    );

    let added = call(&server, "add_rule", block.clone());
    assert_eq!(added["added"]["name"], "AI: teapot");
    assert!(refused(&server, "add_rule", block).contains("already"));
    assert_eq!(
        call(&server, "list_rules", json!({}))["rules"][0]["action"]["status"],
        418
    );
    // The window takes it in, and the proxy follows it.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let has = window
            .update(&mut cx, |view, _, cx| {
                view.look(cx);
                view.traffic_rules().rules.len() == 1
            })
            .unwrap();
        if has {
            break;
        }
        assert!(Instant::now() < deadline, "the window reads the rule");
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(curl("/echo?second"), 418);

    // A recorded flow sent again, changed: the rule answers it too.
    until("the flows", || {
        call(&server, "list_flows", json!({}))["flows"]
            .as_array()
            .is_some_and(|f| f.len() == 2)
    });
    let first = call(&server, "list_flows", json!({ "filter": "first" }))["flows"][0]["id"].clone();
    let sent = call(
        &server,
        "send_request",
        json!({ "id": first, "method": "post", "headers": { "x-from": "ai" }, "body": "hello" }),
    );
    assert_eq!(sent["status"], 418);
    assert_eq!(sent["method"], "POST");

    // Taken away: through to the server, which says what it got.
    assert_eq!(
        call(&server, "remove_rule", json!({ "name": "AI: teapot" }))["removed"],
        "AI: teapot"
    );
    assert!(refused(&server, "remove_rule", json!({ "name": "AI: teapot" })).contains("no rule"));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let none = window
            .update(&mut cx, |view, _, cx| {
                view.look(cx);
                view.traffic_rules().rules.is_empty()
            })
            .unwrap();
        if none {
            break;
        }
        assert!(Instant::now() < deadline, "the window reads it is gone");
        std::thread::sleep(Duration::from_millis(50));
    }
    let sent = call(
        &server,
        "send_request",
        json!({ "id": first, "method": "POST", "headers": { "x-from": "ai" }, "body": "hello" }),
    );
    assert_eq!(sent["status"], 200);
    let echoed: Value = serde_json::from_str(sent["body"].as_str().unwrap()).unwrap();
    assert_eq!(echoed["method"], "POST");
    assert_eq!(echoed["headers"]["x-from"], "ai");
    assert_eq!(echoed["body"], "hello");

    drop(cx);
    std::fs::remove_dir_all(&dir).ok();
}
