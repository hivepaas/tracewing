//! Linux has no one system proxy and no one trust store.
//!
//! - The proxy: GNOME and its relatives keep it in `gsettings`, KDE in
//!   `kioslaverc`. Other desktops have nothing to set.
//! - Trust: the distribution's store (`Scope::Machine`) serves curl, Python and
//!   most command-line programs. Chrome and Firefox do not read it: they read
//!   NSS databases, one per browser and profile (`Scope::User`).
//!
//! Recovery at the next login is an XDG autostart entry.

use anyhow::{Context, Result, bail};
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
};

use super::{
    Place, Saved, Scope,
    launch::{self, Browser},
};

const NICKNAME: &str = "tracewing CA";
const ANCHOR_NAME: &str = "tracewing-ca";
const AUTOSTART_FILE: &str = "tracewing-proxy-recovery.desktop";
const GNOME_BYPASS: &str = "['localhost', '127.0.0.0/8', '::1']";
const KDE_BYPASS: &str = "localhost,127.0.0.1,::1";

fn run(program: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("run {program}"))?;
    if !out.status.success() {
        bail!(
            "{program} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

fn on_path(program: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {program}")])
        .output()
        .is_ok_and(|out| out.status.success())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Desktop {
    Gnome,
    Kde(&'static str),
}

fn desktop() -> Result<Desktop> {
    let names = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let is = |wanted: &[&str]| {
        names
            .split(':')
            .any(|name| wanted.iter().any(|w| name.eq_ignore_ascii_case(w)))
    };
    if is(&["KDE"]) {
        for tool in ["kwriteconfig6", "kwriteconfig5"] {
            if on_path(tool) {
                return Ok(Desktop::Kde(tool));
            }
        }
        bail!("KDE without kwriteconfig");
    }
    if is(&[
        "GNOME", "Unity", "Cinnamon", "Budgie", "Pantheon", "MATE", "XFCE",
    ]) && on_path("gsettings")
    {
        return Ok(Desktop::Gnome);
    }
    bail!(
        "no system proxy setting on this desktop ({})",
        if names.is_empty() { "unknown" } else { &names }
    )
}

const GNOME_KEYS: &[(&str, &str, &str)] = &[
    ("mode", "org.gnome.system.proxy", "mode"),
    ("ignore", "org.gnome.system.proxy", "ignore-hosts"),
    ("autoconfig", "org.gnome.system.proxy", "autoconfig-url"),
    ("http.host", "org.gnome.system.proxy.http", "host"),
    ("http.port", "org.gnome.system.proxy.http", "port"),
    ("https.host", "org.gnome.system.proxy.https", "host"),
    ("https.port", "org.gnome.system.proxy.https", "port"),
];
const KDE_KEYS: &[&str] = &[
    "ProxyType",
    "httpProxy",
    "httpsProxy",
    "NoProxyFor",
    "Proxy Config Script",
];

fn kde_read_tool(write_tool: &str) -> String {
    write_tool.replace("write", "read")
}

fn kde_get(write_tool: &str, key: &str) -> Result<String> {
    run(
        &kde_read_tool(write_tool),
        &[
            "--file",
            "kioslaverc",
            "--group",
            "Proxy Settings",
            "--key",
            key,
        ],
    )
}

fn kde_set(write_tool: &str, key: &str, value: &str) -> Result<()> {
    run(
        write_tool,
        &[
            "--file",
            "kioslaverc",
            "--group",
            "Proxy Settings",
            "--key",
            key,
            value,
        ],
    )?;
    Ok(())
}

/// Tells running KDE programs to read the settings again. Nothing is listening
/// outside a session, which is not an error.
fn kde_notify() {
    let _ = run(
        "dbus-send",
        &[
            "--type=signal",
            "/KIO/Scheduler",
            "org.kde.KIO.Scheduler.reparseSlaveConfiguration",
            "string:",
        ],
    );
}

pub fn read_proxy() -> Result<Saved> {
    let mut saved = Saved::default();
    match desktop()? {
        Desktop::Gnome => {
            for (name, schema, key) in GNOME_KEYS {
                saved.set(name, run("gsettings", &["get", schema, key])?);
            }
        }
        Desktop::Kde(tool) => {
            for key in KDE_KEYS {
                saved.set(key, kde_get(tool, key)?);
            }
        }
    }
    Ok(saved)
}

pub fn set_proxy(addr: SocketAddr) -> Result<()> {
    let (host, port) = (addr.ip().to_string(), addr.port().to_string());
    match desktop()? {
        Desktop::Gnome => {
            let quoted = format!("'{host}'");
            for (schema, key, value) in [
                ("org.gnome.system.proxy.http", "host", quoted.as_str()),
                ("org.gnome.system.proxy.http", "port", port.as_str()),
                ("org.gnome.system.proxy.https", "host", quoted.as_str()),
                ("org.gnome.system.proxy.https", "port", port.as_str()),
                ("org.gnome.system.proxy", "ignore-hosts", GNOME_BYPASS),
                // Last, so that nothing uses the proxy half set.
                ("org.gnome.system.proxy", "mode", "'manual'"),
            ] {
                run("gsettings", &["set", schema, key, value])?;
            }
        }
        Desktop::Kde(tool) => {
            let url = format!("http://{host} {port}");
            kde_set(tool, "httpProxy", &url)?;
            kde_set(tool, "httpsProxy", &url)?;
            kde_set(tool, "NoProxyFor", KDE_BYPASS)?;
            kde_set(tool, "ProxyType", "1")?;
            kde_notify();
        }
    }
    Ok(())
}

pub fn proxy_is(addr: SocketAddr) -> Result<bool> {
    let (host, port) = (addr.ip().to_string(), addr.port().to_string());
    Ok(match desktop()? {
        Desktop::Gnome => {
            let get = |schema, key| run("gsettings", &["get", schema, key]);
            get("org.gnome.system.proxy", "mode")? == "'manual'"
                && get("org.gnome.system.proxy.https", "host")? == format!("'{host}'")
                && get("org.gnome.system.proxy.https", "port")? == port
        }
        Desktop::Kde(tool) => {
            kde_get(tool, "ProxyType")? == "1"
                && kde_get(tool, "httpsProxy")? == format!("http://{host} {port}")
        }
    })
}

pub fn restore_proxy(saved: &Saved, _addr: SocketAddr) -> Result<()> {
    match desktop()? {
        Desktop::Gnome => {
            // The mode first, so that nothing uses the proxy half restored.
            for (name, schema, key) in GNOME_KEYS {
                run("gsettings", &["set", schema, key, saved.get(name)])?;
            }
        }
        Desktop::Kde(tool) => {
            for key in KDE_KEYS {
                kde_set(tool, key, saved.get(key))?;
            }
            kde_notify();
        }
    }
    Ok(())
}

fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
}

fn autostart_path() -> Result<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .map_or_else(|| Ok::<_, anyhow::Error>(home()?.join(".config")), Ok)?;
    Ok(config.join("autostart").join(AUTOSTART_FILE))
}

/// Quotes one argument of an `Exec=` line, by the Desktop Entry rules.
fn exec_quote(part: &str) -> String {
    let inner = part
        .replace('\\', r"\\\\")
        .replace('"', "\\\\\"")
        .replace('`', r"\\`")
        .replace('$', r"\\$")
        .replace('%', "%%");
    format!("\"{inner}\"")
}

pub fn arm_recovery(command: &[String]) -> Result<()> {
    let exec = command
        .iter()
        .map(|part| exec_quote(part))
        .collect::<Vec<_>>()
        .join(" ");
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=tracewing proxy recovery\nExec={exec}\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
    );
    let path = autostart_path()?;
    fs::create_dir_all(path.parent().context("no parent")?)?;
    fs::write(&path, entry).with_context(|| format!("write {}", path.display()))
}

pub fn disarm_recovery() -> Result<()> {
    match fs::remove_file(autostart_path()?) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// The distribution's anchor directory and the command that applies it.
fn system_store() -> Result<(&'static str, &'static [&'static str])> {
    const STORES: &[(&str, &str, &[&str])] = &[
        // Debian, Ubuntu.
        (
            "/usr/local/share/ca-certificates",
            "update-ca-certificates",
            &["update-ca-certificates"],
        ),
        // Fedora, RHEL and their relatives.
        (
            "/etc/pki/ca-trust/source/anchors",
            "update-ca-trust",
            &["update-ca-trust", "extract"],
        ),
        // openSUSE.
        (
            "/etc/pki/trust/anchors",
            "update-ca-certificates",
            &["update-ca-certificates"],
        ),
        // Arch.
        (
            "/etc/ca-certificates/trust-source/anchors",
            "trust",
            &["trust", "extract-compat"],
        ),
    ];
    STORES
        .iter()
        .find(|(dir, tool, _)| Path::new(dir).is_dir() && on_path(tool))
        .map(|(dir, _, apply)| (*dir, *apply))
        .context("no known system certificate store on this distribution")
}

fn anchor_path(dir: &str) -> PathBuf {
    Path::new(dir).join(format!("{ANCHOR_NAME}.crt"))
}

/// Runs a shell script as root: directly if we are, else through polkit's dialog.
fn run_as_admin(script: &str) -> Result<()> {
    if run("id", &["-u"]).is_ok_and(|id| id == "0") {
        run("sh", &["-c", script])?;
    } else {
        run("pkexec", &["sh", "-c", script]).context("ask for administrator rights")?;
    }
    Ok(())
}

fn quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

/// Every NSS database of this user that a browser would read. Chromium shares
/// `~/.pki/nssdb` unless it runs in a sandbox with its own home.
fn nss_databases(create_shared: bool) -> Result<Vec<PathBuf>> {
    let home = home()?;
    let mut found = Vec::new();

    let shared = home.join(".pki/nssdb");
    if create_shared && !shared.join("cert9.db").exists() {
        fs::create_dir_all(&shared)?;
        run(
            "certutil",
            &[
                "-N",
                "--empty-password",
                "-d",
                &format!("sql:{}", shared.display()),
            ],
        )?;
    }
    for sandboxed in [
        home.join(".pki/nssdb"),
        home.join("snap/chromium/current/.pki/nssdb"),
        home.join(".var/app/org.chromium.Chromium/.pki/nssdb"),
        home.join(".var/app/com.google.Chrome/.pki/nssdb"),
    ] {
        if sandboxed.join("cert9.db").exists() {
            found.push(sandboxed);
        }
    }

    for firefox in [
        home.join(".mozilla/firefox"),
        home.join("snap/firefox/common/.mozilla/firefox"),
        home.join(".var/app/org.mozilla.firefox/.mozilla/firefox"),
    ] {
        let Ok(profiles) = fs::read_dir(&firefox) else {
            continue;
        };
        for profile in profiles.flatten() {
            if profile.path().join("cert9.db").exists() {
                found.push(profile.path());
            }
        }
    }
    Ok(found)
}

/// Does `job` to every database, then fails if any failed: one broken profile
/// must not keep the others, Chrome's among them, from being done.
fn each_database(dbs: Vec<PathBuf>, mut job: impl FnMut(&Path) -> Result<()>) -> Result<()> {
    let failed: Vec<String> = dbs
        .iter()
        .filter_map(|db| job(db).err().map(|e| format!("{e:#}")))
        .collect();
    if failed.is_empty() {
        Ok(())
    } else {
        bail!("{}", failed.join("; "))
    }
}

fn nss_holds(db: &Path) -> bool {
    run(
        "certutil",
        &["-L", "-d", &format!("sql:{}", db.display()), "-n", NICKNAME],
    )
    .is_ok()
}

pub fn ca_locations(_cert_pem: &Path, der: &[u8]) -> Result<Vec<Place>> {
    let mut places = Vec::new();
    // The file there may hold an older CA, from before this one was made.
    if let Ok((dir, _)) = system_store()
        && crate::ca_der(&anchor_path(dir)).is_ok_and(|anchor| anchor.as_ref() == der)
    {
        places.push(Place {
            name: format!("system store ({dir})"),
            browsers: false,
        });
    }
    if on_path("certutil") {
        for db in nss_databases(false)? {
            if nss_holds(&db) {
                places.push(Place {
                    name: format!("NSS: {}", db.display()),
                    browsers: true,
                });
            }
        }
    }
    Ok(places)
}

pub fn ca_install(cert_pem: &Path, _der: &[u8], scope: Scope) -> Result<()> {
    match scope {
        Scope::Machine => {
            let (dir, apply) = system_store()?;
            run_as_admin(&format!(
                "install -m 0644 {} {} && {}",
                quote(cert_pem),
                quote(&anchor_path(dir)),
                apply.join(" ")
            ))
        }
        Scope::User => {
            if !on_path("certutil") {
                bail!("browsers need certutil (libnss3-tools or nss-tools) to trust the CA");
            }
            let pem = cert_pem.display().to_string();
            each_database(nss_databases(true)?, |db| {
                run(
                    "certutil",
                    &[
                        "-A",
                        "-d",
                        &format!("sql:{}", db.display()),
                        "-t",
                        "C,,",
                        "-n",
                        NICKNAME,
                        "-i",
                        &pem,
                    ],
                )
                .map(drop)
            })
        }
    }
}

pub fn ca_uninstall(_cert_pem: &Path, _der: &[u8], scope: Scope) -> Result<()> {
    match scope {
        Scope::Machine => {
            let (dir, apply) = system_store()?;
            run_as_admin(&format!(
                "rm -f {} && {}",
                quote(&anchor_path(dir)),
                apply.join(" ")
            ))
        }
        Scope::User => {
            let holding = nss_databases(false)?.into_iter().filter(|db| nss_holds(db));
            each_database(holding.collect(), |db| {
                run(
                    "certutil",
                    &["-D", "-d", &format!("sql:{}", db.display()), "-n", NICKNAME],
                )
                .map(drop)
            })
        }
    }
}

/// Terminals, and what comes before the command each is to run.
const TERMINALS: &[(&str, &[&str])] = &[
    ("ptyxis", &["--new-window", "--"]),
    ("gnome-terminal", &["--"]),
    ("kgx", &["--"]),
    ("konsole", &["-e"]),
    ("xfce4-terminal", &["-x"]),
    ("alacritty", &["-e"]),
    ("kitty", &[]),
    ("wezterm", &["start", "--"]),
    ("foot", &[]),
    ("x-terminal-emulator", &["-e"]),
    ("xterm", &["-e"]),
];

/// The user's `$TERMINAL` first, then the known ones.
fn find_terminal() -> Option<(PathBuf, &'static [&'static str])> {
    let flags = |name: &str| {
        TERMINALS
            .iter()
            .find(|(known, _)| *known == name)
            .map_or(&["-e"][..], |(_, flags)| *flags)
    };
    if let Some(chosen) = std::env::var_os("TERMINAL").filter(|t| !t.is_empty()) {
        let chosen = PathBuf::from(chosen);
        let path = if chosen.is_absolute() {
            Some(chosen.clone())
        } else {
            launch::on_path(&chosen.to_string_lossy())
        };
        if let Some(path) = path.filter(|p| p.is_file()) {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            return Some((path.clone(), flags(&name)));
        }
    }
    TERMINALS
        .iter()
        .find_map(|(name, flags)| launch::on_path(name).map(|path| (path, *flags)))
}

/// What a terminal is told to run: `env` sets the variables, whatever process
/// the terminal runs it in, then the user's shell starts under a line saying so.
fn terminal_command(
    flags: &[&str],
    env: &[(&str, String)],
    banner: &str,
    shell: &str,
) -> Vec<String> {
    let mut args: Vec<String> = flags.iter().map(|f| (*f).to_owned()).collect();
    args.push("env".to_owned());
    args.extend(env.iter().map(|(name, value)| format!("{name}={value}")));
    args.extend([
        "sh".to_owned(),
        "-c".to_owned(),
        r#"printf '%s\n\n' "$1"; exec "$0""#.to_owned(),
        shell.to_owned(),
        banner.to_owned(),
    ]);
    args
}

pub fn open_terminal(env: &[(&str, String)], banner: &str, _data_dir: &Path) -> Result<()> {
    let (program, flags) = find_terminal().context("no terminal found: set $TERMINAL to one")?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
    let mut command = Command::new(&program);
    command.args(terminal_command(flags, env, banner, &shell));
    if let Ok(home) = home() {
        command.current_dir(home);
    }
    launch::spawn(command).with_context(|| format!("open {}", program.display()))
}

const BROWSERS: &[(&str, &str)] = &[
    ("google-chrome", "Chrome"),
    ("google-chrome-stable", "Chrome"),
    ("chromium", "Chromium"),
    ("chromium-browser", "Chromium"),
    ("microsoft-edge", "Edge"),
    ("microsoft-edge-stable", "Edge"),
    ("brave-browser", "Brave"),
];

pub fn find_browser(profiles: &Path) -> Option<Browser> {
    BROWSERS.iter().find_map(|(program, name)| {
        let path = launch::on_path(program)?;
        // A snap may write only in its own folder of the home. Ubuntu's
        // chromium-browser is a script that starts the chromium snap.
        let snap = (path.starts_with("/snap/")
            || program.starts_with("chromium") && Path::new("/snap/bin/chromium").exists())
        .then(|| {
            if program.starts_with("chromium") {
                "chromium"
            } else {
                program
            }
        });
        let profile = match snap {
            Some(snap) => home()
                .ok()?
                .join("snap")
                .join(snap)
                .join("common/tracewing-profile"),
            None => profiles.join(name.to_lowercase()),
        };
        Some(Browser {
            name,
            program: path,
            profile,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_terminal_runs_env_then_the_shell() {
        let args = terminal_command(
            &["--new-window", "--"],
            &[("HTTPS_PROXY", "http://127.0.0.1:9090".to_owned())],
            "hello",
            "/bin/bash",
        );
        assert_eq!(
            args,
            [
                "--new-window",
                "--",
                "env",
                "HTTPS_PROXY=http://127.0.0.1:9090",
                "sh",
                "-c",
                r#"printf '%s\n\n' "$1"; exec "$0""#,
                "/bin/bash",
                "hello",
            ]
        );
    }

    #[test]
    fn a_failing_database_does_not_stop_the_rest() {
        let dbs: Vec<PathBuf> = ["a", "broken", "c"].iter().map(PathBuf::from).collect();
        let mut done = Vec::new();
        let result = each_database(dbs, |db| {
            if db == Path::new("broken") {
                bail!("bad database: {}", db.display());
            }
            done.push(db.to_owned());
            Ok(())
        });
        assert_eq!(done, [PathBuf::from("a"), PathBuf::from("c")]);
        assert_eq!(result.unwrap_err().to_string(), "bad database: broken");
    }

    #[test]
    fn exec_lines_quote_what_the_spec_reserves() {
        assert_eq!(exec_quote("/a b/c"), "\"/a b/c\"");
        assert_eq!(exec_quote("100%"), "\"100%%\"");
        assert_eq!(exec_quote("a$b"), "\"a\\\\$b\"");
    }
}
