//! Telling that a newer tracewing is out, in the window, from a signed list
//! served on this machine: looked for, said in About and on the title bar,
//! downloaded, checked, installed as tracewing quits, and skipped. And a list
//! signed by keys this build does not trust, refused and said.
//!
//! The list is `crates/update/testdata/release.signed.json`, signed by the
//! signing tool with test keys (`testdata/make.sh`), which offers `v99.0.0`.
//! How this copy was installed is said for it (`set_test_install`), so each
//! kind of copy is drawn from Linux. Each state is kept in `target/shots`.
//!
//! `cargo test --release -p tracewing --features headless --test updates`
//!
//! This file claims `ports: 20_710..20_720` for the proxy it starts, the next
//! free ten above `tests/local_live.rs`.

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tracewing::{
    engine::{Engine, EngineOptions},
    layout::SettingsSection,
    traffic::{Screen, Traffic},
    updates::{Badge, Stage},
};
use tracewing_store::settings::StorageSettings;
use tracewing_update::{Install, PackageTool, Version};

const TESTDATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../update/testdata");
const TEST_ED: &str = include_str!("../../update/testdata/test-ed.pub.pem");
const TEST_ML: &str = include_str!("../../update/testdata/test-ml.pub.pem");
/// What the test list's Linux release is, byte for byte.
const RELEASE: &[u8] = include_bytes!("../../update/testdata/v99.0.0.AppImage");

/// Serves the signed list and the release's file, as GitHub would, for as
/// long as the test runs. Its address.
fn serve() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let list = std::fs::read(Path::new(TESTDATA).join("release.signed.json")).unwrap();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { continue };
            let mut head = [0u8; 2048];
            let n = conn.read(&mut head).unwrap_or(0);
            let head = String::from_utf8_lossy(&head[..n]);
            let path = head.split_whitespace().nth(1).unwrap_or("");
            let body: Option<&[u8]> = match path {
                "/release.signed.json" => Some(&list),
                "/v99.0.0/tracewing_99.0.0_linux_x86_64.AppImage" => Some(RELEASE),
                _ => None,
            };
            let answer = match body {
                Some(body) => {
                    let mut a = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .into_bytes();
                    a.extend_from_slice(body);
                    a
                }
                None => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_vec(),
            };
            let _ = conn.write_all(&answer);
        }
    });
    format!("http://{addr}")
}

fn shot(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, name: &str) {
    cx.run_until_parked();
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shots");
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
        ports: 20_710..20_720,
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

/// Reads the view.
fn read<T>(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    f: impl FnOnce(&Traffic) -> T,
) -> T {
    window.update(cx, |view, _, _| f(view)).unwrap()
}

/// Waits for what goes on off the window's thread (the list read, the file
/// downloaded) to have come back.
fn wait_for(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    what: &str,
    done: impl Fn(&Traffic) -> bool,
) {
    for _ in 0..200 {
        cx.run_until_parked();
        if read(cx, window, &done) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("waited 10 s for {what}");
}

/// Looks now, as Check now does, and waits for the answer.
fn check(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>) {
    window
        .update(cx, |view, _, cx| view.check_updates(cx))
        .unwrap();
    wait_for(cx, window, "the look to end", |view| !view.updates.checking);
}

#[test]
fn a_newer_release_is_told_of_downloaded_and_installed_as_tracewing_quits() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-updates-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let at = serve();
    tracewing_update::set_test_source(
        &format!("{at}/release.signed.json"),
        &[("test-ed", TEST_ED), ("test-ml", TEST_ML)],
    );
    tracewing_update::set_test_files(&at);
    tracewing_update::set_test_install(Install::Manual);
    let (mut cx, window) = open(&dir);
    let cx = &mut cx;
    let v99 = Version::parse("v99.0.0").unwrap();

    // About, before any look: it says so.
    window
        .update(cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
            view.show_settings_section(SettingsSection::About, cx);
        })
        .unwrap();
    shot(cx, window, "updates-1-not-looked");

    // A copy put somewhere by hand: told of v99.0.0, and pointed at its page.
    check(cx, window);
    assert_eq!(read(cx, window, |v| v.updates.problem.clone()), None);
    assert_eq!(read(cx, window, |v| v.updates.offered()), Some(v99.clone()));
    assert!(read(cx, window, |v| v.updates.state.checked_at.is_some()));
    shot(cx, window, "updates-2-out-by-hand");
    // What it found is kept, for the next start.
    let kept = std::fs::read_to_string(dir.join("updates.json")).unwrap();
    assert!(kept.contains("v99.0.0"), "{kept}");

    // And on the title bar, while looking is on, as packwing says it: a
    // badge, which leads to About. Looking turned off, it says nothing.
    window
        .update(cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    assert_eq!(
        read(cx, window, |v| v.update_badge_says()),
        Some(Badge::Out(v99.clone()))
    );
    shot(cx, window, "updates-3-badge");
    window
        .update(cx, |view, _, _| view.engine.settings.check_updates = false)
        .unwrap();
    assert_eq!(read(cx, window, |v| v.update_badge_says()), None);
    window
        .update(cx, |view, _, _| view.engine.settings.check_updates = true)
        .unwrap();

    // A copy from the release's .deb: the commands that install the next one,
    // with the list's SHA-256 for it.
    tracewing_update::set_test_install(Install::Package(PackageTool::Apt));
    check(cx, window);
    let package = read(cx, window, |v| v.updates.state.package.clone());
    assert_eq!(
        package,
        Some((
            "tracewing_99.0.0_amd64.deb".to_owned(),
            "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae".to_owned()
        ))
    );
    window
        .update(cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
            view.show_settings_section(SettingsSection::About, cx);
        })
        .unwrap();
    shot(cx, window, "updates-4-package");

    // A copy the app can replace: downloaded, checked against the list's
    // SHA-256, ready, and put in place of this one as tracewing quits.
    let image = dir.join("tracewing.AppImage");
    std::fs::write(&image, b"the copy running").unwrap();
    tracewing_update::set_test_install(Install::AppImage(image.clone()));
    shot(cx, window, "updates-5-can-replace");
    window
        .update(cx, |view, _, cx| view.get_update(cx))
        .unwrap();
    wait_for(cx, window, "the download", |view| {
        !matches!(view.updates.stage, Stage::Downloading(..))
    });
    assert_eq!(
        read(cx, window, |v| v.updates.stage.clone()),
        Stage::Ready(v99.clone())
    );
    // Ready: the badge says so, whether or not looking is on.
    assert_eq!(
        read(cx, window, |v| v.update_badge_says()),
        Some(Badge::Ready(v99.clone()))
    );
    shot(cx, window, "updates-6-ready");
    let staged = read(cx, window, |v| v.updates.state.staged.clone()).expect("staged");
    assert_eq!(std::fs::read(&staged.1).unwrap(), RELEASE);
    window
        .update(cx, |view, _, _| view.install_staged())
        .unwrap();
    assert_eq!(
        std::fs::read(&image).unwrap(),
        RELEASE,
        "the copy is the release now"
    );
    assert!(!Path::new(&staged.1).exists(), "and its download is gone");
    assert_eq!(read(cx, window, |v| v.updates.state.staged.clone()), None);

    // Skipped: not offered again, in About or on the title bar.
    window
        .update(cx, |view, _, cx| {
            view.updates.stage = Stage::Idle;
            view.skip_update(cx);
        })
        .unwrap();
    assert_eq!(read(cx, window, |v| v.updates.offered()), None);
    assert_eq!(read(cx, window, |v| v.update_badge_says()), None);
    shot(cx, window, "updates-7-skipped");

    // A list this build does not trust the keys of: refused, and said.
    tracewing_update::set_test_source(
        &format!("{at}/release.signed.json"),
        &[("other-ed", TEST_ED), ("other-ml", TEST_ML)],
    );
    window
        .update(cx, |view, _, _| view.updates.state.skipped = None)
        .unwrap();
    window
        .update(cx, |view, _, cx| {
            view.updates.state.offered = None;
            view.check_updates(cx);
        })
        .unwrap();
    wait_for(cx, window, "the look to end", |view| !view.updates.checking);
    let problem = read(cx, window, |v| v.updates.problem.clone()).expect("refused");
    assert!(problem.contains("no valid"), "{problem}");
    assert_eq!(read(cx, window, |v| v.updates.offered()), None);
    shot(cx, window, "updates-8-refused");
    let _ = std::fs::remove_dir_all(&dir);
}
