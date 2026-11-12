//! The relay in Settings: an address written and applied takes the next
//! connection through it, and is kept; one that cannot be used changes
//! nothing and says why.
//!
//! `cargo test --release -p tracewing --features headless --test relay`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    input::TextInput,
    traffic::{Screen, Traffic},
};
use tracewing_dev::fake_relay;
use tracewing_proxy::relay::RelaySettings;
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

fn set(input: &Entity<TextInput>, text: &str, cx: &mut gpui::Context<Traffic>) {
    input.update(cx, |input, cx| {
        let all = 0..input.text().len();
        input.set_text(text, all, cx);
    });
}

#[test]
fn a_relay_written_in_settings_is_used_and_kept() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-relay-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let bind = || {
        runtime
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .unwrap()
    };
    let server = bind();
    let port = server.local_addr().unwrap().port();
    runtime.spawn(tracewing_dev::playground::serve(server));
    let corp = bind();
    let corp_port = corp.local_addr().unwrap().port();
    let seen = fake_relay::Seen::default();
    runtime.spawn(fake_relay::serve_http(
        corp,
        Some(("an".to_owned(), "p@ss".to_owned())),
        Arc::clone(&seen),
    ));

    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_300..20_310,
        extra_roots: Vec::new(),
        key_store: Default::default(),
        find_apps: false,
    })
    .unwrap();
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

    // Written and applied: kept, and said.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
        })
        .unwrap();
    cx.run_until_parked();
    window
        .update(&mut cx, |view, _, cx| {
            let fields = view.relay_fields.as_ref().expect("shown in Settings");
            let (proxy, bypass) = (fields.proxy.clone(), fields.bypass.clone());
            assert_eq!(bypass.read(cx).text(), "localhost, 127.0.0.1, ::1");
            set(
                &proxy,
                &format!("http://an:p%40ss@127.0.0.1:{corp_port}"),
                cx,
            );
            set(&bypass, "", cx);
            view.apply_relay(cx);
            let said = view.relay_fields.as_ref().unwrap().said.clone().unwrap();
            assert!(said.0, "{said:?}");
            assert!(
                said.1.contains(&format!("127.0.0.1:{corp_port}")),
                "{said:?}"
            );
            // Tests keep it in the file, never in a keychain, and say so.
            assert!(said.1.contains("relay.toml"), "{said:?}");
            assert_eq!(
                proxy.read(cx).text(),
                format!("http://an@127.0.0.1:{corp_port}"),
                "the password is not left on show"
            );
        })
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(dir.join("relay.toml"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "only this user reads it");
    }
    assert_eq!(
        RelaySettings::load(&dir).unwrap(),
        RelaySettings {
            proxy: format!("http://an:p%40ss@127.0.0.1:{corp_port}"),
            bypass: String::new(),
        }
    );
    cx.run_until_parked();
    shot(&mut cx, window.into(), "83-relay");

    // The next request goes through it.
    window
        .update(&mut cx, |view, _, cx| {
            view.new_compose(cx);
            let url = view.compose_page.as_ref().unwrap().url.clone();
            set(&url, &format!("http://localhost:{port}/json"), cx);
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the answer", |view| {
        view.compose_page
            .as_ref()
            .is_some_and(|page| page.answer.is_some() || page.problem.is_some())
    });
    window
        .update(&mut cx, |view, _, _| {
            let page = view.compose_page.as_ref().unwrap();
            assert_eq!(page.answer.as_ref().map(|a| a.status), Some(200));
        })
        .unwrap();
    assert_eq!(
        *seen.lock().unwrap(),
        [format!("GET http://localhost:{port}/json auth=yes")]
    );

    // One that cannot be used: said, and nothing changed.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
            let proxy = view.relay_fields.as_ref().unwrap().proxy.clone();
            set(&proxy, "proxy.corp:8080", cx);
            view.apply_relay(cx);
            let said = view.relay_fields.as_ref().unwrap().said.clone().unwrap();
            assert!(!said.0 && said.1.contains("scheme"), "{said:?}");
            assert!(view.engine.relay.route_for("example.com").is_some(), "kept");
        })
        .unwrap();
    assert_eq!(
        RelaySettings::load(&dir).unwrap().proxy,
        format!("http://an:p%40ss@127.0.0.1:{corp_port}")
    );

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
