//! `tracewing capture`, `export`, `sessions` and `recover`, as a terminal runs
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
    let out = tracewing()
        .args(["recover", "--data-dir"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("nothing to put back"),
        "{out:?}"
    );

    // A journal from a run that was killed, naming settings that are not on
    // this machine now: it must not put somebody else's settings back.
    // The keys are the journal's own: `addr`, and `saved.` for each setting
    // as it was before tracewing touched it.
    std::fs::write(
        dir.join("sysproxy.journal"),
        "addr=127.0.0.1:9090\nsaved.mode='none'\n",
    )
    .unwrap();
    let out = tracewing()
        .args(["recover", "--data-dir"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(
        said.contains("left alone") || said.contains("back as it was"),
        "it says which of the two happened: {said}"
    );
    assert!(
        !dir.join("sysproxy.journal").exists(),
        "and the journal goes either way, so it cannot be used twice"
    );

    std::fs::remove_dir_all(&dir).ok();
}
