//! The system proxy from the window, and the settings screen. On Linux the system
//! proxy is a `gsettings` call, so a stand-in on the PATH keeps this test off the
//! real desktop's settings: it can run on any machine, and leaves nothing behind.
//!
//! `cargo test --release -p tracewing --features headless --test connect`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    io::Write,
    net::SocketAddr,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    settings_page::SettingsPage,
    system::{CaState, ProxySwitch},
    traffic::{Screen, Traffic},
};
use tracewing_store::settings::StorageSettings;
use tracewing_system::Scope;

/// A `gsettings` that keeps its values in files in a folder.
/// A `certutil` that keeps one certificate as a file. `-N` makes a database.
const FAKE_CERTUTIL: &str = r#"#!/bin/sh
store="$FAKE_GSETTINGS_DIR"
case "$1" in
-N) db=$(echo "$4" | sed 's/^sql://'); touch "$db/cert9.db" ;;
-A) touch "$store/certutil-added" ;;
-L) test -e "$store/certutil-added" ;;
esac
"#;

const FAKE_GSETTINGS: &str = r#"#!/bin/sh
store="$FAKE_GSETTINGS_DIR"
case "$1" in
get) cat "$store/$2.$3" 2>/dev/null || echo "''" ;;
set) printf '%s' "$4" > "$store/$2.$3" ;;
esac
"#;

fn mode(store: &Path) -> String {
    std::fs::read_to_string(store.join("org.gnome.system.proxy.mode")).unwrap_or_default()
}

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

fn save_shot(cx: &mut HeadlessAppContext, window: gpui::AnyWindowHandle, name: &str) {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shots");
    std::fs::create_dir_all(&out).unwrap();
    cx.capture_screenshot(window)
        .unwrap()
        .save(out.join(format!("{name}.png")))
        .unwrap();
}

#[test]
fn the_system_proxy_and_the_settings_from_the_window() {
    let dir = std::env::temp_dir().join(format!("tracewing-connect-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (bin, store) = (dir.join("bin"), dir.join("gsettings"));
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&store).unwrap();
    let fake = bin.join("gsettings");
    std::fs::write(&fake, FAKE_GSETTINGS).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(store.join("org.gnome.system.proxy.mode"), "'auto'").unwrap();
    let certutil = bin.join("certutil");
    std::fs::write(&certutil, FAKE_CERTUTIL).unwrap();
    std::fs::set_permissions(&certutil, std::fs::Permissions::from_mode(0o755)).unwrap();
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    // SAFETY: the only test in this binary, before it starts any thread of its own.
    unsafe {
        std::env::set_var("PATH", &path);
        std::env::set_var("FAKE_GSETTINGS_DIR", &store);
        std::env::set_var("XDG_CURRENT_DESKTOP", "GNOME");
        std::env::remove_var("KDE_SESSION_VERSION");
        std::env::set_var("HOME", &home);
    }

    let _ = tracewing_proxy::crypto_provider().install_default();
    let data_dir = dir.join("data");
    std::fs::create_dir_all(&data_dir).unwrap();
    let engine = Engine::start(EngineOptions {
        data_dir: data_dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        ports: 19_390..19_400,
        extra_roots: Vec::new(),
    })
    .unwrap();
    let addr: SocketAddr = engine.addr;

    let text = Arc::new(gpui_wgpu::CosmicTextSystem::new(tracewing::FONT));
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(tracewing::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(tracewing::load_fonts);
    let opened_dir = data_dir.clone();
    let window: WindowHandle<Traffic> = cx
        .open_window(size(px(1440.), px(900.)), move |window, cx| {
            cx.new(|cx| {
                let mut view = Traffic::new(engine, &opened_dir, window, cx);
                view.system.no_watchdog = true;
                view
            })
        })
        .unwrap();
    wait_for(&mut cx, window, "the certificate check", |view| {
        view.system.ca != CaState::Checking
    });
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(
                view.system.ca,
                CaState::NotTrusted,
                "a new CA is not trusted until it is installed"
            );
        })
        .unwrap();

    // The certificate, with a stand-in for certutil and a home of its own. It goes
    // in the browsers' stores, which the system's verifier does not read, and the
    // window must still call it trusted.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Setup, cx);
        })
        .unwrap();
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "13-setup");
    window
        .update(&mut cx, |view, _, cx| {
            view.install_certificate(Scope::User, cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the install", |view| {
        !matches!(view.system.ca, CaState::Installing)
    });
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(view.system.problem, None);
            assert_eq!(view.system.ca, CaState::Trusted);
            assert_eq!(
                view.system.ca_places.len(),
                1,
                "{:?}",
                view.system.ca_places
            );
            assert!(!view.system.ca_verified);
        })
        .unwrap();
    assert!(store.join("certutil-added").exists());
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "14-setup-trusted");

    // On, and what was there before comes back when it goes off.
    window
        .update(&mut cx, |view, _, cx| view.toggle_system_proxy(cx))
        .unwrap();
    wait_for(&mut cx, window, "the proxy to go on", |view| {
        matches!(view.system.proxy, ProxySwitch::On(_))
    });
    assert_eq!(mode(&store), "'manual'");
    let port = std::fs::read_to_string(store.join("org.gnome.system.proxy.https.port")).unwrap();
    assert_eq!(port, addr.port().to_string());
    save_shot(&mut cx, window.into(), "10-system-proxy");
    window
        .update(&mut cx, |view, _, cx| view.toggle_system_proxy(cx))
        .unwrap();
    wait_for(&mut cx, window, "the proxy to go off", |view| {
        matches!(view.system.proxy, ProxySwitch::Off)
    });
    assert_eq!(mode(&store), "'auto'", "as the user had it");

    // The settings screen: refuses what cannot work, writes what can.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
        })
        .unwrap();
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "11-settings");
    let problem = |page: &Option<SettingsPage>| page.as_ref().unwrap().problem.clone();
    for (key, text, says) in [
        ("session_max_mib", "many", "whole number"),
        ("session_max_mib", "1", "at least 64"),
        ("compress_level", "99", "1 to 19"),
    ] {
        window
            .update(&mut cx, |view, _, cx| {
                view.set_setting(key, text, cx);
                view.save_settings(cx);
                let problem = problem(&view.settings_page).expect("refused");
                assert!(problem.contains(says), "{problem}");
                view.reset_settings(cx);
            })
            .unwrap();
    }
    assert!(
        !data_dir.join("settings.toml").exists(),
        "nothing was written"
    );
    window
        .update(&mut cx, |view, _, cx| {
            view.set_setting("session_max_mib", "512", cx);
            view.save_settings(cx);
            assert_eq!(problem(&view.settings_page), None);
            assert!(view.settings_page.as_ref().unwrap().saved);
        })
        .unwrap();
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "12-settings-saved");
    let saved = StorageSettings::load(&data_dir.join("settings.toml")).unwrap();
    // The list of hosts applies at once, and is kept.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_no_decrypt("*.Bank.vn\n\npinned.dev", cx);
            view.save_settings(cx);
            let rules = view.engine.rules.read().unwrap();
            assert!(rules.leaves_alone("www.bank.vn:443"));
            assert!(rules.leaves_alone("pinned.dev"));
            assert!(
                !rules.leaves_alone("gateway.icloud.com"),
                "replaced the defaults"
            );
        })
        .unwrap();
    let kept = tracewing_proxy::rules::HostRules::load(&data_dir.join("decrypt.toml")).unwrap();
    assert_eq!(kept.no_decrypt, ["*.bank.vn", "pinned.dev"]);
    assert_eq!(saved.session_max_mib, 512);
    assert_eq!(saved.index_bodies, StorageSettings::default().index_bodies);
    drop(cx);

    // The app gone with the system proxy on: its watchdog puts it back. This is the
    // real program, as the system crate starts it.
    let crashed_dir = dir.join("crashed");
    std::fs::create_dir_all(&crashed_dir).unwrap();
    std::mem::forget(tracewing_system::take_over(&crashed_dir, addr, false).unwrap());
    assert_eq!(mode(&store), "'manual'");
    let mut watchdog = Command::new(env!("CARGO_BIN_EXE_tracewing"))
        .args(["watchdog", "--data-dir"])
        .arg(&crashed_dir)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = watchdog.stdin.take().unwrap();
    stdin.flush().unwrap();
    assert_eq!(mode(&store), "'manual'", "still on while the app is there");
    drop(stdin);
    assert!(watchdog.wait().unwrap().success());
    assert_eq!(mode(&store), "'auto'", "back after the crash");

    // And the same at login, with nothing left to do.
    let status = Command::new(env!("CARGO_BIN_EXE_tracewing"))
        .args(["sysproxy", "recover", "--data-dir"])
        .arg(&crashed_dir)
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(mode(&store), "'auto'");

    std::fs::remove_dir_all(&dir).unwrap();
}
