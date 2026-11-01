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
use tracewing_proxy::traffic_rules::TrafficRules;
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

    drop(cx);
    runtime.shutdown_background();
    std::fs::remove_dir_all(&dir).unwrap();
}

fn mock_path(view: &Traffic) -> String {
    view.data_dir_for_tests()
        .join("mock.json")
        .display()
        .to_string()
}
