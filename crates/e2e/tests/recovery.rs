//! S3, after a kill: tracewing holding the system proxy (`e2e-hold`, as the
//! window does) is killed outright, and the setting still comes back, by what
//! it armed behind it — its watchdog — and by `tracewing recover`, which the
//! minute timer runs as `--only-if-gone`.
//!
//! The systemd timer firing by itself needs a systemd, which the container
//! does not run; that part is not here yet (docs/design/e2e.md).
//!
//! Changes this computer's proxy setting. See `tracewing_e2e`.

#![cfg(target_os = "linux")]

use std::{path::Path, time::Duration};
use tracewing_e2e::{
    desktop::{Setting, names_tracewing, setting, the_users_own},
    free_port,
    helper::Helper,
    only_where_allowed, said, scratch, tracewing, wait_for,
};

/// tracewing, holding the setting for `port`.
fn app(dir: &Path, port: u16, watchdog: bool) -> Helper {
    let app = Helper::system_proxy(dir, port, watchdog);
    let now = setting();
    assert!(names_tracewing(&now, port), "it did not take it: {now:?}");
    app
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

    app(&dir, free_port(), true).kill();
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

    // What the minute timer runs, while tracewing is still there: nothing.
    let running = app(&dir, port, false);
    let said = recover(&dir, true);
    assert!(
        names_tracewing(&setting(), port),
        "taken from a running app: {said}"
    );

    // Killed with no watchdog: the setting stays pointing at a dead port
    // until the timer's next run.
    running.kill();
    std::thread::sleep(Duration::from_secs(1));
    assert!(
        names_tracewing(&setting(), port),
        "something else put it back"
    );
    let said = recover(&dir, true);
    assert_eq!(setting(), before, "{said}");

    // And `recover` alone, as a user runs it, does the same at once.
    app(&dir, port, false).kill();
    let said = recover(&dir, false);
    assert_eq!(setting(), before, "{said}");
    std::fs::remove_dir_all(&dir).ok();
}
