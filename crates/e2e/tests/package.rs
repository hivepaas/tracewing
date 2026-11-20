//! S4, the installed package: installed as a user installs it, the program it
//! puts on the computer records HTTPS and stands behind the system proxy it
//! takes; then `tracewing uninstall` and the package removed, and nothing of
//! either is left: no store trusts the CA, the proxy setting is the user's,
//! nothing is armed to run, and no file of the package's is there.
//!
//! The package is `TRACEWING_E2E_PACKAGE`, which `scripts/e2e_linux.sh`
//! builds and hands in: the .deb in the Ubuntu container, the .rpm in the
//! Fedora one.
//!
//! Installs and removes a package, and changes this computer's trust stores
//! and proxy setting. See `tracewing_e2e`.
#![cfg(target_os = "linux")]

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tracewing_e2e::{
    Capture, Origin, curl_through_with, desktop, har_of, helper::Helper, only_where_allowed, said,
    scratch, tracewing_at,
};
use tracewing_proxy::ca_key::KeyStore;
use tracewing_system::{Scope, ca_install, ca_locations, ca_remove};

/// Where the package puts the program.
const INSTALLED: &str = "/usr/bin/tracewing";

/// Every file the package holds: none of them is there before, and none
/// after.
const FILES: &[&str] = &[
    "/usr/bin/tracewing",
    "/usr/share/applications/tracewing.desktop",
    "/usr/share/icons/hicolor/scalable/apps/tracewing.svg",
];

/// Run as root, as a user of a desktop does: through pkexec.
fn as_root(args: &[&str]) -> Output {
    Command::new("pkexec").args(args).output().unwrap()
}

#[derive(Clone, Copy)]
enum Kind {
    Deb,
    Rpm,
}

/// The package, installed until removed: if the test fails first, it is
/// removed when the test ends.
struct Package {
    kind: Kind,
    installed: bool,
}

impl Package {
    fn install(file: &Path) -> Self {
        let kind = match file.extension().and_then(|ext| ext.to_str()) {
            Some("deb") => Kind::Deb,
            Some("rpm") => Kind::Rpm,
            _ => panic!("not a .deb or an .rpm: {}", file.display()),
        };
        let file = file.to_str().unwrap();
        // From the file alone, with what it depends on already here: a
        // dependency the package names and this system lacks fails it.
        let out = match kind {
            Kind::Deb => as_root(&["dpkg", "--install", file]),
            Kind::Rpm => as_root(&["rpm", "--install", file]),
        };
        assert!(out.status.success(), "install {file}: {}", said(&out));
        Package {
            kind,
            installed: true,
        }
    }

    /// Whether the system's package database knows it.
    fn known(kind: Kind) -> bool {
        let out = match kind {
            Kind::Deb => Command::new("dpkg-query")
                .args(["--show", "--showformat=${db:Status-Status}", "tracewing"])
                .output()
                .unwrap(),
            Kind::Rpm => Command::new("rpm")
                .args(["--query", "tracewing"])
                .output()
                .unwrap(),
        };
        // dpkg remembers a removed package's name, as "not-installed".
        out.status.success() && String::from_utf8_lossy(&out.stdout).trim() != "not-installed"
    }

    fn remove(&mut self) -> Output {
        self.installed = false;
        match self.kind {
            Kind::Deb => as_root(&["dpkg", "--purge", "tracewing"]),
            Kind::Rpm => as_root(&["rpm", "--erase", "tracewing"]),
        }
    }
}

impl Drop for Package {
    fn drop(&mut self) {
        if self.installed {
            let _ = self.remove();
        }
    }
}

/// Takes the CA out of every store, however the test ends.
struct Untrust(PathBuf);

impl Drop for Untrust {
    fn drop(&mut self) {
        let _ = ca_remove(&self.0);
    }
}

/// A program called `tracewing` on the PATH.
fn on_path() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join("tracewing"))
        .find(|path| path.is_file())
}

/// What tracewing arms to put the proxy back at the next login.
fn autostart() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".config"));
    config.join("autostart/tracewing-proxy-recovery.desktop")
}

#[test]
fn the_package_records_https_and_leaves_nothing_behind() {
    only_where_allowed!();
    // scripts/e2e_linux.sh always hands one in.
    let Some(file) = std::env::var_os("TRACEWING_E2E_PACKAGE").filter(|file| !file.is_empty())
    else {
        eprintln!("skipped: TRACEWING_E2E_PACKAGE names no .deb or .rpm to install");
        return;
    };
    let file = PathBuf::from(file);
    let program = Path::new(INSTALLED);
    for path in FILES {
        assert!(
            !Path::new(path).exists(),
            "{path} is there before installing"
        );
    }

    let mut package = Package::install(&file);
    for path in FILES {
        assert!(Path::new(path).exists(), "installed, and no {path}");
    }
    // Where Fedora has merged /usr/sbin into /usr/bin, the PATH may name
    // the same folder twice, sbin first.
    assert_eq!(
        on_path().map(|found| found.canonicalize().unwrap()),
        Some(program.canonicalize().unwrap()),
        "not on the PATH"
    );
    let out = tracewing_at(program).arg("--version").output().unwrap();
    assert!(out.status.success(), "{}", said(&out));

    let _put_back = desktop::the_users_own();
    let before = desktop::setting();
    let dir = scratch("package");
    let origin = Origin::start(&dir);
    let data = dir.join("data");
    let capture = Capture::start_from(program, &data, &origin, KeyStore::File);
    let pem = capture.ca_pem();
    let _untrust = Untrust(pem.clone());
    ca_install(&pem, Scope::User).unwrap();
    ca_install(&pem, Scope::Machine).unwrap();

    // HTTPS through it, by a client that trusts only what the system trusts,
    // and what was said both ways in the HAR it writes.
    let marker = format!("sent-through-the-package-{}", std::process::id());
    let url = format!("https://localhost:{}/echo", origin.port);
    let out = curl_through_with(capture.port, &["--data", &marker, &url]);
    assert!(out.status.success(), "curl through it: {}", said(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(&format!("body={marker}")),
        "{}",
        said(&out)
    );
    let mut har = String::new();
    tracewing_e2e::wait_for(
        "the request in the HAR",
        std::time::Duration::from_secs(10),
        || {
            har = har_of(program, &data);
            har.contains(&url)
        },
    );
    // The request's body and the answer's, which held it too: decrypted.
    assert!(
        har.matches(marker.as_str()).count() >= 2,
        "the HAR has not got what was sent and answered: {har}"
    );

    // The system proxy, taken by the installed program, which arms what
    // puts it back as itself; then killed, so that it is left for
    // `tracewing uninstall` to put back.
    let holder = Helper::system_proxy_from(program, &data, capture.port, false);
    assert!(desktop::names_tracewing(&desktop::setting(), capture.port));
    let armed = std::fs::read_to_string(autostart()).expect("nothing armed for the next login");
    assert!(
        armed.contains(INSTALLED),
        "what is armed does not run the installed program: {armed}"
    );
    holder.kill();
    assert!(
        desktop::names_tracewing(&desktop::setting(), capture.port),
        "the setting went back with nobody to put it back"
    );
    // The user quits before uninstalling.
    drop(capture);

    let out = tracewing_at(program)
        .arg("uninstall")
        .arg("--data-dir")
        .arg(&data)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", said(&out));
    assert_eq!(desktop::setting(), before, "{}", said(&out));
    assert_eq!(ca_locations(&pem).unwrap(), vec![], "{}", said(&out));
    assert!(!autostart().exists(), "still armed: {}", said(&out));
    // What was recorded stays, as it says.
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("what tracewing recorded is still in"),
        "{}",
        said(&out)
    );
    assert!(data.join("ca.pem").exists());

    let out = package.remove();
    assert!(out.status.success(), "remove the package: {}", said(&out));
    for path in FILES {
        assert!(
            !Path::new(path).exists(),
            "{path} is left after removing it"
        );
    }
    assert!(!Package::known(package.kind), "the package is still known");
    assert_eq!(on_path(), None, "still on the PATH");

    std::fs::remove_dir_all(&dir).ok();
}
