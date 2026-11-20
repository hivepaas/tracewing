//! S3, after a kill: the app holding the system proxy is killed outright, and
//! the setting still comes back, by tracewing's own programs — its watchdog,
//! and `tracewing recover`, which the minute timer runs as `--only-if-gone`.
//! The app is a process of this test's (see `helper`), so that killing it
//! kills nothing else; the watchdog and `recover` are the real `tracewing`.
//!
//! The systemd timer firing by itself needs a systemd, which the container
//! does not run; that part is not here yet (docs/design/e2e.md).
//!
//! Changes this computer's proxy setting. See `tracewing_e2e`.

#![cfg(target_os = "linux")]

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, Stdio},
    time::Duration,
};
use tracewing_e2e::{
    desktop::{Setting, names_tracewing, setting, the_users_own},
    free_port,
    helper::{Helper, asked, hold_until_killed},
    only_where_allowed, said, scratch, tracewing, wait_for,
};

const APP: &str = "an_app_that_holds_the_system_proxy";

/// Not a test: the app, run by the tests below as a process of its own.
/// Asked `<data dir>|<port>|watchdog` or `<data dir>|<port>|alone`.
#[test]
#[ignore = "run by the other tests here, as the app they kill"]
fn an_app_that_holds_the_system_proxy() {
    let Some(what) = asked() else { return };
    let mut parts = what.split('|');
    let dir = PathBuf::from(parts.next().unwrap());
    let port: u16 = parts.next().unwrap().parse().unwrap();
    let with_watchdog = parts.next() == Some("watchdog");
    let taken =
        tracewing_system::take_over(&dir, SocketAddr::from(([127, 0, 0, 1], port)), false).unwrap();
    // The window starts its watchdog as itself; this app is not tracewing,
    // so it starts tracewing's. Its end of the pipe goes when it does.
    let _watchdog: Option<Child> = with_watchdog.then(|| {
        tracewing()
            .args(["watchdog", "--data-dir"])
            .arg(&dir)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap()
    });
    std::mem::forget(taken);
    hold_until_killed();
}

/// The app, holding the setting, for `port`.
fn app(dir: &Path, port: u16, watchdog: bool) -> Helper {
    let what = format!(
        "{}|{port}|{}",
        dir.display(),
        if watchdog { "watchdog" } else { "alone" }
    );
    let helper = Helper::start(APP, &what);
    let now = setting();
    assert!(
        names_tracewing(&now, port),
        "the app did not take it: {now:?}"
    );
    helper
}

fn recover(dir: &Path, only_if_gone: bool) -> String {
    let mut command = tracewing();
    command.args(["recover", "--data-dir"]).arg(dir);
    if only_if_gone {
        command.arg("--only-if-gone");
    }
    let out = command.output().unwrap();
    assert!(out.status.success(), "{}", said(&out));
    said(&out)
}

fn back(before: &Setting, within: Duration) {
    wait_for("the setting to come back as it was", within, || {
        &setting() == before
    });
}

#[test]
fn the_watchdog_puts_it_back_when_the_app_is_killed() {
    only_where_allowed!();
    let dir = scratch("watchdog");
    the_users_own();
    let before = setting();
    let port = free_port();

    let app = app(&dir, port, true);
    app.kill();
    back(&before, Duration::from_secs(10));
    assert!(
        !dir.join("sysproxy.journal").exists(),
        "the journal outlived the setting it kept"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn recover_leaves_a_running_app_alone_and_puts_it_back_once_it_is_gone() {
    only_where_allowed!();
    let dir = scratch("recover");
    the_users_own();
    let before = setting();
    let port = free_port();

    // What the minute timer runs, while the app is still there: nothing.
    let app = app(&dir, port, false);
    let said = recover(&dir, true);
    assert!(
        names_tracewing(&setting(), port),
        "taken from a running app: {said}"
    );

    // Killed with no watchdog: the setting stays pointing at a dead port
    // until the timer's next run.
    app.kill();
    std::thread::sleep(Duration::from_secs(1));
    assert!(
        names_tracewing(&setting(), port),
        "something else put it back"
    );
    let said = recover(&dir, true);
    assert_eq!(setting(), before, "{said}");

    // And `recover` alone, as a user runs it, does the same at once.
    let app = self::app(&dir, port, false);
    app.kill();
    let said = recover(&dir, false);
    assert_eq!(setting(), before, "{said}");
    std::fs::remove_dir_all(&dir).ok();
}
