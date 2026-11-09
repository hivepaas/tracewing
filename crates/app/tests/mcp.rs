//! The MCP server, as an AI tool sees it: sessions, flows found with the box's
//! filter, one flow whole, and tokens and cookies left out until Settings let
//! them in. Then the same through the program's own stdin and stdout.
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
