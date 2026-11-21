//! The iOS Simulator, as Set up sets it up on a Mac: tracewing's CA put among
//! the simulator's trusted roots (`ios_sim::trust`, what "Set up the
//! simulators" runs) and the Mac's system proxy pointed at tracewing, which a
//! simulator goes through. Then Safari in the simulator opens S5's page,
//! whose script fetches over HTTPS, opens a WebSocket, reads server-sent
//! events and reaches the Set up test page, through tracewing.
//!
//! A simulator of the test's own, made for it and deleted after, so that one
//! somebody uses keeps no certificate of a test's. It needs Xcode with an iOS
//! runtime, which the macOS runners have; anywhere without one it says so.
//!
//! Changes this Mac's proxy setting. See `tracewing_e2e`.
#![cfg(target_os = "macos")]

use std::{collections::BTreeMap, path::Path, process::Command, time::Duration};
use tracewing_e2e::{
    Capture, desktop, har_of,
    helper::Helper,
    only_where_allowed, program, scratch,
    site::{EVENTS, PING, Site},
    wait_for,
};
use tracewing_proxy::ca_key::KeyStore;
use tracewing_system::ios_sim;

fn simctl(xcrun: &Path, args: &[&str]) -> String {
    let out = Command::new(xcrun)
        .arg("simctl")
        .args(args)
        .output()
        .expect("run xcrun simctl");
    assert!(
        out.status.success(),
        "simctl {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// The newest iOS runtime installed, and an iPhone it can run, by
/// identifier; `None` where Xcode has none.
fn an_iphone(xcrun: &Path) -> Option<(String, String)> {
    let runtimes: serde_json::Value =
        serde_json::from_str(&simctl(xcrun, &["list", "runtimes", "available", "-j"])).ok()?;
    let runtime = runtimes["runtimes"]
        .as_array()?
        .iter()
        .filter(|r| r["platform"] == "iOS" && r["isAvailable"] == true)
        .max_by_key(|r| r["version"].as_str().unwrap_or("").to_owned())?;
    let iphone = runtime["supportedDeviceTypes"]
        .as_array()?
        .iter()
        .rev()
        .find(|t| t["productFamily"] == "iPhone")?;
    Some((
        runtime["identifier"].as_str()?.to_owned(),
        iphone["identifier"].as_str()?.to_owned(),
    ))
}

/// The name the site is asked for by.
const NAME: &str = "e2e-site.test";

/// The name in /etc/hosts, taken out again however the test ends. With
/// sudo, which a runner gives without asking and a Mac asks for in the
/// terminal the test runs in.
struct Hosts;

impl Hosts {
    fn add(ip: std::net::IpAddr) -> Self {
        let line = format!("{ip} {NAME}");
        let status = Command::new("sudo")
            .args(["sh", "-c", &format!("echo '{line}' >> /etc/hosts")])
            .status()
            .unwrap();
        assert!(status.success(), "add {NAME} to /etc/hosts");
        flush();
        Hosts
    }
}

impl Drop for Hosts {
    fn drop(&mut self) {
        let _ = Command::new("sudo")
            .args(["sed", "-i", "", &format!("/ {NAME}$/d"), "/etc/hosts"])
            .status();
        flush();
    }
}

/// What macOS has cached of names, forgotten, so /etc/hosts is read again.
fn flush() {
    let _ = Command::new("sudo")
        .args(["dscacheutil", "-flushcache"])
        .status();
    let _ = Command::new("sudo")
        .args(["killall", "-HUP", "mDNSResponder"])
        .status();
}

/// The test's simulator, shut down and deleted however the test ends.
struct Simulator<'a> {
    xcrun: &'a Path,
    udid: String,
}

impl Drop for Simulator<'_> {
    fn drop(&mut self) {
        let _ = Command::new(self.xcrun)
            .args(["simctl", "shutdown", &self.udid])
            .output();
        let _ = Command::new(self.xcrun)
            .args(["simctl", "delete", &self.udid])
            .output();
    }
}

#[test]
fn a_simulator_set_up_reaches_everything_through_tracewing() {
    only_where_allowed!();
    let Some(xcrun) = ios_sim::find_xcrun() else {
        eprintln!("skipped: no xcrun here");
        return;
    };
    let Some((runtime, device)) = an_iphone(&xcrun) else {
        eprintln!("skipped: Xcode has no iOS runtime here");
        return;
    };
    let udid = simctl(&xcrun, &["create", "tracewing-e2e", &device, &runtime]);
    let sim = Simulator {
        xcrun: &xcrun,
        udid,
    };
    simctl(&xcrun, &["boot", &sim.udid]);
    simctl(&xcrun, &["bootstatus", &sim.udid, "-b"]);

    let dir = scratch("ios-simulator");
    // Asked for by a name, which the proxy resolves from /etc/hosts. By its
    // address alone, Safari's HTTPS to the site never reached tracewing at
    // all, while Chrome and Firefox's do (S5): which of the two -- the
    // address or the port -- it minds is what this run says.
    let site = Site::start_named(&dir, Some(NAME));
    let _hosts = Hosts::add(site.ip);
    let capture = Capture::start_trusting(&program(), &dir.join("data"), &site.ca, KeyStore::File);
    // What "Set up the simulators" does: the CA among its roots, and the
    // Mac's proxy, which it goes through, pointed at tracewing.
    ios_sim::trust(&xcrun, &sim.udid, &capture.ca_pem()).unwrap();
    let _put_back = desktop::the_users_own();
    let holder = Helper::system_proxy(&capture.data_dir, capture.port, true);

    // A simulator just booted can take longer to start Safari than
    // `openurl` waits ("Operation timed out"), and Safari may open the page
    // all the same: tried again, and what the page says is what counts.
    for attempt in 1..=3 {
        let out = Command::new(&xcrun)
            .args(["simctl", "openurl", &sim.udid, &site.page()])
            .output()
            .unwrap();
        if out.status.success() {
            break;
        }
        eprintln!(
            "openurl, try {attempt}: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .next()
                .unwrap_or("")
        );
        std::thread::sleep(Duration::from_secs(10));
    }
    let mut report: Option<BTreeMap<String, String>> = None;
    wait_for(
        "Safari in the simulator to say how the page went",
        Duration::from_secs(180),
        || {
            report = site.take_report();
            report.is_some()
        },
    );
    let report = report.unwrap();
    let get = |name: &str| report.get(name).map(String::as_str).unwrap_or("");
    // What reached tracewing, by URL and status: whether a request that
    // failed in Safari got as far as the proxy at all.
    let har: serde_json::Value =
        serde_json::from_str(&har_of(&program(), &capture.data_dir)).unwrap();
    let reached: Vec<String> = har["log"]["entries"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .map(|e| format!("{} {}", e["response"]["status"], e["request"]["url"]))
                .collect()
        })
        .unwrap_or_default();
    let said = format!("Safari in the simulator said {report:?}; tracewing recorded {reached:?}");
    assert!(
        get("json").starts_with("answered") || get("json").starts_with("over"),
        "HTTPS through tracewing, with its CA trusted: {said}"
    );
    assert_eq!(get("ws"), PING, "the WebSocket: {said}");
    assert_eq!(get("sse"), EVENTS.join(","), "the events: {said}");
    assert_eq!(get("check"), "reached", "the Set up test page: {said}");
    assert!(
        get("user_agent").contains("iPhone"),
        "it was the simulator that asked: {said}"
    );
    assert!(holder.let_go().success());
    drop(capture);
    drop(sim);
    std::fs::remove_dir_all(&dir).ok();
}
