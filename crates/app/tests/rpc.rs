//! JSON-RPC calls read in the inspector: a signed transaction out of its RLP,
//! a reverted call that answered 200, and a contract named by an ABI the user
//! chose.
//!
//! `cargo test --release -p tracewing --features headless --test rpc`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use hudsucker::futures::{SinkExt as _, StreamExt as _};
use hudsucker::tokio_tungstenite::tungstenite::Message;
use std::{
    path::PathBuf,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    inspector::Part,
    rpc_view::Shown,
    traffic::{Screen, Traffic},
};
use tracewing_dev::benchkit;
use tracewing_store::{settings::StorageSettings, web3};

/// The same sample the store's tests read: a real transaction, captured from
/// Ethereum mainnet. See `crates/store/testdata/jsonrpc/README.md`.
const SEND_RAW: &str = include_str!("../../store/testdata/jsonrpc/send_raw.req.json");

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

/// The lines of the part, as label and value, for reading an expectation.
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

fn titles(shown: &Shown) -> Vec<&str> {
    shown
        .sections
        .iter()
        .map(|section| section.title.as_str())
        .collect()
}

#[test]
fn json_rpc_calls_are_read_in_the_inspector() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-rpc-{}", std::process::id()));
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
    let ws_ca = dir.join("ws-ca.pem");
    let ws_origin = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let ws_port = ws_origin.local_addr().unwrap().port();
    let ca_out = ws_ca.clone();
    runtime.spawn(async move { benchkit::serve_ws_origin(ws_origin, &ca_out).await });
    while !ws_ca.exists() {
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
        ports: 20_270..20_280,
        extra_roots: vec![origin_ca, ws_ca],
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

    // A call to a contract of the user's own, which only an ABI can name.
    let mut calldata = web3::selector_of("redeem(uint256,address)").to_vec();
    calldata.extend(web3::bytes_of(&format!("0x{:064x}", 5_000_000u64)).unwrap());
    calldata.extend(
        web3::bytes_of("0x000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045")
            .unwrap(),
    );
    let eth_call = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"eth_call","params":[{{"to":"0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48","data":"{}"}},"latest"]}}"#,
        web3::hex_text(&calldata)
    );

    send("/rpc", SEND_RAW.trim());
    send("/rpc?fail=1", &eth_call);
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "two flows", |view| view.count() == 2);

    // The transaction, out of 884 bytes of RLP that the Body part can only
    // show as hex.
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            view.set_part(Part::Rpc, cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the transaction read", |view| {
        view.inspector.rpc.is_some()
    });
    window
        .update(&mut cx, |view, _, _| {
            let shown = view.inspector.rpc.as_ref().unwrap();
            assert_eq!(titles(shown)[0], "eth_sendRawTransaction");
            let lines = lines(shown);
            for said in [
                ("type", "2 (EIP-1559)"),
                ("chainId", "1 (Ethereum)"),
                ("nonce", "204580"),
                ("value", "0 ETH"),
                ("gas", "462389"),
                ("maxFeePerGas", "0.346815537 gwei"),
                ("selector", "0x09c5eabe"),
            ] {
                assert!(lines.contains(&said), "{said:?} in {lines:?}");
            }
        })
        .unwrap();
    shot(&mut cx, window.into(), "92-rpc-transaction");

    // The call that reverted: the status says 200, so the part has to say the
    // rest.
    window
        .update(&mut cx, |view, _, cx| view.select_place(1, cx))
        .unwrap();
    wait_for(&mut cx, window, "the reverted call read", |view| {
        view.inspector
            .rpc
            .as_ref()
            .is_some_and(|shown| shown.sections.len() > 1)
    });
    window
        .update(&mut cx, |view, _, _| {
            let detail = view.inspector.detail.as_ref().unwrap();
            assert_eq!(detail.status, Some(200), "a failed call answers 200");
            assert_eq!(detail.rpc.as_deref(), Some("eth_call"));
            assert_eq!(detail.rpc_error, Some(3));
            let shown = view.inspector.rpc.as_ref().unwrap();
            assert_eq!(titles(shown), ["eth_call", "Failed"]);
            let lines = lines(shown);
            // No ABI yet, so the four bytes stand for the function.
            let four = web3::hex_text(&web3::selector_of("redeem(uint256,address)"));
            assert!(
                lines.contains(&("selector", four.as_str())),
                "{four} in {lines:?}"
            );
            assert!(
                lines.contains(&("reverted", "ERC20: transfer amount exceeds balance")),
                "{lines:?}"
            );
        })
        .unwrap();
    shot(&mut cx, window.into(), "93-rpc-reverted");

    // The user's ABI, chosen in Settings: the same call now has a name.
    let abis = dir.join("abis");
    std::fs::create_dir_all(&abis).unwrap();
    std::fs::write(
        abis.join("vault.json"),
        r#"[{"type":"function","name":"redeem",
             "inputs":[{"name":"shares","type":"uint256"},{"name":"to","type":"address"}]}]"#,
    )
    .unwrap();
    window
        .update(&mut cx, |view, _, cx| {
            view.add_abis(vec![abis.clone()], cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the ABI read and the call named", |view| {
        view.inspector
            .rpc
            .as_ref()
            .is_some_and(|shown| lines(shown).contains(&("function", "redeem(uint256,address)")))
    });
    window
        .update(&mut cx, |view, _, _| {
            let shown = view.inspector.rpc.as_ref().unwrap();
            let lines = lines(shown);
            for said in [
                ("function", "redeem(uint256,address)"),
                ("uint256", "5000000"),
                ("address", "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
            ] {
                assert!(lines.contains(&said), "{said:?} in {lines:?}");
            }
        })
        .unwrap();
    shot(&mut cx, window.into(), "94-rpc-named-by-abi");

    // A socket: its calls are messages, not a body, so nothing above would
    // have found them.
    let proxy_ca = dir.join("ca.pem");
    let mut ws = runtime
        .block_on(benchkit::websocket_through_proxy(
            proxy,
            &proxy_ca,
            "localhost",
            ws_port,
            "/rpc",
        ))
        .unwrap();
    runtime
        .block_on(async {
            ws.send(Message::text(
                r#"{"jsonrpc":"2.0","id":1,"method":"eth_subscribe","params":["newHeads"]}"#,
            ))
            .await?;
            // The answer, then what the subscription sends of its own.
            ws.next().await.unwrap()?;
            ws.next().await.unwrap()
        })
        .unwrap();
    wait_for(&mut cx, window, "the socket's row", |view| {
        view.count() == 3
    });
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(2, cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the socket's messages read", |view| {
        view.inspector
            .rpc
            .as_ref()
            .is_some_and(|shown| shown.sections.len() >= 3)
    });
    window
        .update(&mut cx, |view, _, cx| {
            let detail = view.inspector.detail.as_ref().unwrap();
            assert_eq!(detail.kind, "websocket");
            assert_eq!(detail.rpc, None, "a socket is not one request's call");
            assert!(
                view.offers_rpc(detail),
                "and yet the part is there, because its messages hold calls"
            );
            view.set_part(Part::Rpc, cx);
        })
        .unwrap();
    cx.run_until_parked();
    window
        .update(&mut cx, |view, _, _| {
            let shown = view.inspector.rpc.as_ref().unwrap();
            assert_eq!(
                titles(shown)[..3],
                ["eth_subscribe", "Answer", "eth_subscription"]
            );
            let lines = lines(shown);
            for said in [
                ("kind", "newHeads"),
                ("subscription", "0xcd0c3e8af590364c09d0fa6a1210faf5"),
                ("block", "26,393,566"),
            ] {
                assert!(lines.contains(&said), "{said:?} in {lines:?}");
            }
            assert!(
                shown.sections[2]
                    .note
                    .as_deref()
                    .is_some_and(|note| note.contains("not asked for")),
                "what the server sent without being asked each time"
            );
        })
        .unwrap();
    shot(&mut cx, window.into(), "95-rpc-socket");
    runtime.block_on(async { ws.close(None).await }).ok();

    std::fs::remove_dir_all(&dir).ok();
}
