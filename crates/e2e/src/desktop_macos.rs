//! macOS's proxy setting, read and written with `networksetup`, for every
//! network service that is on, as tracewing takes it: the web and secure web
//! proxies, the bypass list, the automatic configuration and discovery.

use std::process::Command;

/// The whole setting, by `service.what`.
pub type Setting = Vec<(String, String)>;

fn networksetup(args: &[&str]) -> String {
    tried(args).unwrap_or_else(|e| panic!("{e}"))
}

/// `networksetup` that says why rather than panicking, for `PutBack`, which
/// may run while a test is already panicking.
fn tried(args: &[&str]) -> Result<String, String> {
    let out = Command::new("/usr/sbin/networksetup")
        .args(args)
        .output()
        .map_err(|e| format!("run networksetup: {e}"))?;
    let said = String::from_utf8_lossy(&out.stdout).into_owned();
    // Some failures are on stdout, with 0.
    if !out.status.success() || said.starts_with("** Error") {
        return Err(format!(
            "networksetup {args:?}: {}{}",
            String::from_utf8_lossy(&out.stderr),
            said
        ));
    }
    Ok(said)
}

/// The services that are on: the first line says what `*` means, and a `*`
/// marks one that is off.
fn services() -> Vec<String> {
    listed(networksetup(&["-listallnetworkservices"]))
}

fn listed(out: String) -> Vec<String> {
    out.lines()
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
    read().unwrap_or_else(|e| panic!("{e}"))
}

fn read() -> Result<Setting, String> {
    let mut setting = Setting::new();
    for service in listed(tried(&["-listallnetworkservices"])?) {
        let mut put =
            |what: &str, value: String| setting.push((format!("{service}.{what}"), value));
        for (key, verb) in [("web", "-getwebproxy"), ("secure", "-getsecurewebproxy")] {
            let out = tried(&[verb, &service])?;
            put(&format!("{key}.enabled"), field(&out, "Enabled"));
            put(&format!("{key}.server"), field(&out, "Server"));
            put(&format!("{key}.port"), field(&out, "Port"));
        }
        let auto = tried(&["-getautoproxyurl", &service])?;
        put("auto.url", field(&auto, "URL"));
        put("auto.enabled", field(&auto, "Enabled"));
        let discovery = tried(&["-getproxyautodiscovery", &service])?;
        put("discovery", field(&discovery, "Auto Proxy Discovery"));
        let bypass = tried(&["-getproxybypassdomains", &service])?;
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
    Ok(setting)
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
/// test ends however it ends: the machine is left as the test found it, and
/// the test fails when it is not.
pub struct PutBack(Setting);

impl PutBack {
    /// Writes every part of the setting as it was, an empty server too:
    /// leaving that out once left a test's proxy on a Mac, turned on for one
    /// service. Says what did not take.
    fn write(&self) -> Vec<String> {
        let was = &self.0;
        let mut failed = Vec::new();
        let mut set = |args: &[&str]| {
            if let Err(e) = tried(args) {
                failed.push(e);
            }
        };
        let services: Vec<String> = was
            .iter()
            .filter_map(|(k, _)| k.strip_suffix(".web.enabled"))
            .map(str::to_owned)
            .collect();
        for service in &services {
            let at = |what: &str| {
                was.iter()
                    .find(|(k, _)| *k == format!("{service}.{what}"))
                    .map(|(_, v)| v.as_str())
                    .unwrap_or("")
            };
            for (key, verb, state) in [
                ("web", "-setwebproxy", "-setwebproxystate"),
                ("secure", "-setsecurewebproxy", "-setsecurewebproxystate"),
            ] {
                // An empty server clears it. This also turns the protocol
                // on, so the state comes after it.
                let server = at(&format!("{key}.server"));
                let port = at(&format!("{key}.port"));
                let port = if server.is_empty() { "" } else { port };
                set(&[verb, service, server, port]);
                set(&[state, service, on_off(at(&format!("{key}.enabled")))]);
            }
            let bypass = at("bypass");
            let mut args = vec!["-setproxybypassdomains", service.as_str()];
            if bypass.is_empty() {
                args.push("Empty");
            } else {
                args.extend(bypass.split(' '));
            }
            set(&args);
            let url = at("auto.url");
            if !url.is_empty() && url != "(null)" {
                set(&["-setautoproxyurl", service, url]);
            }
            set(&["-setautoproxystate", service, on_off(at("auto.enabled"))]);
            set(&["-setproxyautodiscovery", service, on_off(at("discovery"))]);
        }
        failed
    }
}

impl Drop for PutBack {
    fn drop(&mut self) {
        let mut failed = self.write();
        match read() {
            Ok(now) if now == self.0 => {}
            Ok(now) => failed.push(format!(
                "not as it was: {:?}",
                now.iter()
                    .zip(&self.0)
                    .filter(|(now, was)| now != was)
                    .map(|((k, now), (_, was))| format!("{k}: {now:?}, was {was:?}"))
                    .collect::<Vec<_>>()
            )),
            Err(e) => failed.push(e),
        }
        if failed.is_empty() {
            return;
        }
        let said = format!("the proxy setting was not put back: {}", failed.join("; "));
        // A second panic would abort, and hide the first.
        if std::thread::panicking() {
            eprintln!("{said}");
        } else {
            panic!("{said}");
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

/// No proxy at all on every service, as on a Mac where nobody ever set one:
/// no server, everything off. Keep what it gives until the test ends.
#[must_use = "the setting is put back when this is dropped"]
pub fn none_set() -> PutBack {
    let before = PutBack(setting());
    for service in services() {
        for (verb, state) in [
            ("-setwebproxy", "-setwebproxystate"),
            ("-setsecurewebproxy", "-setsecurewebproxystate"),
        ] {
            networksetup(&[verb, &service, "", ""]);
            networksetup(&[state, &service, "off"]);
        }
        networksetup(&["-setautoproxystate", &service, "off"]);
        networksetup(&["-setproxyautodiscovery", &service, "off"]);
    }
    before
}
