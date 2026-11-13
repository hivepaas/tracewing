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
    flows::group_digits,
    inspector::BodyState,
    traffic::{Screen, Traffic},
};
use tracewing_store::{
    session::{self, Trouble},
    settings::StorageSettings,
};

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
        key_store: Default::default(),
        find_apps: true,
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

    // A session the disk has damaged is not one from another version, and
    // the screen must not say it is: the two send a reader to different
    // places. Half of this one's pages are gibberish.
    let broken = sessions.join("1000000001");
    tracewing_dev::seed::seed(&broken, ROWS as u64, true).unwrap();
    let path = session::flows_db(&broken);
    let mut bytes = std::fs::read(&path).unwrap();
    let from = bytes.len() / 2;
    for byte in &mut bytes[from..] {
        *byte = 0x5a;
    }
    std::fs::write(&path, &bytes).unwrap();
    window
        .update(&mut cx, |view, _, cx| view.refresh_sessions(cx))
        .unwrap();
    wait_for(&mut cx, window, "the damaged one listed", |view| {
        view.sessions_page.sessions.len() == 2
    });
    window
        .update(&mut cx, |view, _, _| {
            let about = view
                .sessions_page
                .sessions
                .iter()
                .find(|about| about.dir == broken)
                .expect("listed");
            assert!(!about.readable);
            assert!(
                matches!(about.trouble, Some(Trouble::Damaged(_))),
                "damaged, not another version: {:?}",
                about.trouble
            );
        })
        .unwrap();

    // Nothing has read its pages until this is asked for.
    window
        .update(&mut cx, |view, _, cx| view.check_session(&broken, cx))
        .unwrap();
    wait_for(&mut cx, window, "the check", |view| {
        view.sessions_page.checked.contains_key(&broken)
    });
    window
        .update(&mut cx, |view, _, _| {
            let said = view.sessions_page.checked[&broken].to_string();
            assert!(said.contains("SQLite"), "it says what SQLite found: {said}");
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "19-session-damaged");

    // What it still holds is worth more than the file: rescued into a new
    // session, which reads like any other, while the damaged one stays
    // where it is for anyone who wants another go at it.
    window
        .update(&mut cx, |view, _, cx| view.rescue_session(&broken, cx))
        .unwrap();
    wait_for(&mut cx, window, "the rescue", |view| {
        view.sessions_page.checked.contains_key(&broken)
    });
    wait_for(&mut cx, window, "the new session listed", |view| {
        view.sessions_page.sessions.len() == 3
    });
    window
        .update(&mut cx, |view, _, _| {
            let said = view.sessions_page.checked[&broken].to_string();
            assert!(said.contains("rescued"), "it says what came out: {said}");
            let rescued = view
                .sessions_page
                .sessions
                .iter()
                .find(|about| about.name.as_deref() == Some("rescued from 1000000001"))
                .expect("the rescued session is listed");
            assert!(rescued.readable, "{:?}", rescued.trouble);
            assert!(rescued.flows > 0, "with flows in it");
            // The number on the row is the number that was rescued. The
            // screen counts a session's flows by the span of its ids, so
            // the rescued rows are numbered again; before that, a rescue of
            // 1,448 was listed as 1,460.
            assert!(
                said.contains(&group_digits(rescued.flows)),
                "{said} against {} on the row",
                rescued.flows
            );
            assert!(broken.exists(), "and the damaged one is still there");
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "20-session-rescued");

    // Recording moves to a new session without the app being restarted.
    // This is also the way back from a proxy that has stopped, and the only
    // way to stop recording into a session you would rather delete: the
    // live one cannot be deleted.
    let live = window
        .update(&mut cx, |view, _, _| view.engine.session.clone())
        .unwrap();
    window
        .update(&mut cx, |view, _, cx| view.start_new_session(cx))
        .unwrap();
    let (now, listening) = window
        .update(&mut cx, |view, _, _| {
            (view.engine.session.clone(), view.engine.addr)
        })
        .unwrap();
    assert_ne!(now, live, "a session of its own");
    // The one before it had nothing in it, and a start takes unnamed
    // sessions with no flows away: that rule is older than this button. One
    // with flows in it stays, which is the whole point of the button.
    assert!(!live.exists(), "an empty session is not kept");
    // It is listening again, and on a port a client can reach.
    assert!(
        listening.port() >= 19_490 && listening.port() < 19_500,
        "{listening}"
    );
    wait_for(&mut cx, window, "an empty list", |view| {
        !view.is_viewing() && view.count() == 0
    });
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Sessions, cx);
            view.refresh_sessions(cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the new one listed", |view| {
        view.sessions_page
            .sessions
            .iter()
            .any(|about| about.dir == now)
    });
    window
        .update(&mut cx, |view, _, _| {
            // The rescued one is still there: it has flows and a name, so
            // neither the clean-up nor the new session touches it.
            assert!(
                view.sessions_page
                    .sessions
                    .iter()
                    .any(|about| about.name.as_deref() == Some("rescued from 1000000001")),
                "what was rescued outlives the session that was recording"
            );
        })
        .unwrap();

    drop(cx);
    std::fs::remove_dir_all(&dir).unwrap();
}
