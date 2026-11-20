//! S5, browsers: Chrome, Edge and Firefox, headless, open a page whose script
//! speaks HTTP/1.1, HTTP/2, a WebSocket and server-sent events, and reaches
//! the Set up test page; once through the profile of tracewing's own that
//! Set up opens them in, once through the system proxy. Each time the
//! browser says each one worked, the HAR holds each one, and no datagram
//! reaches the server's port: with HTTP/3 taken out of `Alt-Svc`, nothing
//! tries it.
//!
//! The browsers are found the way Set up finds them. Which must be there is
//! `TRACEWING_E2E_BROWSERS` (`chrome firefox`, `chrome edge firefox`), which
//! the container or the script that has them sets; without it, this says so
//! and does nothing.
//!
//! Changes this computer's trust stores and proxy setting. See
//! `tracewing_e2e`.
#![cfg(any(target_os = "linux", windows))]

use serde_json::Value;
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};
use tracewing_e2e::{
    Capture, desktop, har_of,
    helper::Helper,
    only_where_allowed, program, said, scratch,
    site::{EVENTS, PING, Site},
    wait_for,
};
use tracewing_proxy::ca_key::KeyStore;
use tracewing_system::{
    Scope, ca_install, ca_remove,
    launch::{Browser, browser_command, find_browsers},
};

/// Takes the CA out of every store, however the test ends.
struct Untrust(PathBuf);

impl Drop for Untrust {
    fn drop(&mut self) {
        let _ = ca_remove(&self.0);
    }
}

/// A browser running, with every process it started: killed together when
/// dropped.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        // Its process group on Unix, which its helpers are in too; its tree
        // of processes on Windows.
        #[cfg(unix)]
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", self.0.id())])
            .status();
        #[cfg(windows)]
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &self.0.id().to_string()])
            .output();
        let _ = self.0.wait();
    }
}

fn start(mut command: Command) -> Running {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    let program = command.get_program().to_owned();
    Running(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("start {}: {e}", program.display())),
    )
}

/// What a headless Chromium needs besides what Set up gives it. In the
/// Linux container: no sandbox of Chrome's own, which needs what a container
/// is not given, and no `/dev/shm` to speak of.
fn chrome_headless() -> Vec<String> {
    let mut flags = vec!["--headless=new", "--disable-gpu"];
    if cfg!(target_os = "linux") {
        flags.extend(["--no-sandbox", "--disable-dev-shm-usage"]);
    }
    flags.into_iter().map(String::from).collect()
}

/// The browser called `name` that Set up finds, which must be there.
fn found(name: &str, data: &Path) -> Browser {
    let browsers = find_browsers(data);
    browsers
        .iter()
        .find(|browser| browser.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("no {name} among the browsers found: {browsers:?}"))
}

/// Whether this machine is to have `engine`: if not, the test says so and
/// ends.
fn wanted(engine: &str) -> bool {
    let Ok(list) = std::env::var("TRACEWING_E2E_BROWSERS") else {
        eprintln!("skipped: TRACEWING_E2E_BROWSERS names no browser to run here");
        return false;
    };
    if !list.split_whitespace().any(|name| name == engine) {
        eprintln!("skipped: TRACEWING_E2E_BROWSERS ({list}) has no {engine}");
        return false;
    }
    true
}

/// Waits for what the page says, then for the HAR to hold all it did.
fn report_of(site: &Site, browser: &str) -> BTreeMap<String, String> {
    let mut report = None;
    wait_for(
        &format!("{browser} to say how the page went"),
        Duration::from_secs(60),
        || {
            report = site.take_report();
            report.is_some()
        },
    );
    report.unwrap()
}

/// What the browser said, and what tracewing recorded of it.
fn check(site: &Site, capture: &Capture, browser: &str, report: &BTreeMap<String, String>) {
    let get = |name: &str| report.get(name).map(String::as_str).unwrap_or("");
    let said = format!("{browser} said {report:?}");
    assert!(
        get("json").starts_with("answered") || get("json").starts_with("over"),
        "{said}"
    );
    assert_eq!(get("ws"), PING, "the WebSocket: {said}");
    assert_eq!(get("sse"), EVENTS.join(","), "the events: {said}");
    assert_eq!(get("check"), "reached", "the Set up test page: {said}");
    assert!(
        !get("alt_svc").contains("h3"),
        "HTTP/3 was offered to the browser: {said}"
    );

    let page = site.page();
    let tls = site.tls_origin();
    let ws = tls.replacen("https://", "wss://", 1) + "/ws";
    let mut har = String::new();
    let mut entries = Vec::new();
    let all_there = |entries: &[Value]| {
        let has = |url: &str| entries.iter().any(|e| e["request"]["url"] == url);
        has(&page)
            && has(&format!("{tls}/json"))
            && has(&format!("{tls}/sse"))
            && (has(&ws) || has(&format!("{tls}/ws")))
    };
    wait_for(
        &format!("{browser}'s flows in the HAR"),
        Duration::from_secs(15),
        || {
            har = har_of(&program(), &capture.data_dir);
            let parsed: Value = serde_json::from_str(&har).unwrap();
            entries = parsed["log"]["entries"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            all_there(&entries)
        },
    );
    let entry = |url: &str| {
        entries
            .iter()
            .find(|e| e["request"]["url"] == url)
            .unwrap_or_else(|| panic!("no {url} in the HAR: {har}"))
    };
    let version = |e: &Value| {
        e["request"]["httpVersion"]
            .as_str()
            .unwrap_or("")
            .to_owned()
    };
    assert!(
        version(entry(&page)).contains("1.1"),
        "the page: {}",
        entry(&page)
    );
    let json = entry(&format!("{tls}/json"));
    assert!(version(json).contains('2'), "not HTTP/2: {json}");
    assert!(
        json["response"]["content"]["text"]
            .as_str()
            .is_some_and(|text| text.contains("answered")),
        "the answer was not decrypted: {json}"
    );
    let sse = entry(&format!("{tls}/sse"));
    let events = sse["response"]["content"]["text"].as_str().unwrap_or("");
    for event in EVENTS {
        assert!(events.contains(event), "no {event} among the events: {sse}");
    }
    let socket = entries
        .iter()
        .find(|e| e["request"]["url"] == ws.as_str() || e["request"]["url"] == format!("{tls}/ws"))
        .unwrap();
    let messages = socket["_webSocketMessages"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        messages.iter().filter(|m| m["data"] == PING).count() >= 2,
        "the WebSocket's messages, sent and echoed, are not in the HAR: {socket}"
    );
    assert_eq!(site.datagrams(), 0, "{browser} tried HTTP/3");
}

/// A capture that trusts `site`'s authority, its own CA trusted by this user
/// as Set up's Install does, which is what Chromium reads.
fn capture_for(dir: &Path, name: &str, site: &Site) -> (Capture, Untrust) {
    let capture = Capture::start_trusting(&program(), &dir.join(name), &site.ca, KeyStore::File);
    let untrust = Untrust(capture.ca_pem());
    ca_install(&capture.ca_pem(), Scope::User).unwrap();
    (capture, untrust)
}

fn addr(capture: &Capture) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], capture.port))
}

/// A browser of Chromium's family, `name` as Set up lists it.
fn chromium(name: &str) {
    let dir = scratch(&name.to_lowercase());
    let site = Site::start(&dir);

    // Set up's own profile.
    {
        let (capture, _untrust) = capture_for(&dir, "own", &site);
        let chrome = found(name, &capture.data_dir);
        let mut command = browser_command(
            &chrome,
            addr(&capture),
            None,
            &capture.ca_pem(),
            &site.page(),
        )
        .unwrap();
        command.args(chrome_headless());
        let running = start(command);
        let report = report_of(&site, name);
        drop(running);
        check(
            &site,
            &capture,
            &format!("{name}, in its own profile"),
            &report,
        );

        // The test page itself, as the browser shows it.
        let dump = Browser {
            profile: dir.join("dump"),
            ..chrome.clone()
        };
        let out = browser_command(
            &dump,
            addr(&capture),
            None,
            &capture.ca_pem(),
            "https://tracewing.test/",
        )
        .unwrap()
        .args(chrome_headless())
        .arg("--dump-dom")
        .output()
        .unwrap();
        let dom = String::from_utf8_lossy(&out.stdout);
        assert!(
            dom.contains("It works") && dom.contains("trusts tracewing's certificate"),
            "the Set up test page: {}",
            said(&out)
        );
    }

    // The system proxy, which it reads from the desktop's settings.
    {
        let _put_back = desktop::the_users_own();
        let (capture, _untrust) = capture_for(&dir, "system", &site);
        let chrome = found(name, &capture.data_dir);
        let holder = Helper::system_proxy(&capture.data_dir, capture.port, true);
        let mut command = Command::new(&chrome.program);
        command
            .arg(format!("--user-data-dir={}", dir.join("plain").display()))
            .args(["--no-first-run", "--no-default-browser-check"])
            .args(chrome_headless())
            .arg(site.page());
        let running = start(command);
        let report = report_of(&site, name);
        drop(running);
        check(
            &site,
            &capture,
            &format!("{name}, through the system proxy"),
            &report,
        );
        assert!(holder.let_go().success());
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn chrome_records_everything_in_its_own_profile_and_through_the_system_proxy() {
    only_where_allowed!();
    if wanted("chrome") {
        chromium("Chrome");
    }
}

#[test]
fn edge_records_everything_in_its_own_profile_and_through_the_system_proxy() {
    only_where_allowed!();
    if wanted("edge") {
        chromium("Edge");
    }
}

#[test]
fn firefox_records_everything_in_its_own_profile_and_through_the_system_proxy() {
    only_where_allowed!();
    if !wanted("firefox") {
        return;
    }
    let dir = scratch("firefox");
    let site = Site::start(&dir);

    // Set up's own profile, which is told to trust the CA itself.
    {
        let (capture, _untrust) = capture_for(&dir, "own", &site);
        let firefox = found("Firefox", &capture.data_dir);
        let mut command = browser_command(
            &firefox,
            addr(&capture),
            None,
            &capture.ca_pem(),
            &site.page(),
        )
        .unwrap();
        command.env("MOZ_HEADLESS", "1");
        let running = start(command);
        let report = report_of(&site, "Firefox");
        drop(running);
        check(&site, &capture, "Firefox, in its own profile", &report);
    }

    // The system proxy. A profile of the user's. On Linux Firefox has made
    // its database before tracewing's CA is installed, as a user's has, and
    // Install puts the CA in it; on Windows Firefox reads the system's
    // roots.
    {
        let profile = user_profile(&dir);
        let _gone = RemoveDir(profile.clone());
        std::fs::create_dir_all(&profile).unwrap();
        #[cfg(target_os = "linux")]
        {
            let out = Command::new("certutil")
                .args(["-N", "--empty-password", "-d"])
                .arg(format!("sql:{}", profile.display()))
                .output()
                .unwrap();
            assert!(out.status.success(), "{}", said(&out));
        }
        // Only what keeps a first run quiet; the proxy is the system's, as
        // Firefox's is by default.
        std::fs::write(
            profile.join("user.js"),
            "user_pref(\"browser.shell.checkDefaultBrowser\", false);\n\
             user_pref(\"browser.aboutwelcome.enabled\", false);\n\
             user_pref(\"datareporting.policy.dataSubmissionEnabled\", false);\n",
        )
        .unwrap();

        let _put_back = desktop::the_users_own();
        let (capture, _untrust) = capture_for(&dir, "system", &site);
        let firefox = found("Firefox", &capture.data_dir);
        let holder = Helper::system_proxy(&capture.data_dir, capture.port, true);
        let mut command = Command::new(&firefox.program);
        command
            .arg("--profile")
            .arg(&profile)
            .args(["--no-remote", "--new-instance"])
            .arg(site.page())
            .env("MOZ_HEADLESS", "1");
        let running = start(command);
        let report = report_of(&site, "Firefox");
        drop(running);
        if kde() {
            // A gap, measured: Firefox reads the system proxy from GNOME's
            // settings only, which tracewing leaves alone on KDE, so it goes
            // straight to the server and cannot check its certificate.
            // Set up's own profile, above, is the way through. Once this
            // fails, Firefox goes through: check it as on GNOME.
            assert!(
                report
                    .get("json")
                    .is_some_and(|json| json.starts_with("error")),
                "Firefox follows KDE's proxy setting now: check it as on GNOME. It said {report:?}"
            );
            eprintln!("known gap: Firefox on KDE does not follow the system proxy");
        } else {
            check(
                &site,
                &capture,
                "Firefox, through the system proxy",
                &report,
            );
        }
        assert!(holder.let_go().success());
    }
    std::fs::remove_dir_all(&dir).ok();
}

/// Where a profile of the user's own is: where Firefox looks for them on
/// Linux, anywhere on Windows.
fn user_profile(dir: &Path) -> PathBuf {
    if cfg!(target_os = "linux") {
        PathBuf::from(std::env::var_os("HOME").unwrap()).join(".mozilla/firefox/e2e.default")
    } else {
        dir.join("firefox-user")
    }
}

fn kde() -> bool {
    #[cfg(target_os = "linux")]
    return desktop::kde();
    #[cfg(not(target_os = "linux"))]
    false
}

/// A folder of the test's in the user's home, gone when the test ends.
struct RemoveDir(PathBuf);

impl Drop for RemoveDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
