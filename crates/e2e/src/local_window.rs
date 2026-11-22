//! S8 through the window, on Windows: the local way in left switched on, as a
//! user leaves it, and the window started again. The window brings the
//! adapter up as it does at every start (`local_apps::start_local`), with the
//! `wintun.dll` beside the program -- an installed package's own, or the one
//! `WINTUN_DLL` names for a build that has none -- and a chosen program's
//! HTTPS is recorded through it. Closed as a user closes it, the adapter and
//! its routes are gone.
//!
//! The window runs on a data folder of the test's own: its `LOCALAPPDATA` is
//! a folder under the test's, so `%LOCALAPPDATA%\tracewing` is nobody's.
//!
//! It needs the interactive desktop (a window opens) and an administrator
//! (the adapter): `scripts/e2e_windows.ps1`, in a task with `/IT`, or a
//! runner.

use crate::{Origin, har_of, said, tracewing_at, wait_for};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
    time::Duration,
};
use tracewing_proxy::roads;

/// Where the chosen client sends: a documentation address, which the
/// adapter's `/1` routes take and nothing answers.
pub const AS_IF_THE_SERVER: &str = "203.0.113.9";

/// The curl every Windows since 1803 has, which is the program chosen.
pub const CURL: &str = r"C:\Windows\System32\curl.exe";

/// Whether `program` can find a `wintun.dll`: beside it, as an installed
/// package has one, or by `WINTUN_DLL`.
pub fn wintun_for(program: &Path) -> Option<PathBuf> {
    let beside = program.parent()?.join("wintun.dll");
    if beside.exists() {
        return Some(beside);
    }
    std::env::var_os("WINTUN_DLL")
        .map(PathBuf::from)
        .filter(|dll| dll.exists())
}

/// The window, until dropped: then killed if it is still there.
struct Window(Child);

impl Drop for Window {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The adapter's `/1` rows: rows of an interface that was not in `before`.
fn ours(before: &[roads::Road]) -> Vec<roads::Road> {
    let was: Vec<u64> = before.iter().map(|road| road.luid).collect();
    roads::roads_now()
        .expect("this machine's routes")
        .into_iter()
        .filter(|road| road.bits == 1 && !was.contains(&road.luid))
        .collect()
}

/// The whole of it, with `program`, in `dir`.
pub fn a_chosen_program_is_recorded_through_the_window(program: &Path, dir: &Path) {
    let appdata = dir.join("appdata");
    let data = appdata.join("tracewing");
    std::fs::create_dir_all(data.join("trusted-servers")).unwrap();
    // The switch left on, with curl chosen: the window's own file.
    std::fs::write(
        data.join("local-programs.toml"),
        format!("on = true\n\n[[program]]\nname = \"curl.exe\"\npath = '{CURL}'\n"),
    )
    .unwrap();
    // The test's server, trusted by the window's way out as a user trusts
    // one in Settings.
    let origin = Origin::start(dir);
    std::fs::copy(&origin.ca, data.join("trusted-servers").join("origin.pem")).unwrap();

    let before = roads::roads_now().expect("this machine's routes");
    let mut command = tracewing_at(program);
    command.env("LOCALAPPDATA", &appdata);
    // `WINTUN_DLL` wins over the one beside the program, so a package that
    // has its own is run without it: the library it ships is the one tried.
    if program.with_file_name("wintun.dll").exists() {
        command.env_remove("WINTUN_DLL");
    }
    let window = Window(
        command
            .spawn()
            .unwrap_or_else(|e| panic!("start {}: {e}", program.display())),
    );
    let pid = window.0.id();
    wait_for(
        "the window to bring the adapter up",
        Duration::from_secs(60),
        || !ours(&before).is_empty(),
    );
    println!("the window brought the adapter up: {:?}", ours(&before));
    wait_for("the window's CA", Duration::from_secs(30), || {
        data.join("ca.pem").exists()
    });

    // The chosen curl, into the adapter under the origin's name; the pump
    // may start a moment after the routes, so it is asked again for a while.
    let marker = format!("through-the-window-{}", std::process::id());
    let url = format!("https://localhost:{}/echo", origin.port);
    let mut out = None;
    wait_for(
        "the chosen curl to be answered",
        Duration::from_secs(30),
        || {
            let asked = Command::new(CURL)
                .args([
                    "--silent",
                    "--show-error",
                    "--max-time",
                    "10",
                    "--ssl-no-revoke",
                ])
                .arg("--connect-to")
                .arg(format!("localhost:{0}:{AS_IF_THE_SERVER}:{0}", origin.port))
                .arg("--cacert")
                .arg(data.join("ca.pem"))
                .args(["--data", &marker, &url])
                .output()
                .expect("run curl");
            let answered = asked.status.success()
                && String::from_utf8_lossy(&asked.stdout).contains(&format!("body={marker}"));
            out = Some(asked);
            answered
        },
    );
    println!(
        "answered: {}",
        said(out.as_ref().unwrap()).lines().next().unwrap_or("")
    );
    let mut har = String::new();
    wait_for("the request in the HAR", Duration::from_secs(15), || {
        har = har_of(program, &data);
        har.contains(&marker)
    });
    assert!(
        har.matches(marker.as_str()).count() >= 2,
        "the HAR has not got both what was sent and what came back: {har}"
    );

    // Closed as a user closes it: the close message, not a kill.
    let out = Command::new("taskkill")
        .args(["/PID", &pid.to_string()])
        .output()
        .unwrap();
    assert!(out.status.success(), "taskkill /PID {pid}: {}", said(&out));
    let mut window = window;
    wait_for("the window to close", Duration::from_secs(30), || {
        window.0.try_wait().ok().flatten().is_some()
    });
    wait_for(
        "the adapter's routes to go with the window",
        Duration::from_secs(15),
        || ours(&before).is_empty(),
    );

    // What the window put outside its folder, taken out as a user takes it.
    let out = tracewing_at(program)
        .arg("uninstall")
        .arg("--data-dir")
        .arg(&data)
        .output()
        .unwrap();
    assert!(out.status.success(), "tracewing uninstall: {}", said(&out));
}
