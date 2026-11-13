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
use tracewing_system::{BrowserTrust, Environment, Scope};

/// A `gsettings` that keeps its values in files in a folder.
/// A `certutil` that keeps the certificate as a file in each database, and says
/// in the store that it added one. `-N` makes a database.
const FAKE_CERTUTIL: &str = r#"#!/bin/sh
store="$FAKE_GSETTINGS_DIR"
case "$1" in
-N) db=$(echo "$4" | sed 's/^sql://'); touch "$db/cert9.db" ;;
-A) db=$(echo "$3" | sed 's/^sql://'); touch "$db/tracewing-ca" "$store/certutil-added" ;;
-L) db=$(echo "$3" | sed 's/^sql://'); test -e "$db/tracewing-ca" ;;
-D) db=$(echo "$3" | sed 's/^sql://'); rm "$db/tracewing-ca" ;;
esac
"#;

/// A browser that writes down how it was started, for the Browsers card to find.
const FAKE_BROWSER: &str =
    "#!/bin/sh\necho \"$@\" > \"$FAKE_GSETTINGS_DIR/$(basename \"$0\").args\"\n";

fn browser_trust(view: &Traffic, name: &str) -> Option<BrowserTrust> {
    view.system
        .browsers
        .iter()
        .find(|(browser, _)| browser.name == name)
        .map(|(_, trust)| *trust)
}

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
    for name in ["google-chrome", "firefox"] {
        let browser = bin.join(name);
        std::fs::write(&browser, FAKE_BROWSER).unwrap();
        std::fs::set_permissions(&browser, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let home = dir.join("home");
    // Firefox, with the one profile it made as it first started.
    let profiles = home.join(".mozilla/firefox");
    std::fs::create_dir_all(profiles.join("one.default")).unwrap();
    std::fs::write(profiles.join("one.default/cert9.db"), "").unwrap();
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
        key_store: Default::default(),
        find_apps: true,
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
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
                2,
                "Chrome's database and Firefox's profile: {:?}",
                view.system.ca_places
            );
            assert!(!view.system.ca_verified);
            let both = Some(BrowserTrust::Own { holding: 1, of: 1 });
            assert_eq!(browser_trust(view, "Chrome"), both);
            assert_eq!(browser_trust(view, "Firefox"), both);
        })
        .unwrap();
    assert!(store.join("certutil-added").exists());
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "14-setup-trusted");

    // A Firefox profile made since does not trust it, until the card's Install.
    std::fs::create_dir_all(profiles.join("two.work")).unwrap();
    std::fs::write(profiles.join("two.work/cert9.db"), "").unwrap();
    window
        .update(&mut cx, |view, _, cx| view.start_ca_check(cx))
        .unwrap();
    wait_for(&mut cx, window, "the new profile", |view| {
        browser_trust(view, "Firefox") == Some(BrowserTrust::Own { holding: 1, of: 2 })
    });
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "83-setup-browsers");
    window
        .update(&mut cx, |view, _, cx| {
            view.install_certificate(Scope::User, cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the profile to trust it", |view| {
        browser_trust(view, "Firefox") == Some(BrowserTrust::Own { holding: 2, of: 2 })
    });

    // Each on its own: Chrome with the proxy as a flag, Firefox with a profile
    // that holds the proxy and trusts the CA.
    let firefox = window
        .update(&mut cx, |view, _, cx| {
            view.open_browser("Chrome", cx);
            view.open_browser("Firefox", cx);
            let (firefox, _) = view
                .system
                .browsers
                .iter()
                .find(|(b, _)| b.name == "Firefox")
                .unwrap();
            firefox.profile.clone()
        })
        .unwrap();
    let args = |name: &str| std::fs::read_to_string(store.join(format!("{name}.args")));
    wait_for(&mut cx, window, "the browsers to start", |view| {
        view.system.problem.is_some() || args("google-chrome").is_ok() && args("firefox").is_ok()
    });
    window
        .update(&mut cx, |view, _, _| assert_eq!(view.system.problem, None))
        .unwrap();
    let chrome_args = args("google-chrome").unwrap();
    assert!(
        chrome_args.contains(&format!("--proxy-server=http://{addr}")),
        "{chrome_args}"
    );
    assert!(
        !chrome_args.contains("spki"),
        "Chrome trusts the CA: {chrome_args}"
    );
    assert_eq!(
        args("firefox").unwrap().trim(),
        format!(
            "--profile {} --no-remote --new-instance https://tracewing.test/",
            firefox.display()
        )
    );
    let prefs = std::fs::read_to_string(firefox.join("user.js")).unwrap();
    assert!(
        prefs.contains(&format!("\"network.proxy.ssl_port\", {});", addr.port())),
        "{prefs}"
    );
    assert!(
        firefox.join("tracewing-ca").exists(),
        "the profile trusts the CA"
    );

    // Taken out of every store, after asking; tracewing's own Firefox profile
    // is not one of the user's.
    window
        .update(&mut cx, |view, _, cx| {
            view.ask_to_remove_certificate(true, cx)
        })
        .unwrap();
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "84-setup-remove");
    window
        .update(&mut cx, |view, _, cx| view.remove_certificate(cx))
        .unwrap();
    wait_for(&mut cx, window, "the removal", |view| {
        view.system.ca != CaState::Removing
    });
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(view.system.problem, None);
            assert_eq!(view.system.ca, CaState::NotTrusted);
            assert!(
                view.system.ca_places.is_empty(),
                "{:?}",
                view.system.ca_places
            );
            assert!(!view.system.confirm_remove);
            let none = Some(BrowserTrust::Own { holding: 0, of: 2 });
            assert_eq!(browser_trust(view, "Firefox"), none);
        })
        .unwrap();
    assert!(!home.join(".pki/nssdb/tracewing-ca").exists());

    // What a machine lacks shows where it matters: certutil in step 1, a
    // desktop with no system proxy in step 2.
    window
        .update(&mut cx, |view, _, cx| {
            view.system.environment = Environment {
                no_system_proxy: Some("sway".into()),
                no_certutil: Some(Some("sudo apt install libnss3-tools")),
                old_node: Some("22.17.1".into()),
            };
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "85-setup-lacking");
    // Below the browsers: a terminal, the line for one of the user's own, and
    // the Node.js that would go around both.
    let scroll = |cx: &mut HeadlessAppContext, by: f32| {
        cx.update_window(window.into(), |_, window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position: gpui::point(px(900.), px(500.)),
                    delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-by))),
                    ..Default::default()
                }),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
    };
    scroll(&mut cx, 700.);
    save_shot(&mut cx, window.into(), "86-setup-terminal");
    scroll(&mut cx, -700.);
    window
        .update(&mut cx, |view, _, cx| {
            view.install_certificate(Scope::User, cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the install again", |view| {
        view.system.ca == CaState::Trusted
    });
    window
        .update(&mut cx, |view, _, _| assert_eq!(view.system.problem, None))
        .unwrap();

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

    // The settings screen: refuses what cannot work, and writes what can as
    // it is changed.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
            view.show_settings_section(tracewing::layout::SettingsSection::Storage, cx);
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
                view.settings_page = None;
                view.open_settings(cx);
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
            view.toggle_setting("hide_http3", cx);
            view.toggle_setting("other_devices", cx);
            view.toggle_setting("os_roots", cx);
            let page = view.settings_page.as_ref().unwrap();
            assert_eq!(
                page.waiting(&view.engine.settings),
                ["os_roots", "session_max_mib"],
                "the others apply at once"
            );
            assert!(view.engine.other_devices.is_on());
            assert!(!view.engine.live.hide_http3());
        })
        .unwrap();
    let written = StorageSettings::load(&data_dir.join("settings.toml")).unwrap();
    assert_eq!(written.session_max_mib, 512);
    assert!(written.other_devices);
    window
        .update(&mut cx, |view, _, cx| {
            view.show_settings_section(tracewing::layout::SettingsSection::Network, cx)
        })
        .unwrap();
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "12-settings-saved");
    // Back to defaults, for the part open only, is written at once, and can
    // be undone.
    window
        .update(&mut cx, |view, _, cx| {
            view.show_settings_section(tracewing::layout::SettingsSection::Storage, cx);
            view.reset_settings(cx);
            let reset = StorageSettings::load(&data_dir.join("settings.toml")).unwrap();
            assert_eq!(
                reset.session_max_mib,
                StorageSettings::default().session_max_mib
            );
            assert!(reset.other_devices, "another part's is left as it was");
            view.undo_reset_settings(cx);
            view.show_settings_section(tracewing::layout::SettingsSection::Network, cx);
            view.reset_settings(cx);
            assert!(!view.engine.other_devices.is_on());
            view.undo_reset_settings(cx);
            assert!(!view.engine.live.hide_http3(), "undone at once too");
            view.toggle_setting("hide_http3", cx);
            view.toggle_setting("other_devices", cx);
            view.toggle_setting("os_roots", cx);
        })
        .unwrap();
    let written = StorageSettings::load(&data_dir.join("settings.toml")).unwrap();
    assert_eq!(written.session_max_mib, 512, "undone");
    assert_eq!(written.hide_http3, StorageSettings::default().hide_http3);
    // Left with a value that cannot work, the field goes back to what is
    // written, and the footer says why.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_setting("session_max_mib", "1", cx);
            view.set_screen(Screen::Traffic, cx);
            let left = view.settings_left.clone().expect("said");
            assert!(left.contains("at least 64"), "{left}");
            let page = view.settings_page.as_ref().unwrap();
            assert_eq!(page.problem, None);
            assert_eq!(page.waiting(&view.engine.settings), ["session_max_mib"]);
            view.set_screen(Screen::Settings, cx);
            assert_eq!(view.settings_left, None);
        })
        .unwrap();
    cx.run_until_parked();
    // A field is written when it is left, not as it is typed in. Focus only
    // moves in a window that is active, as the user's is.
    cx.update_window(window.into(), |_, window, _| window.activate_window())
        .unwrap();
    cx.run_until_parked();
    let body_max = || {
        StorageSettings::load(&data_dir.join("settings.toml"))
            .unwrap()
            .body_max_mib
    };
    window
        .update(&mut cx, |view, window, cx| {
            view.focus_setting("body_max_mib", window, cx);
            view.set_setting("body_max_mib", "8", cx);
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(body_max(), StorageSettings::default().body_max_mib);
    window
        .update(&mut cx, |view, window, cx| {
            view.focus_setting("view_max_mib", window, cx)
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(body_max(), 8);
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(view.engine.live.body_max(), 8 << 20, "applied at once")
        })
        .unwrap();
    assert_eq!(
        StorageSettings::load(&data_dir.join("settings.toml"))
            .unwrap()
            .session_max_mib,
        512
    );
    // A language chosen is used at once and kept; the system's is the default.
    window
        .update(&mut cx, |view, window, cx| {
            view.set_language(Some("vi"), window, cx);
            assert_eq!(tracewing_i18n::language(), "vi");
            view.set_language(None, window, cx);
        })
        .unwrap();
    assert_eq!(
        tracewing::layout::Layout::load(&data_dir.join("layout.toml")).language,
        None
    );
    window
        .update(&mut cx, |view, window, cx| {
            view.set_language(Some("vi"), window, cx)
        })
        .unwrap();
    assert_eq!(
        tracewing::layout::Layout::load(&data_dir.join("layout.toml"))
            .language
            .as_deref(),
        Some("vi")
    );
    // In Vietnamese, the window says its words in it.
    window
        .update(&mut cx, |view, _, cx| {
            view.show_settings_section(tracewing::layout::SettingsSection::General, cx)
        })
        .unwrap();
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "72-settings-vi");
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Setup, cx))
        .unwrap();
    cx.run_until_parked();
    save_shot(&mut cx, window.into(), "73-setup-vi");
    assert_eq!(tracewing_i18n::tr!("Settings"), "Cài đặt");
    // Long words, Cyrillic and Japanese fit the same window.
    for (tag, shot) in [
        ("de", "74-setup-de"),
        ("ru", "75-setup-ru"),
        ("ja", "76-setup-ja"),
    ] {
        window
            .update(&mut cx, |view, window, cx| {
                view.set_language(Some(tag), window, cx)
            })
            .unwrap();
        cx.run_until_parked();
        save_shot(&mut cx, window.into(), shot);
    }
    window
        .update(&mut cx, |view, window, cx| {
            view.set_language(Some("en"), window, cx);
            view.set_screen(Screen::Settings, cx);
        })
        .unwrap();
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
