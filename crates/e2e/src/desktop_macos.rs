//! macOS's proxy setting, read and written with `networksetup`, for every
//! network service that is on, as tracewing takes it: the web and secure web
//! proxies, the bypass list, the automatic configuration and discovery.

use std::process::Command;

/// The whole setting, by `service.what`.
pub type Setting = Vec<(String, String)>;

fn networksetup(args: &[&str]) -> String {
    let out = Command::new("/usr/sbin/networksetup")
        .args(args)
        .output()
        .expect("run networksetup");
    assert!(
        out.status.success(),
        "networksetup {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The services that are on: the first line says what `*` means, and a `*`
/// marks one that is off.
fn services() -> Vec<String> {
    networksetup(&["-listallnetworkservices"])
        .lines()
        .skip(1)
        .filter(|line| !line.starts_with('*') && !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn field(text: &str, name: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix(": "))
        .unwrap_or("")
        .trim()
        .to_owned()
}

pub fn setting() -> Setting {
    let mut setting = Setting::new();
    for service in services() {
        let mut put =
            |what: &str, value: String| setting.push((format!("{service}.{what}"), value));
        for (key, verb) in [("web", "-getwebproxy"), ("secure", "-getsecurewebproxy")] {
            let out = networksetup(&[verb, &service]);
            put(&format!("{key}.enabled"), field(&out, "Enabled"));
            put(&format!("{key}.server"), field(&out, "Server"));
            put(&format!("{key}.port"), field(&out, "Port"));
        }
        let auto = networksetup(&["-getautoproxyurl", &service]);
        put("auto.url", field(&auto, "URL"));
        put("auto.enabled", field(&auto, "Enabled"));
        let discovery = networksetup(&["-getproxyautodiscovery", &service]);
        put("discovery", field(&discovery, "Auto Proxy Discovery"));
        let bypass = networksetup(&["-getproxybypassdomains", &service]);
        // Without a list it says so in a sentence.
        put(
            "bypass",
            if bypass.contains("aren't any") {
                String::new()
            } else {
                bypass.lines().collect::<Vec<_>>().join(" ")
            },
        );
    }
    setting
}

pub fn get<'a>(setting: &'a Setting, key: &str) -> &'a str {
    &setting.iter().find(|(k, _)| k == key).unwrap().1
}

/// Whether every service sends web and secure web traffic to tracewing on
/// `port`.
pub fn names_tracewing(setting: &Setting, port: u16) -> bool {
    let port = port.to_string();
    let services = services();
    !services.is_empty()
        && services.iter().all(|service| {
            ["web", "secure"].iter().all(|key| {
                get(setting, &format!("{service}.{key}.enabled")) == "Yes"
                    && get(setting, &format!("{service}.{key}.server")) == "127.0.0.1"
                    && get(setting, &format!("{service}.{key}.port")) == port
            })
        })
}

fn on_off(value: &str) -> &'static str {
    if value == "Yes" || value == "On" {
        "on"
    } else {
        "off"
    }
}

/// What the setting was before a test changed it, written back when the
/// test ends however it ends: the machine is left as the test found it.
pub struct PutBack(Setting);

impl Drop for PutBack {
    fn drop(&mut self) {
        let was = &self.0;
        for service in services() {
            let at = |what: &str| {
                was.iter()
                    .find(|(k, _)| *k == format!("{service}.{what}"))
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("")
            };
            for (key, set, state) in [
                ("web", "-setwebproxy", "-setwebproxystate"),
                ("secure", "-setsecurewebproxy", "-setsecurewebproxystate"),
            ] {
                let server = at(&format!("{key}.server"));
                if !server.is_empty() {
                    let _ = Command::new("/usr/sbin/networksetup")
                        .args([set, &service, server, at(&format!("{key}.port"))])
                        .status();
                }
                let _ = Command::new("/usr/sbin/networksetup")
                    .args([state, &service, on_off(at(&format!("{key}.enabled")))])
                    .status();
            }
            let bypass = at("bypass");
            let mut args = vec!["-setproxybypassdomains", service.as_str()];
            if bypass.is_empty() {
                args.push("Empty");
            } else {
                args.extend(bypass.split(' '));
            }
            let _ = Command::new("/usr/sbin/networksetup").args(&args).status();
            let url = at("auto.url");
            if !url.is_empty() && url != "(null)" {
                let _ = Command::new("/usr/sbin/networksetup")
                    .args(["-setautoproxyurl", &service, url])
                    .status();
            }
            let _ = Command::new("/usr/sbin/networksetup")
                .args(["-setautoproxystate", &service, on_off(at("auto.enabled"))])
                .status();
            let _ = Command::new("/usr/sbin/networksetup")
                .args(["-setproxyautodiscovery", &service, on_off(at("discovery"))])
                .status();
        }
    }
}

/// A proxy of the user's own on every service, so that "as it was" is more
/// than the defaults. Keep what it gives until the test ends.
#[must_use = "the setting is put back when this is dropped"]
pub fn the_users_own() -> PutBack {
    let before = PutBack(setting());
    for service in services() {
        networksetup(&["-setwebproxy", &service, "corp.example", "3128"]);
        networksetup(&["-setsecurewebproxy", &service, "corp.example", "3128"]);
        networksetup(&["-setproxybypassdomains", &service, "intranet.example"]);
    }
    before
}
