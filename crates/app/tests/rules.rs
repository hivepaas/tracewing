//! Rules made in the window, changing real traffic through the real proxy.
//!
//! `cargo test --release -p tracewing --features headless --test rules`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    rules_page::RuleKind,
    traffic::{Screen, Traffic},
};
use tracewing_dev::benchkit;
use tracewing_proxy::{
    throttle::Conditions,
    traffic_rules::{Action, TrafficRules},
};
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
fn rules_made_in_the_window_change_traffic() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-rules-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mock = dir.join("mock.json");
    std::fs::write(&mock, r#"{"mocked":true}"#).unwrap();

    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        key_store: Default::default(),
        find_apps: true,
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 19_690..19_700,
        extra_roots: Vec::new(),
    })
    .unwrap();
    let proxy = format!("http://{}", engine.addr);
    let ca = dir.join("ca.pem");
    while !ca.exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let client = benchkit::client_through_proxy(&proxy, &ca, false).unwrap();
    let get = |url: &'static str| {
        let client = client.clone();
        runtime.block_on(async move {
            let res = client.get(url).send().await.unwrap();
            let status = res.status().as_u16();
            (status, res.text().await.unwrap())
        })
    };

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
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Rules, cx))
        .unwrap();
    cx.run_until_parked();

    // What cannot work is refused, and nothing is kept.
    window
        .update(&mut cx, |view, _, cx| {
            view.fill_rule("Hide ads", "ads.test", "nope", "", cx);
            view.save_rule(cx);
            assert!(view.traffic_rules().is_empty());
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "22-rules-refused");

    window
        .update(&mut cx, |view, _, cx| {
            view.fill_rule("Hide ads", "ads.test", "451", "", cx);
            view.save_rule(cx);
            assert_eq!(view.traffic_rules().rules.len(), 1);
            view.set_rule_kind(RuleKind::MapLocal, cx);
            let path = mock_path(view);
            view.fill_rule("Mock the API", "api.test/v1/*", &path, "", cx);
            view.save_rule(cx);
            assert_eq!(view.traffic_rules().rules.len(), 2);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "23-rules");

    assert_eq!(get("https://ads.test/banner").0, 451);
    assert_eq!(
        get("https://api.test/v1/me"),
        (200, r#"{"mocked":true}"#.to_owned())
    );

    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "the flows", |view| view.count() == 2);
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(1, cx);
            let detail = view.inspector.detail.as_ref().expect("chosen");
            assert_eq!(detail.rule.as_deref(), Some("Mock the API"));
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "24-rule-on-a-flow");

    // Off, then edited, then gone: each applies at once, and each is kept.
    window
        .update(&mut cx, |view, _, cx| view.toggle_rule(0, cx))
        .unwrap();
    let file = dir.join("traffic-rules.toml");
    let kept = TrafficRules::load(&file).unwrap();
    assert!(!kept.rules[0].enabled);
    assert_eq!(kept.rules.len(), 2);

    window
        .update(&mut cx, |view, _, cx| {
            view.edit_rule(1, cx);
            view.fill_rule("Mock the API", "api.test/v2/*", &mock_path(view), "", cx);
            view.save_rule(cx);
            let rules = view.traffic_rules();
            assert_eq!(rules.rules.len(), 2, "changed, not added");
            assert_eq!(rules.rules[1].url, "api.test/v2/*");
            view.delete_rule(0, cx);
            assert_eq!(view.traffic_rules().rules.len(), 1);
        })
        .unwrap();
    assert_eq!(TrafficRules::load(&file).unwrap().rules.len(), 1);

    // A rule begun from a flow starts from its URL, without the query.
    window
        .update(&mut cx, |view, _, cx| {
            view.start_rule_for("https://shop.test/cart?id=1", cx);
            assert_eq!(view.screen, Screen::Rules);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "25-rule-from-a-flow");

    // Out to a file and back: one is here already, the one deleted comes back.
    let set = dir.join("shared-rules.toml");
    window
        .update(&mut cx, |view, _, cx| {
            view.clear_rule_form(cx);
            view.set_rule_kind(RuleKind::Block, cx);
            view.fill_rule("Hide ads", "ads.test", "451", "", cx);
            view.save_rule(cx);
            assert_eq!(view.traffic_rules().rules.len(), 2);
            view.export_rules_to(&set, cx);
            view.delete_rule(1, cx);
            assert_eq!(view.traffic_rules().rules.len(), 1);
            view.import_rules_from(&set, cx);
            let rules = view.traffic_rules();
            assert_eq!(rules.rules.len(), 2);
            assert_eq!(rules.rules[1].name, "Hide ads");
        })
        .unwrap();
    assert!(
        std::fs::read_to_string(&set)
            .unwrap()
            .contains("format = \"tracewing-rules\"")
    );
    assert_eq!(TrafficRules::load(&file).unwrap().rules.len(), 2, "kept");
    cx.run_until_parked();
    shot(&mut cx, window.into(), "68-rules-imported");

    // A body rule: what to find on one line, what goes in its place on several,
    // kept as typed, and back in the form to be edited.
    window
        .update(&mut cx, |view, _, cx| {
            view.clear_rule_form(cx);
            view.set_rule_kind(RuleKind::ResponseBody, cx);
            view.fill_rule(
                "Rename",
                "api.test/*",
                "\"mocked\"",
                "\"changed\",\n\"by\": 1",
                cx,
            );
            view.save_rule(cx);
            let rules = view.traffic_rules();
            assert_eq!(
                rules.rules[2].action,
                Action::ReplaceResponseBody {
                    find: "\"mocked\"".to_owned(),
                    replace: "\"changed\",\n\"by\": 1".to_owned(),
                }
            );
            view.edit_rule(2, cx);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "86-rule-body");
    window
        .update(&mut cx, |view, _, cx| {
            view.clear_rule_form(cx);
            view.delete_rule(2, cx);
        })
        .unwrap();

    // A script's rule: it answers by itself, and the rule says what it logged.
    let script = dir.join("flags.js");
    std::fs::write(
        &script,
        r#"function onRequest(request) {
             console.log("asked for", request.url);
             return { response: { status: 200, headers: { "content-type": "application/json" },
                                  body: JSON.stringify({ newCheckout: true }) } };
           }"#,
    )
    .unwrap();
    window
        .update(&mut cx, |view, _, cx| {
            view.clear_rule_form(cx);
            view.set_rule_kind(RuleKind::Script, cx);
            view.fill_rule(
                "Feature flags",
                "api.test/flags",
                &script.display().to_string(),
                "",
                cx,
            );
            view.save_rule(cx);
            assert_eq!(
                view.traffic_rules().rules[2].action,
                Action::Script {
                    path: script.clone()
                }
            );
        })
        .unwrap();
    assert_eq!(
        get("https://api.test/flags"),
        (200, r#"{"newCheckout":true}"#.to_owned())
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while view_log(&mut cx, window, &script).lines.is_empty() {
        assert!(std::time::Instant::now() < deadline, "the script's log");
        std::thread::sleep(Duration::from_millis(20));
    }
    window.update(&mut cx, |view, _, cx| view.look(cx)).unwrap();
    cx.run_until_parked();
    // Down to the rule.
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position: gpui::point(px(900.), px(600.)),
                delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-900.))),
                ..Default::default()
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "104-rule-script");
    window
        .update(&mut cx, |view, _, cx| view.delete_rule(2, cx))
        .unwrap();

    // A slow network: a preset, then numbers of one's own, then off again.
    window
        .update(&mut cx, |view, _, cx| {
            view.choose_network("Slow 3G", cx);
            let now = view.network_conditions();
            assert!(now.on);
            assert_eq!((now.latency_ms, now.down_kbps), (400, 400));
        })
        .unwrap();
    let kept = Conditions::load(&dir.join("throttle.toml")).unwrap();
    assert_eq!(kept.name, "Slow 3G", "kept for next time");
    cx.run_until_parked();
    shot(&mut cx, window.into(), "71-slow-network");
    window
        .update(&mut cx, |view, _, cx| {
            view.fill_network_form(["250", "2000", "", "", "api.test"], cx);
            view.apply_network_form(cx);
            let now = view.network_conditions();
            assert_eq!(now.name, "Custom");
            assert_eq!((now.latency_ms, now.down_kbps, now.up_kbps), (250, 2000, 0));
            assert_eq!(now.hosts, ["api.test"]);
            view.fill_network_form(["", "", "", "200", ""], cx);
            view.apply_network_form(cx);
            assert_eq!(
                view.network_conditions().latency_ms,
                250,
                "refused, as it was"
            );
            view.toggle_network(cx);
            assert!(!view.network_conditions().on);
        })
        .unwrap();
    assert!(!Conditions::load(&dir.join("throttle.toml")).unwrap().on);

    drop(cx);
    runtime.shutdown_background();
    std::fs::remove_dir_all(&dir).unwrap();
}

fn view_log(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    script: &std::path::Path,
) -> tracewing_proxy::script::ScriptLog {
    window
        .update(cx, |view, _, _| view.engine.scripts.log(script))
        .unwrap()
}

fn mock_path(view: &Traffic) -> String {
    view.data_dir_for_tests()
        .join("mock.json")
        .display()
        .to_string()
}
