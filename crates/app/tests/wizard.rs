//! Set up as a wizard: one question, and then only what its answer needs.
//!
//! What the routing itself decides — which way in reaches a Java program,
//! which four targets a terminal answers — is tested in `tracewing`'s own
//! `wizard` module, as plain functions. What is tested here is what only a
//! window can show: that every answer **draws**, that the answer is kept
//! across a restart, and that it can be taken back.
//!
//! Drawing each one matters more than it looks. The cards a route shows are
//! Set up's own — browsers, other devices, Android, the simulators — and they
//! read the system as they draw: a card that is fine in the pile Set up used
//! to show can still be the only thing on the page for one answer, and that
//! is the case nothing covered before.
//!
//! `cargo test --release -p tracewing --features headless --test wizard`
//!
//! This file claims `ports: 20_530..20_540` for the proxy it starts, the next
//! free ten above the top of the ranges `tests/wireguard.rs` lists.

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{path::Path, sync::Arc};
use tracewing::{
    engine::{Engine, EngineOptions},
    layout::Layout,
    traffic::{Screen, Traffic},
    wizard::{Target, route},
};
use tracewing_store::settings::StorageSettings;

/// Forces a draw, which is the whole point of the test: a card that panics
/// or cannot find what it reads does so while drawing, not while the answer
/// is being stored.
fn draw(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>) {
    cx.run_until_parked();
    cx.capture_screenshot(window.into())
        .expect("the window drew");
}

/// The answer as the next start would read it.
fn kept(dir: &Path) -> Option<Target> {
    Layout::load(&dir.join("layout.toml")).capture_target
}

fn open(dir: &Path) -> (HeadlessAppContext, WindowHandle<Traffic>) {
    let engine = Engine::start(EngineOptions {
        data_dir: dir.to_path_buf(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_530..20_540,
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
    let data_dir = dir.to_path_buf();
    let window = cx
        .open_window(size(px(1440.), px(900.)), move |window, cx| {
            cx.new(|cx| Traffic::new(engine, &data_dir, window, cx))
        })
        .unwrap();
    (cx, window)
}

#[test]
fn every_answer_draws_a_page_and_the_answer_is_kept() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-wizard-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (mut cx, window) = open(&dir);
    let cx = &mut cx;

    // Unanswered, which is how somebody meets it: the question and nothing
    // else that could overwhelm them.
    window
        .update(cx, |view, _, cx| {
            view.set_screen(Screen::Setup, cx);
        })
        .unwrap();
    draw(cx, window);
    assert_eq!(
        kept(&dir),
        None,
        "nothing is assumed about what somebody wants"
    );

    // Every answer, drawn. `Target::here` rather than `ALL`, because an
    // answer this computer cannot do is not offered: on Linux that leaves the
    // Mac app out.
    for target in Target::here() {
        window
            .update(cx, |view, _, cx| {
                view.choose_target(Some(target), cx);
                // The page is built from the route, so this is what is on it.
                let route = route(target);
                assert!(
                    !route.cards.is_empty(),
                    "{target:?} would draw a question and an empty page"
                );
            })
            .unwrap();
        draw(cx, window);
        // Kept where somebody left it, read from the file rather than from
        // the window — reading the window back would pass even if nothing
        // were written, and the point is that the page opens where they left
        // it after a restart.
        assert_eq!(kept(&dir), Some(target), "the answer outlives the window");
    }

    let last = *Target::here().last().unwrap();

    // And it can be taken back, by choosing the open answer again: a wizard
    // that cannot be unanswered is a page somebody is stuck on.
    window
        .update(cx, |view, _, cx| {
            view.choose_target(Some(last), cx);
        })
        .unwrap();
    draw(cx, window);
    assert_eq!(kept(&dir), None, "taking it back is written too");

    let _ = std::fs::remove_dir_all(&dir);
}
