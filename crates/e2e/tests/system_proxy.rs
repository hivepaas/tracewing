//! S2, the system proxy: turned on, the desktop's own setting names
//! tracewing; turned off, it is what it was, a proxy of the user's own
//! included. Read with the desktop's own tools rather than tracewing's reader,
//! so the reader is checked too.
//!
//! Changes this computer's proxy setting. See `tracewing_e2e`.

#![cfg(target_os = "linux")]

use std::{net::SocketAddr, process::Command};
use tracewing_e2e::{free_port, only_where_allowed, scratch};

fn run(program: &str, args: &[&str]) -> String {
    let out = Command::new(program).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// The desktop the session says it is, as tracewing reads it.
fn kde() -> bool {
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

/// The whole setting, by key.
fn setting() -> Vec<(String, String)> {
    if kde() {
        let tool = kde_tool("read");
        KDE.iter()
            .map(|key| {
                let value = run(
                    &tool,
                    &[
                        "--file",
                        "kioslaverc",
                        "--group",
                        "Proxy Settings",
                        "--key",
                        key,
                    ],
                );
                (key.to_string(), value)
            })
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

fn get<'a>(setting: &'a [(String, String)], key: &str) -> &'a str {
    &setting.iter().find(|(k, _)| k == key).unwrap().1
}

/// A proxy of the user's own, so that "as it was" is more than the defaults.
fn the_users_own() {
    if kde() {
        let tool = kde_tool("write");
        for (key, value) in [
            ("ProxyType", "1"),
            ("httpProxy", "http://corp.example 3128"),
            ("httpsProxy", "http://corp.example 3128"),
            ("NoProxyFor", "intranet.example"),
        ] {
            run(
                &tool,
                &[
                    "--file",
                    "kioslaverc",
                    "--group",
                    "Proxy Settings",
                    "--key",
                    key,
                    value,
                ],
            );
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

#[test]
fn the_system_proxy_names_tracewing_and_comes_back_as_it_was() {
    only_where_allowed!();
    let dir = scratch("system-proxy");
    the_users_own();
    let before = setting();
    let addr = SocketAddr::from(([127, 0, 0, 1], free_port()));

    let taken = tracewing_system::take_over(&dir, addr, false).unwrap();
    let during = setting();
    if kde() {
        let url = format!("http://127.0.0.1 {}", addr.port());
        assert_eq!(get(&during, "ProxyType"), "1", "{during:?}");
        assert_eq!(get(&during, "httpProxy"), url, "{during:?}");
        assert_eq!(get(&during, "httpsProxy"), url, "{during:?}");
    } else {
        let port = addr.port().to_string();
        assert_eq!(
            get(&during, "org.gnome.system.proxy mode"),
            "'manual'",
            "{during:?}"
        );
        for scheme in ["http", "https"] {
            let schema = format!("org.gnome.system.proxy.{scheme}");
            assert_eq!(
                get(&during, &format!("{schema} host")),
                "'127.0.0.1'",
                "{during:?}"
            );
            assert_eq!(get(&during, &format!("{schema} port")), port, "{during:?}");
        }
    }
    taken.release().unwrap();
    assert_eq!(setting(), before, "not put back as it was");
    std::fs::remove_dir_all(&dir).ok();
}
