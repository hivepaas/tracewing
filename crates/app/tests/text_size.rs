//! The text size chosen in Settings: the window grows with it, rows and
//! all, and the list and the body still scroll a row at a time. And the
//! rail's logo, which leads to About.
//!
//! `cargo test --release -p tracewing --features headless --test text_size`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, point, px, size};

mod common;
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

fn type_in(input: &Entity<TextInput>, text: &str, cx: &mut gpui::Context<Traffic>) {
    input.update(cx, |input, cx| input.set_text(text, 0..0, cx));
}

fn get(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, url: &str) {
    window
        .update(cx, |view, _, cx| {
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            type_in(&page.url.clone(), url, cx);
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(cx, window, "the answer", |view| {
        view.compose_page
            .as_ref()
            .is_some_and(|page| page.answer.is_some() || page.problem.is_some())
    });
}

#[test]
fn the_window_grows_with_the_text_size() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-text-size-{}", std::process::id()));
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
        ports: 20_400..20_410,
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
    let main: WindowHandle<Traffic> = cx
        .open_window(size(px(1440.), px(900.)), move |window, cx| {
            cx.new(|cx| Traffic::new(engine, &data_dir, window, cx))
        })
        .unwrap();
    for path in ["/json_big?mb=1", "/xml", "/csv", "/html"] {
        get(&mut cx, main, &format!("http://localhost:{port}{path}"));
    }
    main.update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, main, "the flows", |view| view.count() == 4);
    main.update(&mut cx, |view, _, cx| view.select_place(0, cx))
        .unwrap();
    wait_for(&mut cx, main, "the body", |view| {
        view.inspector.detail.is_some()
    });
    cx.run_until_parked();
    // 14 by default: a size above the design's 13.
    main.update(&mut cx, |view, window, _| {
        assert_eq!(view.text_size(), 14.);
        assert_eq!(window.rem_size(), px(16. * 14. / 13.));
    })
    .unwrap();

    // Larger: the window's rem with it, kept for next time.
    main.update(&mut cx, |view, _, cx| view.set_text_size(Some(18), cx))
        .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |view, window, _| {
        assert_eq!(view.text_size(), 18.);
        assert_eq!(window.rem_size(), px(16. * 18. / 13.));
    })
    .unwrap();
    let kept = tracewing::layout::Layout::load(&dir.join("layout.toml"));
    assert_eq!(kept.text_size, Some(18));
    cx.run_until_parked();
    shot(&mut cx, main.into(), "107-text-size-18");
    main.update(&mut cx, |view, _, cx| {
        view.set_screen(Screen::Settings, cx);
        view.show_settings_section(tracewing::layout::SettingsSection::General, cx);
    })
    .unwrap();
    cx.run_until_parked();
    shot(&mut cx, main.into(), "108-settings-text-size");

    // The rail's logo leads to About, the last part of Settings.
    main.update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    cx.run_until_parked();
    let logo = point(px(52.), px(36.));
    common::drag(&mut cx, main, logo, logo);
    main.update(&mut cx, |view, _, _| {
        assert_eq!(view.screen, Screen::Settings)
    })
    .unwrap();
    let kept = tracewing::layout::Layout::load(&dir.join("layout.toml"));
    assert_eq!(
        kept.settings_section,
        tracewing::layout::SettingsSection::About
    );

    // The design's 13 is a choice like any other now, and kept.
    main.update(&mut cx, |view, _, cx| view.set_text_size(Some(13), cx))
        .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |_, window, _| {
        assert_eq!(window.rem_size(), px(16.))
    })
    .unwrap();
    let kept = tracewing::layout::Layout::load(&dir.join("layout.toml"));
    assert_eq!(kept.text_size, Some(13));

    // The default again: nothing is kept for it.
    main.update(&mut cx, |view, _, cx| view.set_text_size(Some(14), cx))
        .unwrap();
    cx.run_until_parked();
    let kept = tracewing::layout::Layout::load(&dir.join("layout.toml"));
    assert_eq!(kept.text_size, None);
    cx.run_until_parked();
    shot(&mut cx, main.into(), "109-about");

    // Fonts: the code's from the monospaced fonts on this computer,
    // tracewing's own first; the interface's, its own or the system's. Each
    // applies at once and is kept.
    main.update(&mut cx, |view, _, cx| {
        view.set_screen(Screen::Settings, cx);
        view.show_settings_section(tracewing::layout::SettingsSection::General, cx);
    })
    .unwrap();
    cx.run_until_parked();
    let monos = main
        .update(&mut cx, |view, window, _| view.mono_fonts(window))
        .unwrap();
    assert_eq!(
        monos.first().map(String::as_str),
        Some(tracewing::MONO_FONT)
    );
    if let Some(other) = monos.get(1).cloned() {
        main.update(&mut cx, |view, _, cx| {
            view.set_code_font(Some(other.clone()), cx)
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(tracewing::mono_font().as_ref(), other.as_str());
        let kept = tracewing::layout::Layout::load(&dir.join("layout.toml"));
        assert_eq!(kept.code_font.as_deref(), Some(other.as_str()));
    }
    let system = main
        .update(&mut cx, |view, window, _| {
            tracewing::system_ui_font(&window.text_system().all_font_names())
                .map(|name| (name, view.text_size()))
        })
        .unwrap();
    if let Some((system, _)) = system {
        main.update(&mut cx, |view, _, cx| view.set_ui_font(true, cx))
            .unwrap();
        cx.run_until_parked();
        assert_eq!(tracewing::ui_font().as_ref(), system.as_str());
    }
    shot(&mut cx, main.into(), "120-settings-fonts");
    // Back to tracewing's own: nothing is kept for them.
    main.update(&mut cx, |view, _, cx| {
        view.set_ui_font(false, cx);
        view.set_code_font(None, cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(tracewing::ui_font().as_ref(), tracewing::FONT);
    assert_eq!(tracewing::mono_font().as_ref(), tracewing::MONO_FONT);
    let kept = tracewing::layout::Layout::load(&dir.join("layout.toml"));
    assert_eq!((kept.ui_font, kept.code_font), (None, None));

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
