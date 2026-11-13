//! Back and forward, as a browser goes: the mouse's own buttons and
//! Alt+Left/Right, through the screens and the parts of Settings.
//!
//! `cargo test --release -p tracewing --features headless --test back_forward`

#![cfg(all(feature = "headless", target_os = "linux"))]

mod common;

use gpui::{
    AppContext as _, HeadlessAppContext, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    NavigationDirection, PlatformInput, WindowHandle, point, px, size,
};
use std::sync::Arc;
use tracewing::{
    engine::{Engine, EngineOptions},
    layout::SettingsSection,
    navigation::Location,
    traffic::{Screen, Traffic},
};
use tracewing_store::settings::StorageSettings;

/// The mouse's back or forward button, pressed and let go over the window.
fn mouse_button(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, back: bool) {
    let button = MouseButton::Navigate(if back {
        NavigationDirection::Back
    } else {
        NavigationDirection::Forward
    });
    let position = point(px(720.), px(450.));
    common::mouse(
        cx,
        window,
        PlatformInput::MouseDown(MouseDownEvent {
            button,
            position,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        }),
    );
    common::mouse(
        cx,
        window,
        PlatformInput::MouseUp(MouseUpEvent {
            button,
            position,
            modifiers: Modifiers::default(),
            click_count: 1,
        }),
    );
}

fn here(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>) -> Location {
    window.update(cx, |view, _, _| view.here()).unwrap()
}

fn at(screen: Screen) -> Location {
    Location {
        screen,
        section: None,
        session: None,
    }
}

fn settings(section: SettingsSection) -> Location {
    Location {
        screen: Screen::Settings,
        section: Some(section),
        session: None,
    }
}

#[test]
fn back_and_forward_go_through_the_places_shown() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-back-forward-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_420..20_430,
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
    cx.run_until_parked();
    assert_eq!(here(&mut cx, main), at(Screen::Traffic));

    // Traffic, Rules, Settings' General, then its Network, then Compose.
    main.update(&mut cx, |view, _, cx| {
        view.set_screen(Screen::Rules, cx);
        view.set_screen(Screen::Settings, cx);
        view.show_settings_section(SettingsSection::Network, cx);
        view.set_screen(Screen::Compose, cx);
    })
    .unwrap();
    cx.run_until_parked();

    // Back with the mouse, and with the keys, one place at a time.
    mouse_button(&mut cx, main, true);
    assert_eq!(here(&mut cx, main), settings(SettingsSection::Network));
    common::key(&mut cx, main, "alt-left");
    assert_eq!(here(&mut cx, main), settings(SettingsSection::General));
    mouse_button(&mut cx, main, true);
    assert_eq!(here(&mut cx, main), at(Screen::Rules));
    mouse_button(&mut cx, main, true);
    assert_eq!(here(&mut cx, main), at(Screen::Traffic));
    // Nothing before the first place.
    mouse_button(&mut cx, main, true);
    assert_eq!(here(&mut cx, main), at(Screen::Traffic));

    // Forward again.
    mouse_button(&mut cx, main, false);
    assert_eq!(here(&mut cx, main), at(Screen::Rules));
    common::key(&mut cx, main, "alt-right");
    assert_eq!(here(&mut cx, main), settings(SettingsSection::General));

    // Going somewhere new forgets what was ahead.
    main.update(&mut cx, |view, _, cx| view.set_screen(Screen::Sessions, cx))
        .unwrap();
    cx.run_until_parked();
    mouse_button(&mut cx, main, false);
    assert_eq!(here(&mut cx, main), at(Screen::Sessions));
    mouse_button(&mut cx, main, true);
    assert_eq!(here(&mut cx, main), settings(SettingsSection::General));

    // The rail's logo is a place like any other: About, then back.
    main.update(&mut cx, |view, _, cx| view.show_about(cx))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(here(&mut cx, main), settings(SettingsSection::About));
    mouse_button(&mut cx, main, true);
    assert_eq!(here(&mut cx, main), settings(SettingsSection::General));

    // A text field keeps Alt+Left for moving by words.
    common::key(&mut cx, main, "ctrl-f");
    assert_eq!(here(&mut cx, main), at(Screen::Traffic));
    common::key(&mut cx, main, "alt-left");
    assert_eq!(here(&mut cx, main), at(Screen::Traffic));

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
