//! One app at a time through the proxy, with nothing else on the machine
//! touched: a terminal whose environment points at it, and a browser with a
//! profile of its own. The system proxy and the trust stores stay as they are.

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::os;

/// The roots a terminal opened from here trusts, kept in the data directory.
const TERMINAL_BUNDLE: &str = "terminal-roots.pem";
/// Hosts that go straight out: none. A server on this computer is often
/// what is being debugged, so its traffic goes through the proxy as well.
/// The system proxy still leaves it out, as that is every app's.
const NO_PROXY: &str = "";

/// What a browser is built on, which decides how it is opened and where it
/// keeps what it trusts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Chromium,
    Firefox,
    Safari,
}

/// A browser found on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Browser {
    pub name: &'static str,
    pub engine: Engine,
    pub program: PathBuf,
    /// Its profile for tracewing, apart from the user's own.
    pub profile: PathBuf,
    /// Linux only: the NSS database a Chromium reads, or the folders a
    /// Firefox keeps its profiles in. Empty where browsers read the system's
    /// store.
    pub nss: Vec<PathBuf>,
}

/// Opens a terminal whose programs go through the proxy and trust its CA.
pub fn open_terminal(addr: SocketAddr, ca_pem: &Path, data_dir: &Path) -> Result<()> {
    let banner = format!("This terminal goes through tracewing at {addr}.");
    os::open_terminal(&terminal_vars(addr, ca_pem, data_dir)?, &banner, data_dir)
}

/// What a terminal's programs read to go through the proxy, its bundle of
/// roots written first: what Open a terminal sets, and what `tracewing env`
/// says for a terminal of the user's own.
pub fn terminal_vars(
    addr: SocketAddr,
    ca_pem: &Path,
    data_dir: &Path,
) -> Result<Vec<(&'static str, String)>> {
    let bundle = data_dir.join(TERMINAL_BUNDLE);
    write_bundle(ca_pem, &bundle)?;
    Ok(terminal_env(addr, ca_pem, &bundle))
}

/// A shell, for the lines that set variables in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    /// sh, bash and zsh.
    Sh,
    Fish,
    PowerShell,
    /// Windows' `cmd`.
    Cmd,
}

impl Shell {
    /// What a terminal on this OS runs, most often.
    pub fn native() -> Self {
        if cfg!(windows) {
            Self::PowerShell
        } else {
            Self::Sh
        }
    }

    /// A line for each variable, which this shell runs to set it, quoted so
    /// that a space or a quote in a path stays in it.
    pub fn set(self, vars: &[(&str, String)]) -> String {
        let mut out = String::new();
        for (name, value) in vars {
            let line = match self {
                Self::Sh => format!("export {name}='{}'", value.replace('\'', r"'\''")),
                Self::Fish => format!(
                    "set -gx {name} '{}'",
                    value.replace('\\', r"\\").replace('\'', r"\'")
                ),
                // An empty value takes the variable away, which for NO_PROXY
                // is the same: nothing goes around the proxy.
                Self::PowerShell => format!("$env:{name} = '{}'", value.replace('\'', "''")),
                Self::Cmd => format!("set \"{name}={value}\""),
            };
            out.push_str(&line);
            out.push('\n');
        }
        out
    }
}

/// Whether Node.js of this version (`v22.17.1`, `22.17.1`) sends its requests
/// through `HTTP_PROXY` and `HTTPS_PROXY` when `NODE_USE_ENV_PROXY` asks it
/// to: from 22.21 and 24.5 on. 24.0 to 24.4 do it for `fetch` alone, which
/// leaves out every library on `http` and `https`, so they count as not.
/// `None` for what is no version.
pub fn node_reads_proxy_env(version: &str) -> Option<bool> {
    let mut parts = version.trim().trim_start_matches('v').split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    Some(match major {
        22 => minor >= 21,
        24 => minor >= 5,
        major => major >= 25,
    })
}

/// The version of the Node.js a terminal would run, when that one sends its
/// requests straight out whatever `HTTPS_PROXY` says. Asked of the user's
/// own shell, started as a terminal starts it, since that is where nvm, fnm,
/// Volta and Homebrew put Node on the `PATH` and an app opened from the
/// desktop has none of them. On Windows, programs have the user's `PATH`.
pub fn node_past_the_proxy() -> Option<String> {
    #[cfg(unix)]
    let version = {
        let shell = std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
        node_version_in(Path::new(&shell), std::time::Duration::from_secs(5))
    };
    #[cfg(windows)]
    let version = node_version_here();
    version.filter(|v| node_reads_proxy_env(v) == Some(false))
}

/// The version of the Node.js on this process's `PATH`: the one a program
/// started from a terminal runs.
pub fn node_version_here() -> Option<String> {
    let mut command = Command::new("node");
    command.arg("--version");
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0800_0000);
    let out = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    version_in(&String::from_utf8_lossy(&out.stdout))
}

/// `node --version` as `shell` answers it, logged in and interactive as a
/// terminal's is, so that its start-up files run. Given up after `wait`,
/// with whatever it started, as such files can wait on a terminal it has not.
#[cfg(unix)]
fn node_version_in(shell: &Path, wait: std::time::Duration) -> Option<String> {
    use std::io::Read;
    let mut command = Command::new(shell);
    command.args(["-l", "-i", "-c", "node --version"]);
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = std::time::Instant::now() + wait;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20))
            }
            _ => {
                // The group: the shell and anything its files started.
                let pid = child.id() as libc::pid_t;
                // SAFETY: a signal to a process group of our own child's.
                unsafe { libc::kill(-pid, libc::SIGKILL) };
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    version_in(&out)
}

/// The last line that is a version, after whatever start-up files print.
fn version_in(out: &str) -> Option<String> {
    out.lines()
        .rev()
        .map(str::trim)
        .find(|line| node_reads_proxy_env(line).is_some())
        .map(|line| line.trim_start_matches('v').to_owned())
}

/// What a terminal's programs read. Most take a single file of every root
/// they trust, so the bundle holds the system's roots too: hosts tracewing does
/// not decrypt still show their own certificates. Node and Deno add a file to
/// what they trust already, so they get the CA alone.
pub fn terminal_env(addr: SocketAddr, ca_pem: &Path, bundle: &Path) -> Vec<(&'static str, String)> {
    let proxy = format!("http://{addr}");
    let bundle = bundle.display().to_string();
    let ca = ca_pem.display().to_string();
    let mut env = Vec::new();
    for name in ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"] {
        env.push((name, proxy.clone()));
    }
    for name in ["NO_PROXY", "no_proxy"] {
        env.push((name, NO_PROXY.to_owned()));
    }
    for name in [
        "SSL_CERT_FILE",
        "CURL_CA_BUNDLE",
        "REQUESTS_CA_BUNDLE",
        "GIT_SSL_CAINFO",
        "AWS_CA_BUNDLE",
    ] {
        env.push((name, bundle.clone()));
    }
    env.push(("NODE_EXTRA_CA_CERTS", ca.clone()));
    // Node reads HTTP_PROXY only when asked to, and only from 22.21 and
    // 24.5 on: see `node_reads_proxy_env`.
    env.push(("NODE_USE_ENV_PROXY", "1".to_owned()));
    env.push(("DENO_CERT", ca));
    env
}

/// Writes the system's roots and the CA into one PEM file.
fn write_bundle(ca_pem: &Path, bundle: &Path) -> Result<()> {
    let mut pem =
        fs::read_to_string(ca_pem).with_context(|| format!("read {}", ca_pem.display()))?;
    if !pem.ends_with('\n') {
        pem.push('\n');
    }
    // Some roots may not load; the rest still serve.
    for cert in rustls_native_certs::load_native_certs().certs {
        pem.push_str(&pem_block(&cert));
    }
    fs::write(bundle, pem).with_context(|| format!("write {}", bundle.display()))
}

fn pem_block(der: &[u8]) -> String {
    let body = BASE64.encode(der);
    let mut out = String::from("-----BEGIN CERTIFICATE-----\n");
    for line in body.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(line).expect("base64 is ASCII"));
        out.push('\n');
    }
    out.push_str("-----END CERTIFICATE-----\n");
    out
}

/// The browsers found on this machine, one of each name, in the order the
/// OS's list has them: Chromium's family first.
pub fn find_browsers(data_dir: &Path) -> Vec<Browser> {
    os::find_browsers(&data_dir.join("browser"))
}

/// Opens `url` in `browser`, in the user's own profile, as if they had.
pub fn open_url(browser: &Browser, url: &str) -> Result<()> {
    // A Mac app already running takes the URL from `open`, not from a second
    // copy of its program.
    #[cfg(target_os = "macos")]
    if let Some(app) = browser
        .program
        .ancestors()
        .find(|dir| dir.extension().is_some_and(|ext| ext == "app"))
    {
        let mut command = Command::new("/usr/bin/open");
        command.arg("-a").arg(app).arg(url);
        return spawn(command).with_context(|| format!("open {}", app.display()));
    }
    let mut command = Command::new(&browser.program);
    command.arg(url);
    spawn(command).with_context(|| format!("open {}", browser.program.display()))
}

/// Opens `browser` at `url` in its profile for tracewing, going through the
/// proxy. A Chromium gets `spki`, the leaves' key hash, for when it does not
/// trust the CA: it then takes them anyway, in this profile only. A Firefox
/// profile is told to trust `ca_pem`, as Firefox has no such flag.
pub fn open_browser(
    browser: &Browser,
    addr: SocketAddr,
    spki: Option<&str>,
    ca_pem: &Path,
    url: &str,
) -> Result<()> {
    let profile = &browser.profile;
    fs::create_dir_all(profile).with_context(|| format!("create {}", profile.display()))?;
    let mut command = Command::new(&browser.program);
    match browser.engine {
        Engine::Chromium => {
            command.args(browser_args(profile, addr, spki, url));
        }
        Engine::Firefox => {
            let prefs = profile.join("user.js");
            fs::write(&prefs, firefox_prefs(addr))
                .with_context(|| format!("write {}", prefs.display()))?;
            os::trust_in_profile(profile, ca_pem)?;
            command.args(firefox_args(profile, url));
        }
        Engine::Safari => bail!("Safari has no profile of its own to open"),
    }
    spawn(command).with_context(|| format!("open {}", browser.program.display()))
}

/// What a Firefox profile for tracewing holds: the proxy, the system's roots
/// (where Firefox can read them, which is not Linux), no first-run pages, and
/// as little of Firefox's own traffic as can be had, so that what is listed is
/// what the user does. In its first 12 s, a new profile went from 108 flows
/// to 34 with these, none of them mozilla.org's welcome. Remote settings
/// stay: Firefox learns of revoked certificates from them.
pub fn firefox_prefs(addr: SocketAddr) -> String {
    let (host, port) = (addr.ip(), addr.port());
    let text = |value: &str| format!("\"{value}\"");
    let prefs: Vec<(&str, String)> = vec![
        ("network.proxy.type", "1".to_owned()),
        ("network.proxy.http", text(&host.to_string())),
        ("network.proxy.http_port", port.to_string()),
        ("network.proxy.ssl", text(&host.to_string())),
        ("network.proxy.ssl_port", port.to_string()),
        // Servers on this computer too, which Firefox otherwise sends straight.
        ("network.proxy.allow_hijacking_localhost", "true".to_owned()),
        ("network.proxy.no_proxies_on", text("")),
        ("security.enterprise_roots.enabled", "true".to_owned()),
        // No first run: no welcome, no privacy notice, no default browser ask.
        ("browser.shell.checkDefaultBrowser", "false".to_owned()),
        ("browser.aboutwelcome.enabled", "false".to_owned()),
        ("browser.startup.homepage_override.mstone", text("ignore")),
        ("browser.startup.firstrunSkipsHomepage", "true".to_owned()),
        ("startup.homepage_welcome_url", text("")),
        ("startup.homepage_welcome_url.additional", text("")),
        ("datareporting.policy.firstRunURL", text("")),
        (
            "datareporting.policy.dataSubmissionPolicyBypassNotification",
            "true".to_owned(),
        ),
        (
            "toolkit.telemetry.reportingpolicy.firstRun",
            "false".to_owned(),
        ),
        // Less of Firefox's own traffic.
        (
            "datareporting.healthreport.uploadEnabled",
            "false".to_owned(),
        ),
        (
            "datareporting.policy.dataSubmissionEnabled",
            "false".to_owned(),
        ),
        ("toolkit.telemetry.enabled", "false".to_owned()),
        ("toolkit.telemetry.unified", "false".to_owned()),
        ("browser.newtabpage.enabled", "false".to_owned()),
        (
            "browser.newtabpage.activity-stream.feeds.topsites",
            "false".to_owned(),
        ),
        (
            "browser.newtabpage.activity-stream.feeds.section.topstories",
            "false".to_owned(),
        ),
        (
            "browser.newtabpage.activity-stream.showSponsored",
            "false".to_owned(),
        ),
        (
            "browser.newtabpage.activity-stream.showSponsoredTopSites",
            "false".to_owned(),
        ),
        ("browser.urlbar.quicksuggest.enabled", "false".to_owned()),
        (
            "browser.urlbar.suggest.quicksuggest.sponsored",
            "false".to_owned(),
        ),
        ("app.normandy.enabled", "false".to_owned()),
        ("app.shield.optoutstudies.enabled", "false".to_owned()),
        ("extensions.pocket.enabled", "false".to_owned()),
        ("network.captive-portal-service.enabled", "false".to_owned()),
        ("network.connectivity-service.enabled", "false".to_owned()),
    ];
    prefs
        .iter()
        .map(|(name, value)| format!("user_pref(\"{name}\", {value});\n"))
        .collect()
}

/// A Firefox of its own, beside the one the user may have open.
pub fn firefox_args(profile: &Path, url: &str) -> Vec<String> {
    vec![
        "--profile".to_owned(),
        profile.display().to_string(),
        "--no-remote".to_owned(),
        "--new-instance".to_owned(),
        url.to_owned(),
    ]
}

pub fn browser_args(
    profile: &Path,
    addr: SocketAddr,
    spki: Option<&str>,
    url: &str,
) -> Vec<String> {
    let mut args = vec![
        format!("--user-data-dir={}", profile.display()),
        format!("--proxy-server=http://{addr}"),
        // Servers on this computer too, which Chromium otherwise sends straight.
        "--proxy-bypass-list=<-loopback>".to_owned(),
        "--no-first-run".to_owned(),
        "--no-default-browser-check".to_owned(),
    ];
    if let Some(spki) = spki {
        args.push(format!("--ignore-certificate-errors-spki-list={spki}"));
    }
    args.push(url.to_owned());
    args
}

/// Starts a program that outlives the call, and reaps it when it ends. On Unix
/// it gets a process group of its own, so a Ctrl-C meant for tracewing in the
/// terminal it was started from does not close it as well.
pub(crate) fn spawn(mut command: Command) -> Result<()> {
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// A program on the `PATH`, as a full path.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn on_path(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr() -> SocketAddr {
        "127.0.0.1:9090".parse().unwrap()
    }

    #[test]
    fn a_terminal_goes_through_the_proxy_and_trusts_both() {
        let env = terminal_env(addr(), Path::new("/d/ca.pem"), Path::new("/d/roots.pem"));
        let get = |name: &str| {
            env.iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.as_str())
                .unwrap()
        };
        assert_eq!(get("HTTPS_PROXY"), "http://127.0.0.1:9090");
        assert_eq!(get("http_proxy"), "http://127.0.0.1:9090");
        assert_eq!(get("SSL_CERT_FILE"), "/d/roots.pem");
        assert_eq!(get("NODE_EXTRA_CA_CERTS"), "/d/ca.pem");
        assert_eq!(get("NO_PROXY"), "", "localhost goes through too");
        assert_eq!(get("no_proxy"), "");
    }

    #[test]
    fn node_reads_the_proxy_from_22_21_and_24_5() {
        for (version, reads) in [
            ("v20.19.0", false),
            ("v22.17.1", false),
            ("v22.20.9", false),
            ("v22.21.0", true),
            ("22.22.1", true),
            ("v23.11.0", false),
            ("v24.4.1", false),
            ("v24.5.0", true),
            ("v25.0.0", true),
        ] {
            assert_eq!(node_reads_proxy_env(version), Some(reads), "{version}");
        }
        assert_eq!(node_reads_proxy_env("command not found: node"), None);
        assert_eq!(node_reads_proxy_env(""), None);
    }

    /// Paths with a space and a quote in them, as a home folder can have.
    fn awkward() -> Vec<(&'static str, String)> {
        vec![
            (
                "SSL_CERT_FILE",
                r"/Users/O'Neil/My Data/roots.pem".to_owned(),
            ),
            ("NO_PROXY", String::new()),
        ]
    }

    #[test]
    fn each_shell_gets_lines_of_its_own() {
        assert_eq!(
            Shell::Sh.set(&awkward()),
            "export SSL_CERT_FILE='/Users/O'\\''Neil/My Data/roots.pem'\nexport NO_PROXY=''\n"
        );
        assert_eq!(
            Shell::Fish.set(&awkward()),
            "set -gx SSL_CERT_FILE '/Users/O\\'Neil/My Data/roots.pem'\nset -gx NO_PROXY ''\n"
        );
        assert_eq!(
            Shell::PowerShell.set(&awkward()),
            "$env:SSL_CERT_FILE = '/Users/O''Neil/My Data/roots.pem'\n$env:NO_PROXY = ''\n"
        );
        assert_eq!(
            Shell::Cmd.set(&[("HTTPS_PROXY", "http://127.0.0.1:9090".to_owned())]),
            "set \"HTTPS_PROXY=http://127.0.0.1:9090\"\n"
        );
    }

    /// sh itself reads the path back as it was.
    #[cfg(unix)]
    #[test]
    fn sh_reads_the_lines_back_as_written() {
        let script = format!("{}printf %s \"$SSL_CERT_FILE\"", Shell::Sh.set(&awkward()));
        let out = Command::new("/bin/sh")
            .args(["-c", &script])
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            r"/Users/O'Neil/My Data/roots.pem"
        );
    }

    /// A shell that says things as it starts, and one that never finishes.
    #[cfg(unix)]
    #[test]
    fn node_is_asked_of_the_users_shell() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("tracewing-node-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let shell = |name: &str, body: &str| {
            let path = dir.join(name);
            fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            path
        };
        let wait = std::time::Duration::from_secs(5);
        let chatty = shell(
            "chatty",
            r#"[ "$1 $2 $3 $4" = "-l -i -c node --version" ] || exit 9
echo 'Welcome back!'
echo v22.17.1
echo"#,
        );
        assert_eq!(node_version_in(&chatty, wait).as_deref(), Some("22.17.1"));
        let none = shell("none", "echo 'zsh: command not found: node'; exit 127");
        assert_eq!(node_version_in(&none, wait), None);
        let stuck = shell("stuck", "sleep 60 & sleep 60; echo v22.17.1");
        let started = std::time::Instant::now();
        assert_eq!(
            node_version_in(&stuck, std::time::Duration::from_millis(300)),
            None
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_bundle_holds_the_ca_first_and_the_system_roots() {
        let dir = std::env::temp_dir().join(format!("tracewing-bundle-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let ca = dir.join("ca.pem");
        fs::write(&ca, pem_block(b"not a real certificate")).unwrap();
        let bundle = dir.join("roots.pem");
        write_bundle(&ca, &bundle).unwrap();
        let text = fs::read_to_string(&bundle).unwrap();
        assert!(text.starts_with(&fs::read_to_string(&ca).unwrap()));
        assert_eq!(
            text.matches("-----BEGIN CERTIFICATE-----").count(),
            1 + rustls_native_certs::load_native_certs().certs.len()
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn pem_lines_are_64_wide() {
        let block = pem_block(&[7u8; 100]);
        let lines: Vec<&str> = block.lines().collect();
        assert_eq!(lines[0], "-----BEGIN CERTIFICATE-----");
        assert_eq!(lines[1].len(), 64);
        assert_eq!(*lines.last().unwrap(), "-----END CERTIFICATE-----");
    }

    #[test]
    fn a_firefox_profile_goes_through_the_proxy() {
        let prefs = firefox_prefs(addr());
        assert!(prefs.contains("user_pref(\"network.proxy.type\", 1);\n"));
        assert!(prefs.contains("user_pref(\"network.proxy.ssl\", \"127.0.0.1\");\n"));
        assert!(prefs.contains("user_pref(\"network.proxy.ssl_port\", 9090);\n"));
        assert!(prefs.contains("user_pref(\"network.proxy.allow_hijacking_localhost\", true);\n"));
        assert!(prefs.contains("user_pref(\"network.proxy.no_proxies_on\", \"\");\n"));
        assert!(prefs.contains("user_pref(\"datareporting.policy.firstRunURL\", \"\");\n"));
        assert!(prefs.contains("user_pref(\"toolkit.telemetry.enabled\", false);\n"));
        assert_eq!(
            firefox_args(Path::new("/p"), "https://tracewing.test/"),
            [
                "--profile",
                "/p",
                "--no-remote",
                "--new-instance",
                "https://tracewing.test/"
            ]
        );
    }

    #[test]
    fn the_browser_gets_its_own_profile_and_the_proxy() {
        let args = browser_args(
            Path::new("/p"),
            addr(),
            Some("abc="),
            "https://tracewing.test/",
        );
        assert_eq!(
            args,
            [
                "--user-data-dir=/p",
                "--proxy-server=http://127.0.0.1:9090",
                "--proxy-bypass-list=<-loopback>",
                "--no-first-run",
                "--no-default-browser-check",
                "--ignore-certificate-errors-spki-list=abc=",
                "https://tracewing.test/",
            ]
        );
        assert!(
            !browser_args(Path::new("/p"), addr(), None, "u")
                .iter()
                .any(|a| a.contains("spki"))
        );
    }
}
