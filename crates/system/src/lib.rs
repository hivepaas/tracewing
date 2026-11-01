//! Telling the operating system about the proxy: the root CA it must trust and
//! the system proxy it must use.
//!
//! Turning the system proxy on is the dangerous half. If the app dies with the
//! setting still on, every program on the machine loses the network, and
//! nothing tells the user why. So three things stand behind it:
//! - a journal of the previous settings, written before anything changes;
//! - a watchdog process, which puts them back the moment the app dies;
//! - a recovery the OS runs at the next login, for when the watchdog dies too
//!   (a power cut, a killed process tree).
//!
//! Putting settings back only happens while they are still ours. If the user
//! has chosen another proxy in the meantime, that choice stays.

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod os;
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod os;
#[cfg(windows)]
#[path = "windows.rs"]
mod os;

use anyhow::{Context, Result, bail};
use hudsucker::rustls::{
    client::danger::ServerCertVerifier,
    pki_types::{CertificateDer, ServerName, UnixTime, pem::PemObject},
};
use std::{
    fs,
    io::{Read, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::Arc,
};

use tracewing_proxy::ca::LocalCa;

const JOURNAL_FILE: &str = "sysproxy.journal";
const WATCHDOG_LOG: &str = "watchdog.log";
/// The name issued in `verify_ca`; nothing ever connects to it.
const PROBE_HOST: &str = "probe.invalid";

/// Who the root CA is installed for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Scope {
    /// This user. No administrator rights, but the OS may ask to confirm.
    User,
    /// Every user of this machine. Needs administrator rights.
    Machine,
}

/// Settings as the OS holds them, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Saved(pub Vec<(String, String)>);

impl Saved {
    pub fn get(&self, key: &str) -> &str {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map_or("", |(_, v)| v.as_str())
    }

    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        self.0.push((key.to_owned(), value.into()));
    }
}

pub fn ca_der(cert_pem: &Path) -> Result<CertificateDer<'static>> {
    CertificateDer::from_pem_file(cert_pem).with_context(|| format!("read {}", cert_pem.display()))
}

/// Where the root CA is installed, as far as the OS's stores can say.
pub fn ca_locations(cert_pem: &Path) -> Result<Vec<String>> {
    os::ca_locations(cert_pem, &ca_der(cert_pem)?)
}

pub fn ca_install(cert_pem: &Path, scope: Scope) -> Result<()> {
    os::ca_install(cert_pem, &ca_der(cert_pem)?, scope)
}

pub fn ca_uninstall(cert_pem: &Path, scope: Scope) -> Result<()> {
    os::ca_uninstall(cert_pem, &ca_der(cert_pem)?, scope)
}

/// Asks the OS's own verifier whether it would accept a certificate from this
/// CA, which is the only check that means anything: a store can hold a root
/// the OS does not trust.
pub fn verify_ca(ca: &LocalCa, provider: hudsucker::rustls::crypto::CryptoProvider) -> Result<()> {
    let leaf = ca.issue_leaf(PROBE_HOST)?;
    let verifier = rustls_platform_verifier::Verifier::new(Arc::new(provider))
        .context("start the platform verifier")?;
    verifier
        .verify_server_cert(
            &leaf,
            &[],
            &ServerName::try_from(PROBE_HOST)?,
            &[],
            UnixTime::now(),
        )
        .context("the OS rejected a certificate from this CA")?;
    Ok(())
}

pub fn proxy_status() -> Result<Saved> {
    os::read_proxy()
}

/// The system proxy is ours until this is released or dropped with the process.
pub struct TakenOver {
    data_dir: PathBuf,
    /// The watchdog waits for this pipe to close, which happens however this
    /// process ends.
    watchdog: Option<(Child, ChildStdin)>,
}

/// Points the system proxy at `addr`, journal and recovery first.
pub fn take_over(data_dir: &Path, addr: SocketAddr, watchdog: bool) -> Result<TakenOver> {
    recover(data_dir)?;
    fs::create_dir_all(data_dir)?;

    let saved = os::read_proxy()?;
    write_journal(data_dir, addr, &saved)?;
    os::arm_recovery(&recovery_command(data_dir)?)?;
    let result = os::set_proxy(addr);
    if let Err(e) = result {
        recover(data_dir)?;
        return Err(e.context("turn the system proxy on"));
    }

    let watchdog = if watchdog {
        Some(spawn_watchdog(data_dir)?)
    } else {
        None
    };
    Ok(TakenOver {
        data_dir: data_dir.to_owned(),
        watchdog,
    })
}

impl TakenOver {
    pub fn release(mut self) -> Result<Recovered> {
        let recovered = recover(&self.data_dir)?;
        if let Some((mut child, stdin)) = self.watchdog.take() {
            drop(stdin);
            let _ = child.wait();
        }
        Ok(recovered)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Recovered {
    /// No journal: nothing of ours was left behind.
    Nothing,
    PutBack,
    /// The journal was there, but the settings had changed since.
    LeftAlone,
}

/// Puts back the settings in the journal, if there is one.
pub fn recover(data_dir: &Path) -> Result<Recovered> {
    let path = data_dir.join(JOURNAL_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(Recovered::Nothing);
    };
    let (addr, saved) = parse_journal(&text).with_context(|| format!("read {}", path.display()))?;

    let recovered = if os::proxy_is(addr)? {
        os::restore_proxy(&saved, addr).context("put the system proxy back")?;
        Recovered::PutBack
    } else {
        Recovered::LeftAlone
    };
    os::disarm_recovery()?;
    fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    Ok(recovered)
}

/// The watchdog's body: wait for the app to go away, then clean up after it.
pub fn watchdog(data_dir: &Path) -> Result<()> {
    let mut sink = [0u8; 64];
    let mut stdin = std::io::stdin();
    while stdin.read(&mut sink).is_ok_and(|n| n > 0) {}

    let outcome = recover(data_dir);
    let mut log = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(data_dir.join(WATCHDOG_LOG))?;
    writeln!(log, "app gone: {outcome:?}")?;
    outcome.map(|_| ())
}

fn spawn_watchdog(data_dir: &Path) -> Result<(Child, ChildStdin)> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("watchdog")
        .arg("--data-dir")
        .arg(data_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    detach(&mut command);
    let mut child = command.spawn().context("start the watchdog")?;
    let stdin = child.stdin.take().context("no pipe to the watchdog")?;
    Ok((child, stdin))
}

/// Keeps the watchdog alive when the app's terminal or console goes away.
fn detach(command: &mut Command) {
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(command, 0);
    #[cfg(windows)]
    {
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        std::os::windows::process::CommandExt::creation_flags(
            command,
            DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP,
        );
    }
}

fn recovery_command(data_dir: &Path) -> Result<Vec<String>> {
    Ok(vec![
        std::env::current_exe()?.display().to_string(),
        "sysproxy".to_owned(),
        "recover".to_owned(),
        "--data-dir".to_owned(),
        data_dir.display().to_string(),
    ])
}

fn write_journal(data_dir: &Path, addr: SocketAddr, saved: &Saved) -> Result<()> {
    let mut text = format!("addr={addr}\n");
    for (key, value) in &saved.0 {
        if value.contains('\n') {
            bail!("system proxy setting {key} has a line break");
        }
        text.push_str(&format!("saved.{key}={value}\n"));
    }
    // A journal cut short would lose the settings, so it appears whole or not at all.
    let tmp = data_dir.join(format!("{JOURNAL_FILE}.tmp"));
    let mut file = fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(&tmp, data_dir.join(JOURNAL_FILE))?;
    Ok(())
}

fn parse_journal(text: &str) -> Result<(SocketAddr, Saved)> {
    let mut addr = None;
    let mut saved = Saved::default();
    for line in text.lines() {
        let (key, value) = line.split_once('=').context("a line without =")?;
        match key.strip_prefix("saved.") {
            Some(key) => saved.set(key, value),
            None if key == "addr" => addr = Some(value.parse()?),
            None => bail!("unknown key {key}"),
        }
    }
    Ok((addr.context("no addr")?, saved))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_round_trip() {
        let dir = std::env::temp_dir().join(format!("journal-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut saved = Saved::default();
        saved.set("server", "http=a:1;https=a:2");
        saved.set("empty", "");
        let addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();
        write_journal(&dir, addr, &saved).unwrap();
        let text = fs::read_to_string(dir.join(JOURNAL_FILE)).unwrap();
        assert_eq!(parse_journal(&text).unwrap(), (addr, saved));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_broken_journal_is_an_error() {
        assert!(parse_journal("saved.a=1\n").is_err());
        assert!(parse_journal("what\n").is_err());
    }
}
