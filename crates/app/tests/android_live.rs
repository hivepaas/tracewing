//! S9, Android on a real device or emulator, through the window: Set up every
//! device, Undo, and a device a killed tracewing left pointed at the proxy
//! put back when tracewing starts again. The device has a proxy of its own
//! first, so that "put back" is more than "taken off".
//!
//! Only with a device named, as `TRACEWING_ANDROID_SERIAL=emulator-5580`, and
//! `adb` that can root it (the emulator's Google APIs images can); otherwise
//! it says so and does nothing. docs/design/e2e.md, S9.
//!
//! `TRACEWING_ANDROID_SERIAL=emulator-5580 cargo test --release -p tracewing --features headless --test android_live -- --nocapture`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    traffic::Traffic,
};
use tracewing_proxy::{ca::LocalCa, ca_key::KeyStore};
use tracewing_store::settings::StorageSettings;
use tracewing_system::android;

/// A proxy of the device's own, as a user on a company Wi-Fi has.
const THEIRS: &str = "10.0.2.2:3128";

fn proxy_of(adb: &Path, serial: &str) -> String {
    let out = Command::new(adb)
        .args(["-s", serial, "shell", "settings get global http_proxy"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn set_proxy(adb: &Path, serial: &str, proxy: &str) {
    let command = if proxy == "null" || proxy.is_empty() {
        "settings delete global http_proxy".to_owned()
    } else {
        format!("settings put global http_proxy {proxy}")
    };
    // adbd starts again after `root` and `unroot`, and a command sent while
    // it does is lost: a few tries, a second apart.
    let mut last = None;
    for _ in 0..5 {
        let out = Command::new(adb)
            .args(["-s", serial, "shell", &command])
            .output()
            .unwrap();
        if out.status.success() {
            return;
        }
        last = Some(out);
        std::thread::sleep(Duration::from_secs(1));
    }
    panic!("adb shell {command}: {last:?}");
}

/// The device's proxy as the test found it, written back however it ends.
struct PutBack {
    adb: PathBuf,
    serial: String,
    was: String,
}

impl Drop for PutBack {
    fn drop(&mut self) {
        set_proxy(&self.adb, &self.serial, &self.was);
        let _ = Command::new(&self.adb)
            .args(["-s", &self.serial, "reverse", "--remove-all"])
            .status();
    }
}

fn open(
    cx: &mut HeadlessAppContext,
    data_dir: &Path,
    ports: std::ops::Range<u16>,
) -> WindowHandle<Traffic> {
    let engine = Engine::start(EngineOptions {
        data_dir: data_dir.to_owned(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports,
        extra_roots: Vec::new(),
        key_store: KeyStore::File,
        find_apps: false,
    })
    .unwrap();
    while !data_dir.join("ca.pem").exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let data_dir = data_dir.to_owned();
    cx.open_window(size(px(1440.), px(900.)), move |window, cx| {
        cx.new(|cx| Traffic::new(engine, &data_dir, window, cx))
    })
    .unwrap()
}

/// Waits for `done`, with the window let run, or fails saying what.
fn wait_for(
    cx: &mut HeadlessAppContext,
    what: &str,
    limit: Duration,
    mut done: impl FnMut(&mut HeadlessAppContext) -> bool,
) {
    let deadline = Instant::now() + limit;
    while !done(cx) {
        assert!(Instant::now() < deadline, "waited {limit:?} for {what}");
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn a_device_is_set_up_undone_and_put_back_after_a_kill() {
    let Ok(serial) = std::env::var("TRACEWING_ANDROID_SERIAL") else {
        eprintln!("skipped: no device named: TRACEWING_ANDROID_SERIAL");
        return;
    };
    let _ = tracewing_proxy::crypto_provider().install_default();
    let adb = android::find_adb().expect("adb");
    let _put_back = PutBack {
        adb: adb.clone(),
        serial: serial.clone(),
        was: proxy_of(&adb, &serial),
    };
    set_proxy(&adb, &serial, THEIRS);

    let dir = std::env::temp_dir().join(format!("tracewing-android-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let text = Arc::new(gpui_wgpu::CosmicTextSystem::new(tracewing::FONT));
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(tracewing::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(tracewing::load_fonts);

    // Set up every device, as the button does: the device goes through the
    // proxy and its own setting is written down.
    let first = dir.join("first");
    std::fs::create_dir_all(&first).unwrap();
    let window = open(&mut cx, &first, 20_600..20_610);
    let port = window
        .update(&mut cx, |view, _, cx| {
            view.set_up_android_all(cx);
            view.engine.addr.port()
        })
        .unwrap();
    wait_for(
        &mut cx,
        "the device set up",
        Duration::from_secs(120),
        |cx| {
            window
                .update(cx, |view, _, _| {
                    let state = &view.system.android;
                    !state.looking && state.working.is_none() && state.set.contains_key(&serial)
                })
                .unwrap()
        },
    );
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(view.system.android.problem, None);
        })
        .unwrap();
    assert_eq!(proxy_of(&adb, &serial), format!("127.0.0.1:{port}"));
    assert_eq!(
        android::read_journal(&first)
            .iter()
            .map(|e| e.previous.clone())
            .collect::<Vec<_>>(),
        [Some(THEIRS.to_owned())],
        "what it had before, written down"
    );

    // Undo, as the button by it does: its own proxy is back.
    window
        .update(&mut cx, |view, _, cx| view.undo_android(serial.clone(), cx))
        .unwrap();
    wait_for(
        &mut cx,
        "the device put back",
        Duration::from_secs(60),
        |cx| {
            window
                .update(cx, |view, _, _| {
                    let state = &view.system.android;
                    state.working.is_none() && !state.set.contains_key(&serial)
                })
                .unwrap()
        },
    );
    assert_eq!(proxy_of(&adb, &serial), THEIRS);
    assert!(android::read_journal(&first).is_empty());

    // A tracewing killed while the device was set up: the device points at
    // a port nobody will answer, and the journal is all that remembers. A
    // tracewing started again on that data folder puts it back by itself.
    let killed = dir.join("killed");
    std::fs::create_dir_all(&killed).unwrap();
    let ca = LocalCa::load_or_create(&killed, tracewing_proxy::crypto_provider(), KeyStore::File)
        .unwrap();
    let der = ca.cert_der().unwrap();
    android::set_up(&adb, &serial, 20_699, ca.cert_path(), &der, &killed).unwrap();
    assert_eq!(proxy_of(&adb, &serial), "127.0.0.1:20699");
    let again = open(&mut cx, &killed, 20_610..20_620);
    wait_for(
        &mut cx,
        "the started tracewing to put the device back",
        Duration::from_secs(60),
        |_| proxy_of(&adb, &serial) == THEIRS,
    );
    assert!(
        android::read_journal(&killed).is_empty(),
        "nothing left to put back"
    );

    for window in [window, again] {
        let _ = window.update(&mut cx, |_, window, _| window.remove_window());
    }
    std::fs::remove_dir_all(&dir).ok();
}
