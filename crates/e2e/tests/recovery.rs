//! S3, after a kill: tracewing holding the system proxy (`e2e-hold`, as the
//! window does) is killed outright, and the setting still comes back, by what
//! it armed behind it — its watchdog — and by `tracewing recover`, which the
//! minute timer runs as `--only-if-gone`.
//!
//! On Windows the scheduled task behind it is seen firing too, with the app
//! and its watchdog both killed. The systemd timer firing by itself needs a
//! user's systemd, which the Linux container does not run: CI runs that one
//! on the runner (docs/design/e2e.md).
//!
//! Changes this computer's proxy setting. See `tracewing_e2e`.

#![cfg(any(target_os = "linux", windows))]

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
    let _put_back = the_users_own();
    let before = setting();

    app(&dir, free_port(), true).kill();
    back(&before, Duration::from_secs(10));
    // The setting first, then the journal that kept it: a moment apart.
    wait_for("the journal to go", Duration::from_secs(5), || {
        !dir.join("sysproxy.journal").exists()
    });
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn recover_leaves_a_running_app_alone_and_puts_it_back_once_it_is_gone() {
    only_where_allowed!();
    let dir = scratch("recover");
    let _put_back = the_users_own();
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

/// The last line of defence: with the watchdog killed first, so that it
/// cannot do it, and then the app, only the task the app armed is left, and
/// it runs `recover --only-if-gone` once a minute.
#[cfg(windows)]
#[test]
fn the_scheduled_task_puts_it_back_when_the_app_and_its_watchdog_are_killed() {
    only_where_allowed!();
    let dir = scratch("task");
    let _put_back = the_users_own();
    let before = setting();
    let port = free_port();

    let running = app(&dir, port, true);
    // Its own name, so that it is the one killed (crates/system: WATCHDOG_EXE).
    let out = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "trw-watchdog.exe"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", said(&out));
    running.kill();
    std::thread::sleep(Duration::from_secs(2));
    assert!(
        names_tracewing(&setting(), port),
        "put back at once, so not by the task"
    );
    back(&before, Duration::from_secs(150));
    std::fs::remove_dir_all(&dir).ok();
}

/// Linux's last line of defence: with the watchdog killed first and then the
/// app, only the systemd user timer the app armed is left, and it runs
/// `recover --only-if-gone` once a minute. It needs a user's systemd, which
/// the container has none of: CI runs it on the runner itself
/// (`linux-e2e`'s "systemd timer" step), and anywhere without one it says so.
#[cfg(target_os = "linux")]
#[test]
fn the_systemd_timer_puts_it_back_when_the_app_and_its_watchdog_are_killed() {
    only_where_allowed!();
    let state = std::process::Command::new("systemctl")
        .args(["--user", "is-system-running"])
        .output();
    let state = state
        .as_ref()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .unwrap_or_default();
    if !matches!(state.as_str(), "running" | "degraded") {
        eprintln!("skipped: no systemd user manager here ({state:?})");
        return;
    }
    let dir = scratch("timer");
    let _put_back = the_users_own();
    let before = setting();
    let port = free_port();

    let running = app(&dir, port, true);
    let timer = std::process::Command::new("systemctl")
        .args(["--user", "is-active", "tracewing-proxy-recovery.timer"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&timer.stdout).trim(),
        "active",
        "no timer armed: {}",
        tracewing_e2e::said(&timer)
    );
    // Its own name, so that it is the one killed (crates/system: the
    // watchdog renames itself).
    let out = std::process::Command::new("pkill")
        .args(["-KILL", "-x", "trw-watchdog"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", tracewing_e2e::said(&out));
    running.kill();
    std::thread::sleep(Duration::from_secs(2));
    assert!(
        names_tracewing(&setting(), port),
        "put back at once, so not by the timer"
    );
    back(&before, Duration::from_secs(150));
    // Its work done, it disarms itself.
    wait_for("the timer to stop", Duration::from_secs(10), || {
        let out = std::process::Command::new("systemctl")
            .args(["--user", "is-active", "tracewing-proxy-recovery.timer"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).trim() != "active"
    });
    std::fs::remove_dir_all(&dir).ok();
}
