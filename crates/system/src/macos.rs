//! macOS: `networksetup` for the proxy of every network service, `security`
//! for the keychains, a LaunchAgent for recovery at the next login, and a
//! launchd agent of the session's for a tracewing that is killed.
//!
//! Changing trust settings asks for the user's password, in a system dialog,
//! whichever keychain the certificate goes in. There is no silent way, and
//! over SSH it fails outright.

use anyhow::{Context, Result, bail};
use std::{fs, net::SocketAddr, path::Path, path::PathBuf, process::Command};

use super::{
    BrowserTrust, Environment, Place, Saved, Scope,
    launch::{self, Browser, Engine},
};

const AGENT_LABEL: &str = "com.hivepaas.tracewing.proxy-recovery";
/// The waiter's label, before the part that names its data folder.
const WAITER_LABEL: &str = "com.hivepaas.tracewing.proxy-waiter";
/// Where the waiter's plist is written for the moment launchd takes it in.
const WAITER_PLIST: &str = "sysproxy.waiter.plist";
/// What the waiter says, kept beside the watchdog's log.
const WAITER_LOG: &str = "recovery.log";
const SYSTEM_KEYCHAIN: &str = "/Library/Keychains/System.keychain";
const CA_NAME: &str = "tracewing CA";
const BYPASS: &[&str] = &["localhost", "127.0.0.1", "::1", "*.local"];

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
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `networksetup` reports some failures on stdout and still exits with 0.
fn networksetup(args: &[&str]) -> Result<String> {
    let out = run("/usr/sbin/networksetup", args)?;
    if out.contains("requires admin privileges") || out.starts_with("** Error") {
        bail!("networksetup {}: {}", args.join(" "), out.trim());
    }
    Ok(out)
}

/// The first line says what `*` means; a `*` marks a disabled service.
fn services() -> Result<Vec<String>> {
    Ok(networksetup(&["-listallnetworkservices"])?
        .lines()
        .skip(1)
        .filter(|line| !line.starts_with('*') && !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn field<'a>(text: &'a str, name: &str) -> &'a str {
    text.lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix(": "))
        .unwrap_or("")
        .trim()
}

pub fn read_proxy() -> Result<Saved> {
    let mut saved = Saved::default();
    for (i, service) in services()?.iter().enumerate() {
        saved.set(&format!("{i}.service"), service.as_str());
        for (key, verb) in [("web", "-getwebproxy"), ("secure", "-getsecurewebproxy")] {
            let out = networksetup(&[verb, service])?;
            saved.set(&format!("{i}.{key}.enabled"), field(&out, "Enabled"));
            saved.set(&format!("{i}.{key}.server"), field(&out, "Server"));
            saved.set(&format!("{i}.{key}.port"), field(&out, "Port"));
        }
        let auto = networksetup(&["-getautoproxyurl", service])?;
        saved.set(&format!("{i}.auto.url"), field(&auto, "URL"));
        saved.set(&format!("{i}.auto.enabled"), field(&auto, "Enabled"));
        let discovery = networksetup(&["-getproxyautodiscovery", service])?;
        saved.set(
            &format!("{i}.discovery"),
            field(&discovery, "Auto Proxy Discovery"),
        );
        let bypass = networksetup(&["-getproxybypassdomains", service])?;
        // Without a list it prints a sentence, not a list.
        let bypass = if bypass.contains("aren't any") {
            String::new()
        } else {
            bypass.lines().collect::<Vec<_>>().join(" ")
        };
        saved.set(&format!("{i}.bypass"), bypass);
    }
    Ok(saved)
}

pub fn set_proxy(addr: SocketAddr) -> Result<()> {
    let (host, port) = (addr.ip().to_string(), addr.port().to_string());
    for service in services()? {
        networksetup(&["-setautoproxystate", &service, "off"])?;
        networksetup(&["-setproxyautodiscovery", &service, "off"])?;
        let mut bypass = vec!["-setproxybypassdomains", service.as_str()];
        bypass.extend_from_slice(BYPASS);
        networksetup(&bypass)?;
        networksetup(&["-setwebproxy", &service, &host, &port])?;
        networksetup(&["-setsecurewebproxy", &service, &host, &port])?;
    }
    Ok(())
}

pub fn proxy_is(addr: SocketAddr) -> Result<bool> {
    let (host, port) = (addr.ip().to_string(), addr.port().to_string());
    for service in services()? {
        let out = networksetup(&["-getsecurewebproxy", &service])?;
        if field(&out, "Enabled") == "Yes"
            && field(&out, "Server") == host
            && field(&out, "Port") == port
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn on_off(value: &str) -> &'static str {
    if value == "Yes" || value == "On" {
        "on"
    } else {
        "off"
    }
}

/// Goes through each service and each protocol on its own: the user may have
/// changed one of them since, and that choice stays.
pub fn restore_proxy(saved: &Saved, addr: SocketAddr) -> Result<()> {
    let (host, port) = (addr.ip().to_string(), addr.port().to_string());
    for i in 0.. {
        let service = saved.get(&format!("{i}.service"));
        if service.is_empty() {
            break;
        }
        let mut still_ours = false;
        for (key, get, set, state) in [
            ("web", "-getwebproxy", "-setwebproxy", "-setwebproxystate"),
            (
                "secure",
                "-getsecurewebproxy",
                "-setsecurewebproxy",
                "-setsecurewebproxystate",
            ),
        ] {
            let now = networksetup(&[get, service])?;
            if field(&now, "Enabled") != "Yes"
                || field(&now, "Server") != host
                || field(&now, "Port") != port
            {
                continue;
            }
            still_ours = true;
            // This also turns the protocol on, so the state comes after it.
            networksetup(&[
                set,
                service,
                saved.get(&format!("{i}.{key}.server")),
                saved.get(&format!("{i}.{key}.port")),
            ])?;
            let enabled = on_off(saved.get(&format!("{i}.{key}.enabled")));
            networksetup(&[state, service, enabled])?;
        }
        if !still_ours {
            continue;
        }
        let url = saved.get(&format!("{i}.auto.url"));
        if !url.is_empty() && url != "(null)" {
            networksetup(&["-setautoproxyurl", service, url])?;
        }
        let auto = on_off(saved.get(&format!("{i}.auto.enabled")));
        networksetup(&["-setautoproxystate", service, auto])?;
        let discovery = on_off(saved.get(&format!("{i}.discovery")));
        networksetup(&["-setproxyautodiscovery", service, discovery])?;

        let bypass = saved.get(&format!("{i}.bypass"));
        let mut args = vec!["-setproxybypassdomains", service];
        if bypass.is_empty() {
            args.push("Empty");
        } else {
            args.extend(bypass.split(' '));
        }
        networksetup(&args)?;
    }
    Ok(())
}

fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
}

fn agent_path() -> Result<PathBuf> {
    Ok(home()?
        .join("Library/LaunchAgents")
        .join(format!("{AGENT_LABEL}.plist")))
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn arm_recovery(command: &[String]) -> Result<()> {
    let args: String = command
        .iter()
        .map(|part| format!("    <string>{}</string>\n", escape_xml(part)))
        .collect();
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{AGENT_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
{args}  </array>
  <key>RunAtLoad</key>
  <true/>
</dict>
</plist>
"#
    );
    let path = agent_path()?;
    fs::create_dir_all(path.parent().context("no parent")?)?;
    fs::write(&path, plist).with_context(|| format!("write {}", path.display()))
}

pub fn disarm_recovery() -> Result<()> {
    match fs::remove_file(agent_path()?) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// The domain of this user's login session, which is where an agent runs
/// with the session's access to the network settings.
fn gui_domain() -> String {
    // SAFETY: getuid cannot fail.
    format!("gui/{}", unsafe { libc::getuid() })
}

/// Waits, from a process launchd keeps, for the app to go, and then puts the
/// settings back: the timer of Linux, without the minute.
///
/// launchd starts `command` at once and starts it again whenever it ends with
/// the journal still there — a `pkill tracewing` takes it with the app, and
/// launchd brings it straight back to find the app gone. Once the journal is
/// gone it is left ended. It belongs to this session alone: it is not in
/// `~/Library/LaunchAgents`, so it neither comes back at the next login (the
/// login agent is there for that) nor appears among the login items. The
/// plist is read once and removed, so nothing of it stays on disk.
pub fn arm_timer(command: &[String], journal: &Path) -> Result<()> {
    let dir = journal.parent().context("the journal has no folder")?;
    let path = dir.join(WAITER_PLIST);
    let plist = waiter_plist(
        &waiter_label(journal),
        command,
        journal,
        &dir.join(WAITER_LOG),
    );
    fs::write(&path, plist).with_context(|| format!("write {}", path.display()))?;
    // One that is still there from before is replaced, not added to.
    let _ = disarm_timer(journal);
    let loaded = run(
        "/bin/launchctl",
        &["bootstrap", &gui_domain(), &path.display().to_string()],
    );
    let _ = fs::remove_file(&path);
    loaded.map(|_| ())
}

/// One waiter for each data folder, as there is one journal for each: a
/// tracewing with a folder of its own — a test's, say — never takes another's
/// waiter away. The folder is named by a hash that does not change from one
/// build of tracewing to the next, so that the version that disarms it finds
/// what the version before it armed.
fn waiter_label(journal: &Path) -> String {
    let dir = journal.parent().unwrap_or(journal);
    let dir = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_owned());
    // FNV-1a, 64 bits.
    let hash = dir
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, &byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    format!("{WAITER_LABEL}.{hash:016x}")
}

fn waiter_plist(label: &str, command: &[String], journal: &Path, log: &Path) -> String {
    let args: String = command
        .iter()
        .map(|part| format!("    <string>{}</string>\n", escape_xml(part)))
        .collect();
    let journal = escape_xml(&journal.display().to_string());
    let log = escape_xml(&log.display().to_string());
    // KeepAlive while the journal is there: a waiter killed or failed comes
    // back, one that has put the settings back and removed the journal does
    // not. ThrottleInterval bounds the coming back: one that has waited
    // longer than it comes back at once, one that fails at once — the
    // settings cannot be read, say — comes back once a minute, as on Linux.
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
{args}  </array>
  <key>KeepAlive</key>
  <dict>
    <key>PathState</key>
    <dict>
      <key>{journal}</key>
      <true/>
    </dict>
  </dict>
  <key>ThrottleInterval</key>
  <integer>60</integer>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#
    )
}

pub fn disarm_timer(journal: &Path) -> Result<()> {
    let label = waiter_label(journal);
    // The waiter itself, having put the settings back: taking itself out of
    // launchd would be launchd ending it before the journal goes. It ends on
    // its own a moment later, and with no journal launchd leaves it ended: an
    // entry with no process, until the session ends or tracewing next starts
    // and finds nothing to recover, which takes it out.
    if std::env::var("XPC_SERVICE_NAME").is_ok_and(|name| name == label) {
        return Ok(());
    }
    let target = format!("{}/{label}", gui_domain());
    // Nothing was armed: say so without asking launchd to remove it.
    if Command::new("/bin/launchctl")
        .args(["print", &target])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| !status.success())
    {
        return Ok(());
    }
    run("/bin/launchctl", &["bootout", &target]).map(|_| ())
}

fn login_keychain() -> Result<String> {
    Ok(home()?
        .join("Library/Keychains/login.keychain-db")
        .display()
        .to_string())
}

fn holds(keychain: &str, cert_pem: &Path) -> Result<bool> {
    let wanted = fs::read_to_string(cert_pem)?;
    // Fails when nothing matches the name.
    let found = run(
        "/usr/bin/security",
        &["find-certificate", "-a", "-p", "-c", CA_NAME, keychain],
    )
    .unwrap_or_default();
    Ok(found.contains(wanted.trim()))
}

pub fn ca_locations(cert_pem: &Path, _der: &[u8]) -> Result<Vec<Place>> {
    let mut places = Vec::new();
    if holds(SYSTEM_KEYCHAIN, cert_pem)? {
        places.push(Place {
            name: "System keychain".to_owned(),
            browsers: true,
            scope: Scope::Machine,
        });
    }
    if holds(&login_keychain()?, cert_pem)? {
        places.push(Place {
            name: "login keychain".to_owned(),
            browsers: true,
            scope: Scope::User,
        });
    }
    Ok(places)
}

fn is_root() -> bool {
    run("/usr/bin/id", &["-u"]).is_ok_and(|id| id.trim() == "0")
}

/// Runs `security` as root: directly if we are, else through the system's
/// password dialog.
fn security_as_admin(args: &[&str]) -> Result<String> {
    if is_root() {
        return run("/usr/bin/security", args);
    }
    let line = std::iter::once("/usr/bin/security")
        .chain(args.iter().copied())
        .map(|part| format!("'{}'", part.replace('\'', r"'\''")))
        .collect::<Vec<_>>()
        .join(" ");
    let escaped = line.replace('\\', r"\\").replace('"', "\\\"");
    let script = format!("do shell script \"{escaped}\" with administrator privileges");
    run("/usr/bin/osascript", &["-e", &script])
}

pub fn ca_install(cert_pem: &Path, _der: &[u8], scope: Scope) -> Result<()> {
    let pem = cert_pem.display().to_string();
    match scope {
        Scope::User => {
            let keychain = login_keychain()?;
            run(
                "/usr/bin/security",
                &["add-trusted-cert", "-r", "trustRoot", "-k", &keychain, &pem],
            )?;
        }
        Scope::Machine => {
            security_as_admin(&[
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                SYSTEM_KEYCHAIN,
                &pem,
            ])?;
        }
    }
    Ok(())
}

pub fn ca_uninstall(cert_pem: &Path, _der: &[u8], scope: Scope) -> Result<()> {
    let pem = cert_pem.display().to_string();
    match scope {
        Scope::User => {
            let keychain = login_keychain()?;
            run("/usr/bin/security", &["remove-trusted-cert", &pem])?;
            run(
                "/usr/bin/security",
                &["delete-certificate", "-c", CA_NAME, &keychain],
            )?;
        }
        Scope::Machine => {
            security_as_admin(&["remove-trusted-cert", "-d", &pem])?;
            security_as_admin(&["delete-certificate", "-c", CA_NAME, SYSTEM_KEYCHAIN])?;
        }
    }
    Ok(())
}

/// Writes a script that sets the variables and starts the user's shell, and has
/// Terminal run it: Terminal opens `.command` files in a window of their own.
pub fn open_terminal(env: &[(&str, String)], banner: &str, data_dir: &Path) -> Result<()> {
    let script = data_dir.join("terminal.command");
    let quote = |text: &str| format!("'{}'", text.replace('\'', r"'\''"));
    let mut text = String::from("#!/bin/sh\n");
    for (name, value) in env {
        text.push_str(&format!("export {name}={}\n", quote(value)));
    }
    text.push_str(&format!("printf '%s\\n\\n' {}\n", quote(banner)));
    text.push_str("exec \"${SHELL:-/bin/zsh}\" -l\n");
    fs::write(&script, text).with_context(|| format!("write {}", script.display()))?;
    fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;
    let mut command = Command::new("/usr/bin/open");
    command.args(["-a", "Terminal"]).arg(&script);
    launch::spawn(command).context("open Terminal")
}

/// Apps, by bundle and program, and what they are.
const BROWSERS: &[(&str, &str, &str, Engine)] = &[
    ("Google Chrome", "Google Chrome", "Chrome", Engine::Chromium),
    ("Chromium", "Chromium", "Chromium", Engine::Chromium),
    ("Microsoft Edge", "Microsoft Edge", "Edge", Engine::Chromium),
    ("Brave Browser", "Brave Browser", "Brave", Engine::Chromium),
    ("Opera", "Opera", "Opera", Engine::Chromium),
    ("Firefox", "firefox", "Firefox", Engine::Firefox),
    ("Safari", "Safari", "Safari", Engine::Safari),
];

pub fn find_browsers(profiles: &Path) -> Vec<Browser> {
    let mut apps = vec![PathBuf::from("/Applications")];
    if let Ok(home) = home() {
        apps.push(home.join("Applications"));
    }
    BROWSERS
        .iter()
        .filter_map(|&(app, program, name, engine)| {
            apps.iter()
                .map(|dir| dir.join(format!("{app}.app/Contents/MacOS/{program}")))
                .find(|program| program.is_file())
                .map(|program| Browser {
                    name,
                    engine,
                    program,
                    profile: profiles.join(name.to_lowercase()),
                    nss: Vec::new(),
                })
        })
        .collect()
}

/// Safari and Chromium's family read the keychains, and so does Firefox
/// since version 120.
pub fn browser_trust(_browser: &Browser) -> BrowserTrust {
    BrowserTrust::System
}

/// A Firefox profile reads the keychains, as tracewing's sets
/// `security.enterprise_roots.enabled`.
pub fn trust_in_profile(_profile: &Path, _cert_pem: &Path) -> Result<()> {
    Ok(())
}

/// The system proxy is always there to set, and the stores need no tool.
pub fn environment() -> Environment {
    Environment::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// launchd keeps the waiter while there is a journal and only then, and
    /// reads every argument as written, whatever the data folder is called.
    #[test]
    fn the_waiter_lives_as_long_as_the_journal() {
        let command = [
            "/Applications/t & w.app/Contents/MacOS/tracewing",
            "recover",
            "--data-dir",
            "/d/<x>",
            "--wait",
        ]
        .map(str::to_owned);
        let plist = waiter_plist(
            "com.hivepaas.tracewing.proxy-waiter.0123456789abcdef",
            &command,
            Path::new("/d/<x>/sysproxy.journal"),
            Path::new("/d/<x>/recovery.log"),
        );
        assert!(
            plist.contains("<string>com.hivepaas.tracewing.proxy-waiter.0123456789abcdef</string>")
        );
        assert!(
            plist.contains("<string>/Applications/t &amp; w.app/Contents/MacOS/tracewing</string>"),
            "{plist}"
        );
        assert!(plist.contains("<string>/d/&lt;x&gt;</string>"), "{plist}");
        assert!(
            plist.contains(
                "<key>PathState</key>\n    <dict>\n      <key>/d/&lt;x&gt;/sysproxy.journal</key>\n      <true/>"
            ),
            "{plist}"
        );
        assert!(plist.contains("<key>ThrottleInterval</key>\n  <integer>60</integer>"));
        // Nothing that would bring it back at a login or with no journal.
        assert!(!plist.contains("RunAtLoad"), "{plist}");
        assert!(!plist.contains("SuccessfulExit"), "{plist}");
        // The plist parses: plutil is on every Mac.
        let file = std::env::temp_dir().join(format!("waiter-{}.plist", std::process::id()));
        fs::write(&file, &plist).unwrap();
        let lint = Command::new("/usr/bin/plutil")
            .arg("-lint")
            .arg(&file)
            .output()
            .unwrap();
        fs::remove_file(&file).ok();
        assert!(lint.status.success(), "{lint:?}");
    }

    /// The same folder, however it is spelled, is the same waiter; another
    /// folder is another.
    #[test]
    fn a_waiter_for_each_data_folder() {
        let dir = std::env::temp_dir().join(format!("waiter-label-{}", std::process::id()));
        fs::create_dir_all(dir.join("sub")).unwrap();
        let label = waiter_label(&dir.join("sysproxy.journal"));
        assert!(
            label.starts_with("com.hivepaas.tracewing.proxy-waiter."),
            "{label}"
        );
        assert_eq!(
            waiter_label(&dir.join("sub/../sysproxy.journal")),
            label,
            "the folder, not its spelling"
        );
        assert_ne!(waiter_label(&dir.join("sub/sysproxy.journal")), label);
        fs::remove_dir_all(&dir).ok();
    }
}
