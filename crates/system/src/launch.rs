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
/// Hosts that go straight out, as the system proxy leaves them too.
const NO_PROXY: &str = "localhost,127.0.0.1,::1";

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
    let bundle = data_dir.join(TERMINAL_BUNDLE);
    write_bundle(ca_pem, &bundle)?;
    let banner = format!("This terminal goes through tracewing at {addr}.");
    os::open_terminal(&terminal_env(addr, ca_pem, &bundle), &banner, data_dir)
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
    // Node reads HTTP_PROXY only when asked to (Node 24 and later).
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
/// (where Firefox can read them, which is not Linux), and no first-run pages.
pub fn firefox_prefs(addr: SocketAddr) -> String {
    let (host, port) = (addr.ip(), addr.port());
    let prefs: [(&str, String); 9] = [
        ("network.proxy.type", "1".to_owned()),
        ("network.proxy.http", format!("\"{host}\"")),
        ("network.proxy.http_port", port.to_string()),
        ("network.proxy.ssl", format!("\"{host}\"")),
        ("network.proxy.ssl_port", port.to_string()),
        ("security.enterprise_roots.enabled", "true".to_owned()),
        ("browser.shell.checkDefaultBrowser", "false".to_owned()),
        ("browser.aboutwelcome.enabled", "false".to_owned()),
        (
            "browser.startup.homepage_override.mstone",
            "\"ignore\"".to_owned(),
        ),
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
        assert_eq!(get("NO_PROXY"), NO_PROXY);
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
