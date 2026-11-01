//! An earlier session, opened again from the window with no display.
//!
//! `cargo test --release -p tracewing --features headless --test sessions`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    inspector::BodyState,
    traffic::{Screen, Traffic},
};
use tracewing_store::{session, settings::StorageSettings};

const ROWS: usize = 3000;

fn wait_for(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    what: &str,
    done: impl Fn(&Traffic) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
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
fn an_earlier_session_opens_again() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-sessions-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let sessions = dir.join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();

    // As a crash leaves one: flows on disk, and no index at all.
    let earlier = sessions.join("1000000000");
    tracewing_dev::seed::seed(&earlier, ROWS as u64, true).unwrap();

    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 19_490..19_500,
        extra_roots: Vec::new(),
    })
    .unwrap();
    assert!(earlier.exists(), "starting does not lose it");

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
    while !session::flows_db(
        &window
            .update(&mut cx, |view, _, _| view.engine.session.clone())
            .unwrap(),
    )
    .exists()
    {
        std::thread::sleep(Duration::from_millis(10));
    }

    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Sessions, cx))
        .unwrap();
    wait_for(&mut cx, window, "the list", |view| {
        view.sessions_page.sessions.len() == 2
    });
    window
        .update(&mut cx, |view, _, _| {
            let listed = &view.sessions_page.sessions;
            assert_eq!(listed[1].dir, earlier, "the newest first");
            assert_eq!(listed[1].flows, ROWS as u64);
            assert!(listed[1].readable);
            assert_eq!(listed[0].dir, view.engine.session);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "16-sessions");

    // Opened: every flow is there, read-only, with its search index finished.
    window
        .update(&mut cx, |view, _, cx| {
            view.open_session(&earlier, cx);
            view.set_screen(Screen::Traffic, cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the flows", |view| view.count() == ROWS);
    assert!(
        window
            .update(&mut cx, |view, _, _| view.is_viewing())
            .unwrap()
    );
    wait_for(&mut cx, window, "the index", |view| !view.is_catching_up());
    window
        .update(&mut cx, |view, _, cx| {
            view.set_filter_text("thanh toan", cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the search", |view| {
        !view.is_filtering() && view.count() > 0 && view.count() <= ROWS
    });

    window
        .update(&mut cx, |view, _, cx| {
            assert_eq!(view.first_row_in_view(), 0, "an answer starts at the top");
            cx.notify();
        })
        .unwrap();

    // Its bodies and details come from it, not from the live session.
    window
        .update(&mut cx, |view, _, cx| view.select_place(0, cx))
        .unwrap();
    wait_for(&mut cx, window, "the body", |view| {
        matches!(view.inspector.body, BodyState::Shown(_))
    });
    cx.run_until_parked();
    shot(&mut cx, window.into(), "17-earlier-session");

    // While it is on show it cannot be deleted.
    window
        .update(&mut cx, |view, _, cx| {
            view.delete_session(&earlier, cx);
            view.delete_session(&earlier, cx);
            view.set_filter_text("", cx);
            view.back_to_live(cx);
        })
        .unwrap();
    assert!(earlier.exists());
    wait_for(&mut cx, window, "the live session", |view| {
        !view.is_viewing() && view.count() == 0
    });

    // Named, it has a name on disk. Deleting takes asking twice.
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Sessions, cx))
        .unwrap();
    wait_for(&mut cx, window, "the list again", |view| {
        view.sessions_page.sessions.len() == 2
    });
    window
        .update(&mut cx, |view, _, cx| {
            view.name_session(&earlier, "checkout bug", cx)
        })
        .unwrap();
    assert_eq!(session::name_of(&earlier).as_deref(), Some("checkout bug"));
    window
        .update(&mut cx, |view, _, cx| view.delete_session(&earlier, cx))
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "18-delete-asked");
    assert!(earlier.exists(), "asked, not done");
    window
        .update(&mut cx, |view, _, cx| view.delete_session(&earlier, cx))
        .unwrap();
    assert!(!earlier.exists());
    wait_for(&mut cx, window, "the shorter list", |view| {
        view.sessions_page.sessions.len() == 1
    });

    drop(cx);
    std::fs::remove_dir_all(&dir).unwrap();
}
