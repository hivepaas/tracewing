//! The local way in's tab, drawn in every state it has.
//!
//! **This is a test that exists to be looked at.** The tab is assembled per
//! answer — what this machine can do, whether the switch is on, whether the
//! engine is serving it, whether anything is chosen — and a page built that
//! way inherits text written while a *different* answer was on screen. That
//! text stays true and stops being about the page it is on, so no assertion
//! can catch it. What catches it is drawing each state and reading it as a
//! reader would, starting with the one somebody meets first: everything off
//! and nothing chosen.
//!
//! So each state is drawn **and** kept in `target/shots`, and the states this
//! machine cannot be in are drawn too — which is the whole reason
//! `local_apps::Support` is a value on the view rather than a `cfg!` inside
//! the drawing, and the reason `Stopper` holds a `dyn Device` rather than a
//! Wintun adapter: a fake device is enough to draw the running states from
//! Linux.
//!
//! `cargo test --release -p tracewing --features headless --test local_tab`
//!
//! This file claims `ports: 20_550..20_560` for the proxy it starts, the next
//! free ten above `tests/refused.rs`.

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{path::Path, sync::Arc};
use tracewing::{
    engine::{Engine, EngineOptions},
    layout::{SettingsSection, Way},
    local_apps::{Local, Picked, Support},
    local_source::{NoSource, Running, Stopper},
    traffic::{Screen, Traffic},
};
use tracewing_proxy::{sieve::Chosen, tun::Device};
use tracewing_store::settings::StorageSettings;

/// A device that does nothing, so that the running states can be drawn on a
/// machine with no adapter of any kind.
struct Nothing;

impl Device for Nothing {
    fn name(&self) -> String {
        "nothing".to_owned()
    }

    fn read(&self) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("nothing to read")
    }

    fn write(&self, _packet: &[u8]) -> anyhow::Result<()> {
        Ok(())
    }

    fn shutdown(&self) {}
}

fn shot(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, name: &str) {
    cx.run_until_parked();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shots");
    std::fs::create_dir_all(&out).unwrap();
    cx.capture_screenshot(window.into())
        .expect("the window drew")
        .save(out.join(format!("{name}.png")))
        .unwrap();
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
        ports: 20_550..20_560,
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

/// Every state the tab has, drawn in the order a reader meets them.
#[test]
fn every_state_of_the_local_tab_draws() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-local-tab-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (mut cx, window) = open(&dir);
    let cx = &mut cx;

    // The tab itself, open, which is where all of this is drawn.
    window
        .update(cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
            view.show_settings_section(SettingsSection::Capture, cx);
            view.show_way(Way::Apps, cx);
            cx.notify();
        })
        .unwrap();

    // **First**, the state somebody meets: it can run here, the switch is
    // off, nothing is chosen. Everything a reader is told here has to be
    // about *this* page -- what the switch would do, and what the list is
    // for.
    let set = |view: &mut Traffic, support: Support, local: Local| {
        view.local.support = support;
        view.local.local = local;
    };
    window
        .update(cx, |view, _, cx| {
            set(view, Support::Ready, Local::default());
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-1-off-and-empty");

    // Chosen, still off: the list has something in it and the switch has not
    // been touched, which is what picking before turning on looks like.
    let two = vec![
        Picked::at("/usr/bin/curl".into()),
        Picked::at("/opt/my build".into()),
    ];
    window
        .update(cx, |view, _, cx| {
            set(
                view,
                Support::Ready,
                Local {
                    on: false,
                    programs: two.clone(),
                },
            );
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-2-off-with-programs");

    // On in the file and nothing running: the attempt ended without an
    // adapter. The sentence has to say that rather than "on".
    window
        .update(cx, |view, _, cx| {
            set(
                view,
                Support::Ready,
                Local {
                    on: true,
                    programs: two.clone(),
                },
            );
            view.engine.local = None;
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-3-on-and-taking-nothing");

    // On and running, with nothing chosen: the adapter is up and the machine
    // is spliced straight through. The tab must not imply capture.
    let running = |view: &mut Traffic, programs: Vec<String>| {
        view.engine.local = Some(Running {
            chosen: Chosen::of(programs),
            stopper: Stopper::of(Arc::new(Nothing)),
        });
    };
    window
        .update(cx, |view, _, cx| {
            set(
                view,
                Support::Ready,
                Local {
                    on: true,
                    programs: Vec::new(),
                },
            );
            running(view, Vec::new());
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-4-on-keeping-nothing");

    // On, running, keeping two.
    window
        .update(cx, |view, _, cx| {
            set(
                view,
                Support::Ready,
                Local {
                    on: true,
                    programs: two.clone(),
                },
            );
            running(view, vec!["/usr/bin/curl".to_owned()]);
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-5-on-keeping-two");

    // A start in flight, which is what somebody sees while a password
    // dialog waits in front of this window — the state this whole change
    // exists to make visible.
    window
        .update(cx, |view, _, cx| {
            set(
                view,
                Support::Ready,
                Local {
                    on: true,
                    programs: two.clone(),
                },
            );
            view.engine.local = None;
            view.local.starting = true;
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-6-starting");
    window
        .update(cx, |view, _, cx| {
            view.local.starting = false;
            cx.notify();
        })
        .unwrap();

    // Not an administrator: the switch offers to come back as one, and says
    // what that costs before it is pressed.
    window
        .update(cx, |view, _, cx| {
            set(
                view,
                Support::NeedsAdministrator,
                Local {
                    on: false,
                    programs: two.clone(),
                },
            );
            view.engine.local = None;
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-7-needs-administrator");

    // Refused: nothing started and nothing changed, and the switch is off.
    window
        .update(cx, |view, _, cx| {
            view.engine.local_problem = Some(NoSource::Refused);
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-8-refused");

    // Something else went wrong, which is the one case shown as it came.
    window
        .update(cx, |view, _, cx| {
            view.engine.local_problem = Some(NoSource::Failed(
                "create the tracewing adapter: Failed to create adapter".to_owned(),
            ));
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-9-failed");

    // And a system that cannot: no switch, no list, and a sentence that is
    // about this tab rather than about the macOS one.
    window
        .update(cx, |view, _, cx| {
            view.engine.local_problem = None;
            set(view, Support::NotOnThisSystem, Local::default());
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-10-not-on-this-system");

    // And the line that says tracewing itself is elevated, which is the
    // visible half of the relaunch: the thing worth avoiding was never
    // elevation, which somebody agreed to, but elevation nobody can see. It
    // lives in the status bar rather than the title bar because the title
    // bar is the system's on some platforms and is not drawn here at all --
    // a state nobody can draw is a state nobody checks.
    window
        .update(cx, |view, _, cx| {
            set(view, Support::Ready, Local::default());
            view.elevated = true;
            cx.notify();
        })
        .unwrap();
    shot(cx, window, "local-11-running-as-administrator");

    let _ = std::fs::remove_dir_all(&dir);
}
