//! S4 on macOS, the disk image: tracewing.app copied out of it into
//! /Applications as a person drags it there, the program inside it run to
//! record HTTPS and to take the system proxy, which arms the login agent as
//! the installed program; then `tracewing uninstall` and the app moved to the
//! bin, and nothing of either is left.
//!
//! The image is `TRACEWING_E2E_DMG`, which `scripts/e2e_macos.sh --packages`
//! builds and hands in. A Mac that already has a tracewing.app in
//! /Applications keeps it: the test says so and does nothing.
//!
//! The client here trusts tracewing's CA from its own file (`--cacert`):
//! trusting it in the keychain asks for a password, and S1 is what checks
//! that.
//!
//! Installs an app, and changes this Mac's proxy setting. See
//! `tracewing_e2e`.
#![cfg(target_os = "macos")]

use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use tracewing_e2e::{
    Capture, Origin, desktop, har_of, helper::Helper, only_where_allowed, said, scratch,
    tracewing_at, wait_for,
};
use tracewing_proxy::ca_key::KeyStore;

const INSTALLED: &str = "/Applications/tracewing.app";

/// The login agent tracewing arms while it holds the proxy.
fn agent() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap())
        .join("Library/LaunchAgents/com.hivepaas.tracewing.proxy-recovery.plist")
}

/// The image, mounted until dropped.
struct Mounted(PathBuf);

impl Drop for Mounted {
    fn drop(&mut self) {
        let _ = Command::new("/usr/bin/hdiutil")
            .args(["detach", "-quiet", "-force"])
            .arg(&self.0)
            .output();
    }
}

/// The app in /Applications, taken out however the test ends.
struct Installed;

impl Drop for Installed {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(INSTALLED);
    }
}

fn run(program: &str, args: &[&str]) {
    let out = Command::new(program).args(args).output().unwrap();
    assert!(out.status.success(), "{program} {args:?}: {}", said(&out));
}

#[test]
fn the_disk_image_installs_records_https_and_leaves_nothing_behind() {
    only_where_allowed!();
    let Some(dmg) = std::env::var_os("TRACEWING_E2E_DMG").filter(|f| !f.is_empty()) else {
        eprintln!("skipped: TRACEWING_E2E_DMG names no disk image (e2e_macos.sh --packages)");
        return;
    };
    if Path::new(INSTALLED).exists() {
        eprintln!("skipped: {INSTALLED} is there already, and it is not this test's to replace");
        return;
    }
    let dir = scratch("dmg");

    eprintln!("dmg: opening {}", Path::new(&dmg).display());
    // Opened as a person opens it, and the app copied out, as dragging it
    // onto /Applications does.
    let mount = dir.join("mount");
    std::fs::create_dir_all(&mount).unwrap();
    run(
        "/usr/bin/hdiutil",
        &[
            "attach",
            "-nobrowse",
            "-quiet",
            "-mountpoint",
            mount.to_str().unwrap(),
            dmg.to_str().unwrap(),
        ],
    );
    let mounted = Mounted(mount.clone());
    assert!(
        mount.join("Applications").is_symlink(),
        "no /Applications to drag it onto"
    );
    let _installed = Installed;
    run(
        "/usr/bin/ditto",
        &[mount.join("tracewing.app").to_str().unwrap(), INSTALLED],
    );
    drop(mounted);
    eprintln!("dmg: installed in {INSTALLED}");
    let program = Path::new(INSTALLED).join("Contents/MacOS/tracewing");
    let out = tracewing_at(&program).arg("--version").output().unwrap();
    assert!(out.status.success(), "{}", said(&out));

    eprintln!("dmg: recording HTTPS");
    // HTTPS through it, both bodies in the HAR it writes.
    let _put_back = desktop::the_users_own();
    let before = desktop::setting();
    let origin = Origin::start(&dir);
    let data = dir.join("data");
    let capture = Capture::start_from(&program, &data, &origin, KeyStore::File);
    let marker = format!("sent-through-the-app-{}", std::process::id());
    let url = format!("https://localhost:{}/echo", origin.port);
    let out = Command::new("curl")
        .args(["--silent", "--show-error", "--max-time", "10", "--proxy"])
        .arg(format!("http://127.0.0.1:{}", capture.port))
        .arg("--cacert")
        .arg(capture.ca_pem())
        .args(["--data", &marker, &url])
        .output()
        .unwrap();
    assert!(out.status.success(), "curl through it: {}", said(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(&format!("body={marker}")),
        "{}",
        said(&out)
    );
    let mut har = String::new();
    wait_for("the request in the HAR", Duration::from_secs(10), || {
        har = har_of(&program, &data);
        har.contains(&url)
    });
    assert!(
        har.matches(marker.as_str()).count() >= 2,
        "the HAR has not got what was sent and answered: {har}"
    );

    eprintln!("dmg: taking the system proxy");
    // The system proxy, taken by the installed app, which arms the login
    // agent as itself; let go, the agent goes and the setting is back.
    let holder = Helper::system_proxy_from(&program, &data, capture.port, true);
    assert!(desktop::names_tracewing(&desktop::setting(), capture.port));
    let armed = std::fs::read_to_string(agent()).expect("no login agent armed");
    assert!(
        armed.contains(INSTALLED),
        "the agent does not run the installed app: {armed}"
    );
    assert!(holder.let_go().success());
    assert_eq!(desktop::setting(), before);
    assert!(!agent().exists(), "the login agent stayed");
    drop(capture);

    eprintln!("dmg: uninstalling");
    // Then as before removing it: uninstall, and the app to the bin.
    let out = tracewing_at(&program)
        .arg("uninstall")
        .arg("--data-dir")
        .arg(&data)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", said(&out));
    std::fs::remove_dir_all(INSTALLED).unwrap();
    assert!(!Path::new(INSTALLED).exists());
    assert!(!agent().exists());
    assert_eq!(desktop::setting(), before);
    let left = Command::new("/usr/bin/pgrep")
        .args(["-f", INSTALLED])
        .output()
        .unwrap();
    assert!(
        !left.status.success(),
        "something of the app still runs: {}",
        said(&left)
    );
    std::fs::remove_dir_all(&dir).ok();
}
