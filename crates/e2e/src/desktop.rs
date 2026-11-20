//! A Linux desktop's proxy setting, read and written with the desktop's own
//! tools rather than tracewing's, so that what tracewing reads is checked too.
//! GNOME keeps it in dconf (`gsettings`), KDE in `kioslaverc` (kconfig).

use std::process::Command;

fn run(program: &str, args: &[&str]) -> String {
    let out = Command::new(program).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Whether the session says it is KDE, as tracewing reads it.
pub fn kde() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|names| names.split(':').any(|n| n == "KDE"))
}

const GNOME: &[(&str, &str)] = &[
    ("org.gnome.system.proxy", "mode"),
    ("org.gnome.system.proxy", "ignore-hosts"),
    ("org.gnome.system.proxy", "autoconfig-url"),
    ("org.gnome.system.proxy.http", "host"),
    ("org.gnome.system.proxy.http", "port"),
    ("org.gnome.system.proxy.https", "host"),
    ("org.gnome.system.proxy.https", "port"),
];

const KDE: &[&str] = &["ProxyType", "httpProxy", "httpsProxy", "NoProxyFor"];

fn kde_tool(verb: &str) -> String {
    // Looked for on the PATH as tracewing looks: not every system has
    // `which` (Fedora's container has none).
    let path = std::env::var_os("PATH").unwrap_or_default();
    for tool in [format!("k{verb}config6"), format!("k{verb}config5")] {
        if std::env::split_paths(&path).any(|dir| dir.join(&tool).is_file()) {
            return tool;
        }
    }
    panic!("no k{verb}config on this KDE");
}

fn kde_args(key: &str) -> [&str; 6] {
    [
        "--file",
        "kioslaverc",
        "--group",
        "Proxy Settings",
        "--key",
        key,
    ]
}

/// The whole setting, by key.
pub type Setting = Vec<(String, String)>;

pub fn setting() -> Setting {
    if kde() {
        let tool = kde_tool("read");
        KDE.iter()
            .map(|key| (key.to_string(), run(&tool, &kde_args(key))))
            .collect()
    } else {
        GNOME
            .iter()
            .map(|(schema, key)| {
                (
                    format!("{schema} {key}"),
                    run("gsettings", &["get", schema, key]),
                )
            })
            .collect()
    }
}

pub fn get<'a>(setting: &'a Setting, key: &str) -> &'a str {
    &setting.iter().find(|(k, _)| k == key).unwrap().1
}

/// Whether the setting sends both HTTP and HTTPS to tracewing on `port`.
pub fn names_tracewing(setting: &Setting, port: u16) -> bool {
    if kde() {
        let url = format!("http://127.0.0.1 {port}");
        get(setting, "ProxyType") == "1"
            && get(setting, "httpProxy") == url
            && get(setting, "httpsProxy") == url
    } else {
        get(setting, "org.gnome.system.proxy mode") == "'manual'"
            && ["http", "https"].iter().all(|scheme| {
                let schema = format!("org.gnome.system.proxy.{scheme}");
                get(setting, &format!("{schema} host")) == "'127.0.0.1'"
                    && get(setting, &format!("{schema} port")) == port.to_string()
            })
    }
}

/// What the setting was before a test changed it, written back when the
/// test ends however it ends: the machine is left as the test found it.
pub struct PutBack(Setting);

impl Drop for PutBack {
    fn drop(&mut self) {
        if kde() {
            let tool = kde_tool("write");
            for (key, value) in &self.0 {
                let mut args = kde_args(key).to_vec();
                if value.is_empty() {
                    args.push("--delete");
                } else {
                    args.push(value);
                }
                let _ = Command::new(&tool).args(&args).status();
            }
        } else {
            for (name, value) in &self.0 {
                let (schema, key) = name.split_once(' ').unwrap();
                let _ = Command::new("gsettings")
                    .args(["set", schema, key, value])
                    .status();
            }
        }
    }
}

/// A proxy of the user's own, so that "as it was" is more than the defaults.
/// Keep what it gives until the test ends.
#[must_use = "the setting is put back when this is dropped"]
pub fn the_users_own() -> PutBack {
    let before = PutBack(setting());
    if kde() {
        let tool = kde_tool("write");
        for (key, value) in [
            ("ProxyType", "1"),
            ("httpProxy", "http://corp.example 3128"),
            ("httpsProxy", "http://corp.example 3128"),
            ("NoProxyFor", "intranet.example"),
        ] {
            let mut args = kde_args(key).to_vec();
            args.push(value);
            run(&tool, &args);
        }
    } else {
        for (schema, key, value) in [
            ("org.gnome.system.proxy.http", "host", "'corp.example'"),
            ("org.gnome.system.proxy.http", "port", "3128"),
            ("org.gnome.system.proxy.https", "host", "'corp.example'"),
            ("org.gnome.system.proxy.https", "port", "3128"),
            (
                "org.gnome.system.proxy",
                "ignore-hosts",
                "['intranet.example']",
            ),
            ("org.gnome.system.proxy", "mode", "'manual'"),
        ] {
            run("gsettings", &["set", schema, key, value]);
        }
    }
    before
}
