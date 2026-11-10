//! The keyboard: each shortcut does what the list of them says, and a text
//! field keeps the keys it types with.
//!
//! `cargo test --release -p tracewing --features headless --test keys`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{
    AppContext as _, Entity, Focusable as _, HeadlessAppContext, Keystroke, WindowHandle, px, size,
};
use std::{
    path::PathBuf,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    detail::Side,
    engine::{Engine, EngineOptions},
    input::TextInput,
    rules_page::RulesPage,
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

fn press(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, keys: &str) {
    // The window, not the view: the key goes to the view, which must be free.
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_keystroke(Keystroke::parse(keys).unwrap(), cx);
    })
    .unwrap();
    cx.run_until_parked();
}

/// The `size` the chosen flow asked for.
fn chosen(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>) -> Option<String> {
    window
        .update(cx, |view, _, _| {
            let url = &view.inspector.detail.as_ref()?.url;
            Some(url.rsplit_once('=')?.1.to_owned())
        })
        .unwrap()
}

#[test]
fn shortcuts_do_what_they_say() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-keys-{}", std::process::id()));
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
        ports: 20_230..20_240,
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

    for size in [1, 2, 3] {
        let out = Command::new("curl")
            .args(["-sS", "-o", "/dev/null", "--proxy"])
            .arg(format!("http://{proxy}"))
            .arg("--cacert")
            .arg(dir.join("ca.pem"))
            .arg(format!("https://localhost:{origin}/json?size={size}"))
            .output()
            .expect("curl");
        assert!(out.status.success(), "{out:?}");
    }
    window
        .update(&mut cx, |view, window, cx| {
            view.set_screen(Screen::Traffic, cx);
            window.focus(&view.focus_handle(cx), cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "three flows", |view| view.count() == 3);

    // Through the list.
    press(&mut cx, window, "down");
    assert_eq!(chosen(&mut cx, window).as_deref(), Some("1"));
    press(&mut cx, window, "end");
    assert_eq!(chosen(&mut cx, window).as_deref(), Some("3"));
    press(&mut cx, window, "up");
    assert_eq!(chosen(&mut cx, window).as_deref(), Some("2"));
    press(&mut cx, window, "home");
    assert_eq!(chosen(&mut cx, window).as_deref(), Some("1"));

    // The chosen flow's sides.
    press(&mut cx, window, "secondary-[");
    assert_eq!(
        window.update(&mut cx, |v, _, _| v.inspector.side).unwrap(),
        Side::Request
    );
    press(&mut cx, window, "secondary-]");
    assert_eq!(
        window.update(&mut cx, |v, _, _| v.inspector.side).unwrap(),
        Side::Response
    );

    // The chosen flow's parts, round the ends.
    let part = |cx: &mut HeadlessAppContext| window.update(cx, |v, _, _| v.inspector.part).unwrap();
    let first = part(&mut cx);
    press(&mut cx, window, "]");
    assert_ne!(part(&mut cx), first);
    press(&mut cx, window, "[");
    assert_eq!(part(&mut cx), first);
    let offered = window.update(&mut cx, |v, _, _| v.parts_offered()).unwrap();
    if offered[0] == first {
        press(&mut cx, window, "[");
        assert_eq!(part(&mut cx), *offered.last().unwrap(), "round to the last");
        press(&mut cx, window, "]");
    }

    // Recording, and the screens by number.
    press(&mut cx, window, "secondary-r");
    assert!(
        !window
            .update(&mut cx, |v, _, _| v.engine.is_recording())
            .unwrap()
    );
    press(&mut cx, window, "secondary-r");
    assert!(
        window
            .update(&mut cx, |v, _, _| v.engine.is_recording())
            .unwrap()
    );
    press(&mut cx, window, "secondary-3");
    assert_eq!(
        window.update(&mut cx, |v, _, _| v.screen).unwrap(),
        Screen::Rules
    );
    // Tab goes from field to field, and Shift-Tab back.
    let focused = |cx: &mut HeadlessAppContext, which: fn(&RulesPage) -> &Entity<TextInput>| {
        window
            .update(cx, |v, window, cx| {
                let page = v.rules_page.as_ref().unwrap();
                which(page).read(cx).focus_handle(cx).is_focused(window)
            })
            .unwrap()
    };
    window
        .update(&mut cx, |v, window, cx| {
            let page = v.rules_page.as_ref().unwrap();
            let name = page.name.read(cx).focus_handle(cx);
            window.focus(&name, cx);
        })
        .unwrap();
    press(&mut cx, window, "tab");
    assert!(focused(&mut cx, |page| &page.url), "the URL after the name");
    press(&mut cx, window, "shift-tab");
    assert!(focused(&mut cx, |page| &page.name), "and back");
    window
        .update(&mut cx, |view, window, cx| {
            window.focus(&view.focus_handle(cx), cx);
        })
        .unwrap();
    press(&mut cx, window, "secondary-2");
    assert_eq!(
        window.update(&mut cx, |v, _, _| v.screen).unwrap(),
        Screen::Traffic
    );

    // The list of them, opened and closed.
    press(&mut cx, window, "shift-?");
    assert!(
        window
            .update(&mut cx, |v, _, _| v.shortcuts_open())
            .unwrap()
    );
    shot(&mut cx, window.into(), "66-shortcuts");
    press(&mut cx, window, "escape");
    assert!(
        !window
            .update(&mut cx, |v, _, _| v.shortcuts_open())
            .unwrap()
    );

    // In the filter, `?` is a character and the arrows move the caret.
    press(&mut cx, window, "secondary-f");
    press(&mut cx, window, "shift-?");
    assert!(
        !window
            .update(&mut cx, |v, _, _| v.shortcuts_open())
            .unwrap()
    );
    press(&mut cx, window, "end");
    assert_eq!(chosen(&mut cx, window).as_deref(), Some("1"));
    // `mod` keys still work there.
    press(&mut cx, window, "secondary-/");
    assert!(
        window
            .update(&mut cx, |v, _, _| v.shortcuts_open())
            .unwrap()
    );
    press(&mut cx, window, "escape");

    // Cleared.
    window
        .update(&mut cx, |view, window, cx| {
            window.focus(&view.focus_handle(cx), cx);
        })
        .unwrap();
    press(&mut cx, window, "secondary-k");
    wait_for(&mut cx, window, "an empty list", |view| view.count() == 0);

    std::fs::remove_dir_all(&dir).ok();
}
