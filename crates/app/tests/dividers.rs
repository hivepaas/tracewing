//! The inspector's place and size, by hand: its divider dragged wider beside
//! the list and shorter below it, its handle dragged to the drop zone below
//! the list, and the inspector hidden and shown again by its shortcut. Each
//! is read back from the layout file, where the window keeps it.
//!
//! The dividers are found from where the content was laid out
//! (`Traffic::content_bounds`): the inspector lies against its right edge
//! beside the list and against its bottom below it, and the divider is the
//! strip just before it.
//!
//! `cargo test --release -p tracewing --features headless --test dividers`

#![cfg(all(feature = "headless", target_os = "linux"))]

mod common;

use gpui::{
    AppContext as _, HeadlessAppContext, Modifiers, MouseButton, MouseMoveEvent, MouseUpEvent,
    PlatformInput, WindowHandle, point, px, size,
};
use std::{path::PathBuf, sync::Arc};
use tracewing::{
    engine::{Engine, EngineOptions},
    layout::{Layout, Place},
    traffic::{Screen, Traffic},
};
use tracewing_store::settings::StorageSettings;

/// Half the divider's width, at the design's size: its middle is this far
/// before the inspector's edge.
const HALF_DIVIDER: f32 = 5.;

/// How much larger than the design the interface is drawn: its rem against
/// the design's 16 px.
fn scale(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>) -> f32 {
    window
        .update(cx, |_, window, _| f32::from(window.rem_size()) / 16.)
        .unwrap()
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
fn the_inspector_is_resized_moved_and_hidden_by_hand() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-dividers-{}", std::process::id()));
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
        ports: 20_640..20_650,
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
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    cx.run_until_parked();
    let kept = || Layout::load(&dir.join("layout.toml"));
    let content = window
        .update(&mut cx, |view, _, _| view.content_bounds())
        .unwrap();
    let before = kept();
    assert_eq!(before.place, Place::Beside);
    let scale = scale(&mut cx, window);

    // Beside: the divider dragged 100 px to the left makes it as much wider,
    // in the design's pixels.
    let x = content.right() - px((before.inspector_w + HALF_DIVIDER) * scale);
    let y = content.center().y;
    common::drag(&mut cx, window, point(x, y), point(x - px(100.), y));
    let wider = kept().inspector_w;
    assert!(
        // Measured from the pointer, which was pressed half a divider before
        // the edge: that half is the inspector's too, once let go.
        (wider - (before.inspector_w + HALF_DIVIDER + 100. / scale)).abs() < 1.,
        "{} → {wider}",
        before.inspector_w
    );
    shot(&mut cx, window.into(), "140-inspector-dragged-wider");

    // Its handle pressed, then let go over the zone below the list: it goes
    // below.
    window
        .update(&mut cx, |view, _, cx| view.begin_inspector_drag(cx))
        .unwrap();
    let below = point(
        content.left() + content.size.width / 3.,
        content.bottom() - px(60.),
    );
    for event in [
        PlatformInput::MouseMove(MouseMoveEvent {
            position: below,
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::default(),
        }),
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: below,
            modifiers: Modifiers::default(),
            click_count: 1,
        }),
    ] {
        common::mouse(&mut cx, window, event);
    }
    assert_eq!(kept().place, Place::Below);

    // Below: the divider dragged 80 px down makes it as much shorter. Up
    // would soon meet the list's least height, which the window keeps.
    let content = window
        .update(&mut cx, |view, _, _| view.content_bounds())
        .unwrap();
    let h = kept().inspector_h;
    let x = content.center().x;
    let y = content.bottom() - px((h + HALF_DIVIDER) * scale);
    common::drag(&mut cx, window, point(x, y), point(x, y + px(80.)));
    let shorter = kept().inspector_h;
    assert!(
        (shorter - (h + HALF_DIVIDER - 80. / scale)).abs() < 1.,
        "{h} → {shorter}"
    );
    shot(&mut cx, window.into(), "141-inspector-below-shorter");

    // Hidden by its shortcut, and shown again.
    common::key(&mut cx, window, "ctrl-i");
    assert!(!kept().inspector_shown);
    shot(&mut cx, window.into(), "142-inspector-hidden");
    common::key(&mut cx, window, "ctrl-i");
    assert!(kept().inspector_shown);

    std::fs::remove_dir_all(&dir).ok();
}
