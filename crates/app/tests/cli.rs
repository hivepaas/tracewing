//! `tracewing capture`, `env`, `export`, `sessions` and `recover`, as a terminal runs
//! them: the program itself, recording with no window, then its HAR.
//!
//! `cargo test --release -p tracewing --test cli`

use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn tracewing() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tracewing"))
}

/// A command that may put the system proxy back, with the XDG folders pointed
/// inside `dir`. Putting the settings back takes the login recovery and the
/// minute timer away with them, and on a machine somebody works on those
/// belong to whatever tracewing they have running, not to a test.
fn in_its_own_folders(dir: &std::path::Path, name: &str) -> Command {
    let mut command = tracewing();
    command
        .args([name, "--data-dir"])
        .arg(dir)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir.join("config"));
    command
}

fn recover(dir: &std::path::Path) -> Command {
    in_its_own_folders(dir, "recover")
}

/// What `recover` leaves behind, either way. On a machine with a desktop it
/// says which of the two happened and the journal goes with it. On one without
/// — a build runner — there is no setting to read, so it says what it could
/// not do and keeps the journal: that is the only record of what the settings
/// were, and throwing it away on a machine that cannot read them would lose
/// them for the machine that can.
fn it_put_them_back_or_said_why_not(out: &std::process::Output, journal: &std::path::Path) {
    let said = String::from_utf8_lossy(&out.stdout);
    if out.status.success() {
        assert!(
            said.contains("left alone") || said.contains("back as it was"),
            "it says which of the two happened: {said}"
        );
        assert!(
            !journal.exists(),
            "and the journal goes either way, so it cannot be used twice"
        );
    } else {
        let complaint = String::from_utf8_lossy(&out.stderr);
        assert!(
            complaint.contains("read this computer's proxy setting"),
            "{complaint}"
        );
        assert!(
            journal.exists(),
            "and the journal stays, being all that is left of the settings"
        );
    }
}

/// A journal as a killed run leaves one: the address it pointed the system
/// proxy at, and each setting as it was before. The keys are the journal's
/// own, `addr` and `saved.<name>`.
fn journal_of_a_killed_run(dir: &std::path::Path) {
    std::fs::write(
        dir.join("sysproxy.journal"),
        "addr=127.0.0.1:9090\nsaved.mode='none'\n",
    )
    .unwrap();
}

#[test]
fn it_records_with_no_window_and_writes_a_har() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let origin = listener.local_addr().unwrap().port();
    runtime.spawn(tracewing_dev::playground::serve(listener));

    // Help is said, and is no failure.
    let help = tracewing().arg("--help").output().unwrap();
    assert!(help.status.success());
    let said = String::from_utf8_lossy(&help.stdout);
    assert!(
        said.contains("capture") && said.contains("export"),
        "{said}"
    );

    let har = dir.join("out.har");
    let child = tracewing()
        .args([
            "capture",
            "--port",
            "20340",
            "--seconds",
            "4",
            "--ca-key-in-file",
        ])
        .args(["--har"])
        .arg(&har)
        .args(["--filter", "/json", "--data-dir"])
        .arg(&dir)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !dir.join("ca.pem").exists() || !dir.join("proxy-address").exists() {
        assert!(Instant::now() < deadline, "it starts");
        std::thread::sleep(Duration::from_millis(20));
    }
    // Its key in a file, as asked, not in the system's keychain.
    assert!(dir.join("ca.key").is_file());
    let address = std::fs::read_to_string(dir.join("proxy-address")).unwrap();
    assert_eq!(address, "127.0.0.1:20340");
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(format!("http://{address}")).unwrap())
        .build()
        .unwrap();
    runtime.block_on(async {
        for path in ["/json", "/xml"] {
            let res = client
                .get(format!("http://localhost:{origin}{path}"))
                .send()
                .await
                .unwrap();
            assert_eq!(res.status(), 200);
            res.bytes().await.unwrap();
        }
    });
    let out = child.wait_with_output().unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{said}");
    assert!(
        said.contains("recording on http://127.0.0.1:20340"),
        "{said}"
    );
    assert!(said.contains("2 flows recorded"), "{said}");
    assert!(
        said.contains("1 flow written"),
        "only what the filter found: {said}"
    );
    assert!(
        !dir.join("proxy-address").exists(),
        "taken away as it stops"
    );

    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&har).unwrap()).unwrap();
    let entries = written["log"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0]["request"]["url"]
            .as_str()
            .unwrap()
            .ends_with("/json")
    );

    // The sessions, and the whole of the newest written again.
    let listed = tracewing()
        .args(["sessions", "--data-dir"])
        .arg(&dir)
        .output()
        .unwrap();
    let listed = String::from_utf8_lossy(&listed.stdout);
    assert_eq!(listed.lines().count(), 2, "a heading and the one: {listed}");
    let id = listed
        .lines()
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    let all = dir.join("all.har");
    let exported = tracewing()
        .args(["export", "--session", id, "--without-secrets", "--har"])
        .arg(&all)
        .arg("--data-dir")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(exported.status.success(), "{exported:?}");
    assert!(String::from_utf8_lossy(&exported.stdout).starts_with("2 flows written"));
    let missing = tracewing()
        .args(["export", "--session", "nope", "--har"])
        .arg(&all)
        .arg("--data-dir")
        .arg(&dir)
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("no session nope"));

    std::fs::remove_dir_all(&dir).ok();
}

/// The command somebody runs when the network has gone: it has to work with
/// no window, no proxy running, and nothing of ours left behind.
#[test]
fn recover_says_what_it_did_and_leaves_a_stranger_alone() {
    let dir = std::env::temp_dir().join(format!("tracewing-recover-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Nothing of tracewing's is on the machine: it says so and does nothing.
    let out = recover(&dir).output().unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("nothing to put back"),
        "{out:?}"
    );

    // A journal from a run that was killed, naming settings that are not on
    // this machine now: it must not put somebody else's settings back.
    journal_of_a_killed_run(&dir);
    let out = recover(&dir).output().unwrap();
    it_put_them_back_or_said_why_not(&out, &dir.join("sysproxy.journal"));

    std::fs::remove_dir_all(&dir).ok();
}

/// What the minute timer runs, once a minute, for the whole of an ordinary
/// session. A run of it against a healthy app has to come to nothing, or
/// turning the system proxy on would take the network away a minute later.
#[test]
fn recover_only_if_gone_leaves_a_running_app_alone() {
    let dir = std::env::temp_dir().join(format!("tracewing-if-gone-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let journal = dir.join("sysproxy.journal");
    journal_of_a_killed_run(&dir);

    // The lock an app holds for as long as the system proxy is its own, held
    // here by the test: another process entirely, which is what the timer is.
    let held = tracewing_system::hold_alive(&dir).unwrap();
    let out = recover(&dir).arg("--only-if-gone").output().unwrap();
    assert!(out.status.success(), "{out:?}");
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(said.contains("still running"), "{said}");
    assert!(journal.exists(), "and the journal is left for the real end");

    // The lock goes however the app ends. Now there is work to do — once no
    // other test in this program is in the moment between starting a process
    // and that process taking over, when the half-made process holds a copy
    // of everything its parent had open, this lock among it.
    drop(held);
    let out = (0..50)
        .find_map(|_| {
            let out = recover(&dir).arg("--only-if-gone").output().unwrap();
            if String::from_utf8_lossy(&out.stdout).contains("still running") {
                std::thread::sleep(Duration::from_millis(20));
                return None;
            }
            Some(out)
        })
        .expect("it went on saying tracewing was running");
    it_put_them_back_or_said_why_not(&out, &journal);

    std::fs::remove_dir_all(&dir).ok();
}

/// A terminal of the user's own, sent to whichever tracewing is running on the
/// data folder, and told plainly when none is.
#[test]
fn env_takes_a_terminal_through_what_is_running() {
    let dir = std::env::temp_dir().join(format!("tracewing-env-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let env = |extra: &[&str]| {
        tracewing()
            .arg("env")
            .args(extra)
            .arg("--data-dir")
            .arg(&dir)
            .output()
            .unwrap()
    };

    let out = env(&[]);
    assert!(!out.status.success());
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("tracewing is not running"), "{said}");
    assert!(out.stdout.is_empty(), "nothing for a shell to run");

    let mut capture = tracewing()
        .args([
            "capture",
            "--port",
            "20341",
            "--seconds",
            "30",
            "--ca-key-in-file",
        ])
        .arg("--data-dir")
        .arg(&dir)
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !dir.join("ca.pem").exists() || !dir.join("proxy-address").exists() {
        assert!(Instant::now() < deadline, "it starts");
        std::thread::sleep(Duration::from_millis(20));
    }

    let out = env(&["--shell", "sh"]);
    let lines = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        lines.contains("export HTTPS_PROXY='http://127.0.0.1:20341'\n"),
        "{lines}"
    );
    let bundle = dir.join("terminal-roots.pem");
    let ca = std::fs::read_to_string(dir.join("ca.pem")).unwrap();
    assert!(std::fs::read_to_string(&bundle).unwrap().starts_with(&ca));
    // As a shell reads them.
    #[cfg(unix)]
    {
        let script = format!("{lines}printf '%s|%s' \"$https_proxy\" \"$SSL_CERT_FILE\"");
        let read = Command::new("/bin/sh")
            .args(["-c", &script])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&read.stdout),
            format!("http://127.0.0.1:20341|{}", bundle.display())
        );
    }
    let out = env(&["--shell", "powershell"]);
    assert!(
        String::from_utf8_lossy(&out.stdout)
            .contains("$env:HTTPS_PROXY = 'http://127.0.0.1:20341'"),
    );

    capture.kill().unwrap();
    capture.wait().unwrap();
    // A killed run leaves its address behind, where nothing listens now.
    assert!(dir.join("proxy-address").exists());
    let out = env(&[]);
    assert!(!out.status.success());
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains("nothing listens at 127.0.0.1:20341"),
        "{said}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// What the launchd agent runs on macOS: one process for the whole time the
/// setting is on, which has to sleep through a healthy app and wake the moment
/// it goes.
#[test]
fn recover_wait_waits_for_the_app_to_go() {
    let dir = std::env::temp_dir().join(format!("tracewing-wait-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let journal = dir.join("sysproxy.journal");
    journal_of_a_killed_run(&dir);

    let held = tracewing_system::hold_alive(&dir).unwrap();
    let mut waiting = recover(&dir)
        .arg("--wait")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        waiting.try_wait().unwrap().is_none(),
        "it waits for as long as the app is there"
    );
    assert!(journal.exists(), "and touches nothing meanwhile");

    drop(held);
    let until = Instant::now() + Duration::from_secs(20);
    while waiting.try_wait().unwrap().is_none() {
        assert!(
            Instant::now() < until,
            "it went on waiting after the app had gone"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = waiting.wait_with_output().unwrap();
    it_put_them_back_or_said_why_not(&out, &journal);

    std::fs::remove_dir_all(&dir).ok();
}

/// `pkill tracewing` matches every process whose name has tracewing in it, so
/// before the watchdog took a name of its own, the one process whose job was
/// to survive the app died with it — and the machine kept the proxy setting
/// and lost the network.
#[cfg(target_os = "linux")]
#[test]
fn the_watchdog_is_not_caught_by_a_kill_of_the_app() {
    let dir = std::env::temp_dir().join(format!("tracewing-watchdog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let mut watchdog = tracewing()
        .args(["watchdog", "--data-dir"])
        .arg(&dir)
        .env("HOME", &dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    // `/proc` has nothing to say about a process until its `exec` has gone
    // through, and on a loaded machine that takes a moment — long enough that
    // reading once made this test fail with an empty string and nothing to
    // say why. So everything read of it here is polled.
    fn until<T>(mut look: impl FnMut() -> Option<T>) -> Option<T> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(found) = look() {
                return Some(found);
            }
            if Instant::now() > deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    let pid = watchdog.id();
    // It hides from nobody: the command line is the one it was started with,
    // which `pkill -f tracewing` reads.
    let args = until(|| {
        let args = std::fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        (!args.is_empty()).then_some(args)
    });
    if let Some(ended) = watchdog.try_wait().unwrap() {
        panic!("the watchdog ended before anything could be read of it: {ended}");
    }
    let args = args.expect("a command line, once the watchdog has started");
    assert!(
        args.contains("tracewing"),
        "a command line `pkill -f tracewing` would find: {args:?}"
    );

    // And the name is its own, once it has renamed itself, which is the first
    // thing it does.
    let name = until(|| {
        let name = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        let name = name.trim().to_owned();
        (!name.is_empty() && name != "tracewing").then_some(name)
    })
    .unwrap_or_else(|| std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default());
    assert!(!name.trim().is_empty(), "the watchdog never started");
    assert!(
        !name.contains("tracewing"),
        "a name `pkill tracewing` would catch: {name}"
    );

    // The app going is what it waits for, and then it has nothing to do: no
    // journal, nothing to put back.
    drop(watchdog.stdin.take());
    assert!(watchdog.wait().unwrap().success());

    std::fs::remove_dir_all(&dir).ok();
}

/// What the installer for Windows runs when tracewing is removed, and what
/// anybody can run by hand before removing it: it undoes what tracewing did
/// to the computer and leaves alone what is the person's.
#[test]
fn uninstall_undoes_what_it_did_and_keeps_what_was_recorded() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-uninstall-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // A folder tracewing never used: there is nothing of its to undo.
    let out = in_its_own_folders(&dir, "uninstall").output().unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("no certificate authority"),
        "{out:?}"
    );

    // A folder as a run leaves one: an authority of its own, and a session.
    let recorded = tracewing()
        .args([
            "capture",
            "--ca-key-in-file",
            "--seconds",
            "1",
            "--data-dir",
        ])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(recorded.status.success(), "{recorded:?}");
    assert!(dir.join("ca.pem").exists(), "it made no authority");

    let out = in_its_own_folders(&dir, "uninstall").output().unwrap();
    assert!(out.status.success(), "{out:?}");
    let said = String::from_utf8_lossy(&out.stdout);
    // Which stores held it, or that none did: either way it is said, since a
    // line naming them is all anybody has to check this by afterwards.
    assert!(
        said.contains("trusts tracewing's authority") || said.contains("no longer trusts it"),
        "{said}"
    );
    // What was recorded is the person's: it says where it is and leaves it.
    assert!(said.contains("is still in"), "{said}");
    assert!(dir.join("sessions").is_dir(), "it took the sessions away");

    std::fs::remove_dir_all(&dir).ok();
}

/// The watchdog's whole reason to exist, which nothing tested: the app goes,
/// and whatever it did to the system proxy is undone. The app going is its
/// end of the pipe closing, which happens however it died — a kill, a panic,
/// a power of the plug.
///
/// On a machine with no desktop to read the setting from there is nothing to
/// put back, so it says so and keeps the journal, exactly as `recover` does:
/// see `it_put_them_back_or_said_why_not`.
#[cfg(target_os = "linux")]
#[test]
fn the_watchdog_puts_the_setting_back_when_the_app_goes() {
    let dir = std::env::temp_dir().join(format!("tracewing-watchdog-job-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let journal = dir.join("sysproxy.journal");
    journal_of_a_killed_run(&dir);

    let mut watchdog = tracewing()
        .args(["watchdog", "--data-dir"])
        .arg(&dir)
        .env("HOME", &dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    // The app going, whatever took it: its end of the pipe closes.
    drop(watchdog.stdin.take());
    let ended = watchdog.wait().unwrap();

    let log = std::fs::read_to_string(dir.join("watchdog.log")).unwrap_or_default();
    assert!(
        log.contains("app gone"),
        "it writes down what it found, either way: {log:?}"
    );
    if ended.success() {
        assert!(
            !journal.exists(),
            "the journal goes with the settings, so it cannot be used twice"
        );
    } else {
        assert!(
            journal.exists(),
            "nothing could be read, so the journal stays: it is all that is \
             left of what the settings were"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}
