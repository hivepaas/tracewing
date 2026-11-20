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
    for tool in [format!("k{verb}config6"), format!("k{verb}config5")] {
        if Command::new("which")
            .arg(&tool)
            .output()
            .is_ok_and(|o| o.status.success())
        {
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

/// A proxy of the user's own, so that "as it was" is more than the defaults.
pub fn the_users_own() {
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
}
