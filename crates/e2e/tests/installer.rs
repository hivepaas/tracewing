//! S4 on Windows, the installer: `setup.exe` for this user and for everyone,
//! quiet, as a user or an administrator runs it. The program it puts on the
//! computer records HTTPS and stands behind the system proxy it takes; then
//! the uninstaller, which runs `tracewing uninstall` itself, and nothing of
//! either is left: no store trusts the CA, the proxy setting is the user's,
//! no scheduled task is armed, and the folder, the uninstall key, the
//! shortcut and the PATH entry are gone. And the `.zip`, run from a folder.
//!
//! Both carry Wintun's `wintun.dll` and its licence beside the program; the
//! installed one, left with the local way in switched on, brings the adapter
//! up with that library and records a chosen program's HTTPS through it
//! (`tracewing_e2e::local_window`).
//!
//! And the updater's way of installing the next one (docs/updates.md): the
//! script a quitting tracewing leaves behind waits for that tracewing to be
//! gone, then runs the installer quietly over it.
//!
//! The installer is `TRACEWING_E2E_SETUP` and the zip `TRACEWING_E2E_ZIP`,
//! which `scripts/e2e_windows.ps1 -Packages` builds and hands in.
//!
//! The uninstaller's `tracewing uninstall` reads the window's data folder,
//! `%LOCALAPPDATA%\tracewing`, so this test records there. What is there
//! already is set aside first and put back after.
//!
//! Installs and uninstalls tracewing, and changes this computer's trust
//! stores and proxy setting. See `tracewing_e2e`.
#![cfg(windows)]

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use tracewing_e2e::{
    Capture, Origin, curl_through_with, desktop, har_of, helper::Helper, local_window,
    only_where_allowed, said, scratch, tracewing_at, wait_for,
};
use tracewing_proxy::ca_key::KeyStore;
use tracewing_system::{Scope, ca_install, ca_locations, ca_remove};

const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\tracewing";
const TASK: &str = "tracewing-proxy-recovery";

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("no {name}")))
}

/// A value from the registry's 64-bit view, if it is there.
fn reg_value(key: &str, name: &str) -> Option<String> {
    let out = Command::new("reg")
        .args(["query", key, "/v", name, "/reg:64"])
        .output()
        .unwrap();
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| {
            let rest = line.trim_start().strip_prefix(name)?.trim_start();
            let (_kind, data) = rest.split_once(char::is_whitespace)?;
            Some(data.trim().to_owned())
        })
}

#[derive(Clone, Copy, Debug)]
enum For {
    User,
    Everyone,
}

impl For {
    fn flag(self) -> &'static str {
        match self {
            For::User => "/CURRENTUSER",
            For::Everyone => "/ALLUSERS",
        }
    }

    fn folder(self) -> PathBuf {
        match self {
            For::User => env_path("LOCALAPPDATA").join(r"Programs\tracewing"),
            For::Everyone => env_path("ProgramFiles").join("tracewing"),
        }
    }

    fn hive(self) -> &'static str {
        match self {
            For::User => "HKCU",
            For::Everyone => "HKLM",
        }
    }

    fn uninstall_key(self) -> String {
        format!(r"{}\{UNINSTALL_KEY}", self.hive())
    }

    /// The PATH Windows keeps for this user, or for the computer.
    fn path(self) -> String {
        match self {
            For::User => reg_value(r"HKCU\Environment", "Path"),
            For::Everyone => reg_value(
                r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
                "Path",
            ),
        }
        .unwrap_or_default()
    }

    fn shortcut(self) -> PathBuf {
        match self {
            For::User => env_path("APPDATA"),
            For::Everyone => env_path("ProgramData"),
        }
        .join(r"Microsoft\Windows\Start Menu\Programs\tracewing.lnk")
    }
}

fn holds(path: &str, dir: &Path) -> bool {
    let dir = dir.to_string_lossy();
    path.split(';').any(|entry| {
        entry
            .trim_end_matches('\\')
            .eq_ignore_ascii_case(dir.trim_end_matches('\\'))
    })
}

/// The window's data folder, set aside while the test uses it and put back
/// after: what was there is somebody's.
struct Aside {
    data: PathBuf,
    kept: PathBuf,
}

impl Aside {
    fn take() -> Self {
        let data = tracewing_proxy::default_data_dir();
        let kept = data.with_file_name(format!("tracewing.e2e-aside-{}", std::process::id()));
        if data.exists() {
            std::fs::rename(&data, &kept).unwrap();
        }
        Aside { data, kept }
    }
}

impl Drop for Aside {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.data);
        if self.kept.exists() {
            let _ = std::fs::rename(&self.kept, &self.data);
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

/// Installed until uninstalled: if the test fails first, uninstalled when it
/// ends.
struct Installed {
    how: For,
    gone: bool,
}

impl Installed {
    fn exe(&self) -> PathBuf {
        self.how.folder().join("tracewing.exe")
    }

    /// The uninstaller copies itself aside and ends at once: what it
    /// removes is waited for.
    fn uninstall(&mut self) {
        self.gone = true;
        let status = Command::new(self.how.folder().join("uninstall.exe"))
            .arg("/S")
            .status()
            .unwrap();
        assert!(status.success(), "uninstall.exe /S: {status}");
        let key = self.how.uninstall_key();
        wait_for(
            "the uninstaller to finish",
            Duration::from_secs(120),
            || !self.exe().exists() && reg_value(&key, "InstallLocation").is_none(),
        );
    }
}

impl Drop for Installed {
    fn drop(&mut self) {
        if !self.gone && self.how.folder().join("uninstall.exe").exists() {
            let _ = Command::new(self.how.folder().join("uninstall.exe"))
                .arg("/S")
                .status();
        }
    }
}

fn install(setup: &Path, how: For) -> Installed {
    assert!(
        !how.folder().join("tracewing.exe").exists(),
        "tracewing is already in {}",
        how.folder().display()
    );
    let status = Command::new(setup)
        .args(["/S", how.flag()])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "{} /S {}: {status}",
        setup.display(),
        how.flag()
    );
    Installed { how, gone: false }
}

/// The task tracewing arms, as `schtasks` describes it, if there is one.
fn task() -> Option<String> {
    let out = Command::new("schtasks")
        .args(["/Query", "/TN", TASK, "/XML"])
        .output()
        .unwrap();
    out.status.success().then(|| text_of(&out.stdout))
}

/// `schtasks /XML` writes UTF-16 on some versions and UTF-8 on others.
fn text_of(bytes: &[u8]) -> String {
    match bytes {
        [0xFF, 0xFE, rest @ ..] => {
            let (pairs, _) = rest.as_chunks::<2>();
            let wide: Vec<u16> = pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
            String::from_utf16_lossy(&wide)
        }
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// HTTPS through `program` in `data`, as a client that trusts only what the
/// system trusts, and both bodies in the HAR it writes.
fn records_https(program: &Path, data: &Path, origin: &Origin, capture: &Capture) {
    let marker = format!("sent-through-the-installed-one-{}", std::process::id());
    let url = format!("https://localhost:{}/echo", origin.port);
    let out = curl_through_with(capture.port, &["--data", &marker, &url]);
    assert!(out.status.success(), "curl through it: {}", said(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains(&format!("body={marker}")),
        "{}",
        said(&out)
    );
    let mut har = String::new();
    wait_for("the request in the HAR", Duration::from_secs(10), || {
        har = har_of(program, data);
        har.contains(&url)
    });
    assert!(
        har.matches(marker.as_str()).count() >= 2,
        "the HAR has not got what was sent and answered: {har}"
    );
}

/// A package handed in by `name`, or `None`, said, when there is none.
fn handed_in(name: &str) -> Option<PathBuf> {
    let file = std::env::var_os(name).filter(|file| !file.is_empty());
    if file.is_none() {
        eprintln!("skipped: {name} names nothing to install (e2e_windows.ps1 -Packages)");
    }
    file.map(PathBuf::from)
}

/// What `tracewing_update::apply` does on Windows as tracewing quits with an
/// update downloaded: PowerShell, started detached, waits for tracewing's own
/// process to be gone, then runs the installer quietly. The installer copies
/// tracewing.exe over the old one and, quiet, gives up after five seconds of
/// trying while it is still running: so the wait is the whole point, and is
/// what this sees.
///
/// The installed tracewing is kept running (`tracewing mcp`, which lives
/// until its stdin closes), a file of the install is taken away, and the
/// script started. While tracewing runs, nothing is installed; once it has
/// gone, the installer runs over the same folder and the file is back.
fn the_updater_installs_over_it_once_it_has_gone(setup: &Path, program: &Path, how: For) {
    let mut running = Command::new(program)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let licence = how.folder().join("WINTUN-LICENSE.txt");
    std::fs::remove_file(&licence).unwrap();
    let script = tracewing_update::apply::installer_script(running.id(), setup, None);
    let mut updater = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command"])
        .arg(&script)
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_secs(5));
    assert!(
        updater.try_wait().unwrap().is_none(),
        "the updater did not wait for tracewing: {script}"
    );
    assert!(
        !licence.exists(),
        "installed while tracewing was still running"
    );
    // tracewing goes, as it does when it quits.
    drop(running.stdin.take());
    let gone = running.wait().unwrap();
    println!("tracewing went: {gone}");
    let status = updater.wait().unwrap();
    assert!(status.success(), "the updater's script: {status}");
    assert!(
        licence.exists(),
        "the installer did not run over {}",
        how.folder().display()
    );
    assert!(program.exists());
    assert_eq!(
        reg_value(&how.uninstall_key(), "InstallLocation").map(PathBuf::from),
        Some(how.folder()),
        "{how:?}: the update went somewhere else"
    );
}

fn the_installer(how: For) {
    let Some(setup) = handed_in("TRACEWING_E2E_SETUP") else {
        return;
    };
    let _aside = Aside::take();
    let data = tracewing_proxy::default_data_dir();

    let mut installed = install(&setup, how);
    let program = installed.exe();
    assert!(program.exists(), "{how:?}: no {}", program.display());
    assert_eq!(
        reg_value(&how.uninstall_key(), "InstallLocation").map(PathBuf::from),
        Some(how.folder()),
        "{how:?}: the uninstall key"
    );
    assert!(
        holds(&how.path(), &how.folder()),
        "{how:?}: not in the PATH"
    );
    assert!(how.shortcut().exists(), "{how:?}: no shortcut");
    // Wintun's library beside the program, for the local way in, with its
    // licence.
    for file in ["wintun.dll", "WINTUN-LICENSE.txt"] {
        assert!(how.folder().join(file).exists(), "{how:?}: no {file}");
    }

    let _put_back = desktop::the_users_own();
    let before = desktop::setting();
    let dir = scratch(&format!("installer-{how:?}"));
    let origin = Origin::start(&dir);
    let capture = Capture::start_from(&program, &data, &origin, KeyStore::File);
    let pem = capture.ca_pem();
    let _untrust = Untrust(pem.clone());
    ca_install(&pem, Scope::User).unwrap();
    ca_install(&pem, Scope::Machine).unwrap();
    records_https(&program, &data, &origin, &capture);
    // And with no proxy settings: the window, left switched on for curl,
    // brings the adapter up with the wintun.dll it was installed with.
    if matches!(how, For::User) {
        let local = dir.join("local");
        std::fs::create_dir_all(&local).unwrap();
        local_window::a_chosen_program_is_recorded_through_the_window(&program, &local);
        the_updater_installs_over_it_once_it_has_gone(&setup, &program, how);
    }

    // The system proxy, taken by the installed program, which arms the task
    // that puts it back as itself; then killed, so that it is left for the
    // uninstaller to put back.
    let holder = Helper::system_proxy_from(&program, &data, capture.port, false);
    assert!(desktop::names_tracewing(&desktop::setting(), capture.port));
    let armed = task().expect("no task armed");
    assert!(
        armed
            .to_lowercase()
            .contains(&program.to_string_lossy().to_lowercase()),
        "the task does not run the installed program: {armed}"
    );
    holder.kill();
    assert!(desktop::names_tracewing(&desktop::setting(), capture.port));
    drop(capture);

    installed.uninstall();
    assert_eq!(desktop::setting(), before, "{how:?}: the proxy setting");
    assert_eq!(
        ca_locations(&pem).unwrap(),
        vec![],
        "{how:?}: still trusted"
    );
    assert_eq!(task(), None, "{how:?}: the task is still armed");
    assert!(!how.folder().exists(), "{how:?}: the folder is left");
    assert!(
        !holds(&how.path(), &how.folder()),
        "{how:?}: still in the PATH"
    );
    assert!(!how.shortcut().exists(), "{how:?}: the shortcut is left");
    // What was recorded stays.
    assert!(pem.exists(), "{how:?}: the data folder went too");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_installer_for_this_user_records_https_and_leaves_nothing_behind() {
    only_where_allowed!();
    the_installer(For::User);
}

#[test]
fn the_installer_for_everyone_records_https_and_leaves_nothing_behind() {
    only_where_allowed!();
    the_installer(For::Everyone);
}

#[test]
fn the_zip_runs_from_its_folder_and_leaves_nothing_behind() {
    only_where_allowed!();
    let Some(zip) = handed_in("TRACEWING_E2E_ZIP") else {
        return;
    };
    let dir = scratch("zip");
    let unpacked = dir.join("unpacked");
    std::fs::create_dir_all(&unpacked).unwrap();
    let out = Command::new("tar")
        .arg("-xf")
        .arg(&zip)
        .arg("-C")
        .arg(&unpacked)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "unpack {}: {}",
        zip.display(),
        said(&out)
    );
    let program = std::fs::read_dir(&unpacked)
        .unwrap()
        .flatten()
        .map(|entry| entry.path().join("tracewing.exe"))
        .find(|exe| exe.exists())
        .expect("no tracewing.exe in a folder of the zip");
    for file in ["wintun.dll", "WINTUN-LICENSE.txt"] {
        assert!(
            program.with_file_name(file).exists(),
            "the zip has no {file}"
        );
    }
    let user_path = For::User.path();

    let data = dir.join("data");
    let origin = Origin::start(&dir);
    let capture = Capture::start_from(&program, &data, &origin, KeyStore::File);
    let pem = capture.ca_pem();
    let _untrust = Untrust(pem.clone());
    ca_install(&pem, Scope::User).unwrap();
    records_https(&program, &data, &origin, &capture);
    drop(capture);

    let out = tracewing_at(&program)
        .arg("uninstall")
        .arg("--data-dir")
        .arg(&data)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", said(&out));
    assert_eq!(ca_locations(&pem).unwrap(), vec![], "{}", said(&out));
    // Run from a folder, it put nothing in the PATH.
    assert_eq!(For::User.path(), user_path);
    std::fs::remove_dir_all(&dir).unwrap();
}
