//! Telling the operating system about the proxy: the root CA it must trust and
//! the system proxy it must use.
//!
//! Turning the system proxy on is the dangerous half. If the app dies with the
//! setting still on, every program on the machine loses the network, and
//! nothing tells the user why. So three things stand behind it:
//! - a journal of the previous settings, written before anything changes;
//! - a watchdog process, which puts them back the moment the app dies, under
//!   a name of its own so that a blanket kill of the app does not take the
//!   safety net with it;
//! - something the OS itself keeps running, for when the watchdog was killed
//!   along with the app, as `pkill tracewing` does: on Linux a timer that
//!   looks once a minute for an app that is gone and settings that are still
//!   its own, on macOS a launchd agent that waits for the app to go and that
//!   launchd starts again if it is killed;
//! - a recovery the OS runs at the next login, for when the machine went down
//!   with everything on it.
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

#[cfg(target_os = "linux")]
#[path = "quic_linux.rs"]
mod quic;
#[cfg(target_os = "macos")]
#[path = "quic_macos.rs"]
mod quic;
#[cfg(windows)]
#[path = "quic_windows.rs"]
mod quic;

mod alive;
pub mod android;
pub mod ios_sim;
pub mod launch;

pub use alive::{Alive, app_is_gone, hold_alive, wait_until_gone};

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
/// The second name the watchdog is started under on Windows and macOS, beside
/// the journal: see [`watchdog_program`].
#[cfg(windows)]
const WATCHDOG_EXE: &str = "trw-watchdog.exe";
#[cfg(target_os = "macos")]
const WATCHDOG_EXE: &str = "trw-watchdog";
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

/// A store that holds the root CA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// Where, in words.
    pub name: String,
    /// Browsers read this store. On Linux the system's store is read by
    /// command-line tools only, and browsers keep NSS stores of their own.
    pub browsers: bool,
    /// Who it serves, which is what taking the CA out of it needs.
    pub scope: Scope,
}

/// Where the root CA is installed, as far as the OS's stores can say.
pub fn ca_locations(cert_pem: &Path) -> Result<Vec<Place>> {
    os::ca_locations(cert_pem, &ca_der(cert_pem)?)
}

/// Whether a browser trusts the root CA, as far as its stores can say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserTrust {
    /// It reads the system's store, so it trusts the CA when the system does.
    System,
    /// It reads stores of its own: one for Chromium's family, one a profile
    /// for Firefox. `of` is 0 for a Firefox that has made no profile yet.
    Own { holding: usize, of: usize },
}

pub fn browser_trust(browser: &launch::Browser) -> BrowserTrust {
    os::browser_trust(browser)
}

pub fn ca_install(cert_pem: &Path, scope: Scope) -> Result<()> {
    os::ca_install(cert_pem, &ca_der(cert_pem)?, scope)
}

pub fn ca_uninstall(cert_pem: &Path, scope: Scope) -> Result<()> {
    os::ca_uninstall(cert_pem, &ca_der(cert_pem)?, scope)
}

/// Takes the CA out of every store that holds it: this user's first, which
/// needs no rights, then the machine's. One failing does not keep the other.
pub fn ca_remove(cert_pem: &Path) -> Result<()> {
    let places = ca_locations(cert_pem)?;
    let mut failed = Vec::new();
    for scope in [Scope::User, Scope::Machine] {
        if places.iter().any(|place| place.scope == scope)
            && let Err(e) = ca_uninstall(cert_pem, scope)
        {
            failed.push(format!("{e:#}"));
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        bail!("{}", failed.join("; "))
    }
}

/// What this machine lacks for the Set up page's steps to work, found by
/// looking rather than by trying.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    /// No system proxy setting tracewing can change, on this desktop, by its
    /// name (empty when it has none).
    pub no_system_proxy: Option<String>,
    /// `certutil` is missing, which browsers on Linux need to be told to
    /// trust the CA: the command that installs it, where known.
    pub no_certutil: Option<Option<&'static str>>,
}

pub fn environment() -> Environment {
    os::environment()
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
            // A CA a company signed: what leads to its root.
            ca.chain(),
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

/// Outgoing UDP to port 443 is refused on this machine while this lives, so apps
/// that would speak HTTP/3 fall back to TCP, and to the proxy. Putting it in
/// asks for an administrator. The OS takes it away by itself as soon as what
/// holds it ends, so a crash leaves nothing behind and taking it away needs no
/// rights: no journal and no watchdog.
pub struct QuicBlock(quic::QuicBlock);

pub fn block_quic(data_dir: &Path) -> Result<QuicBlock> {
    fs::create_dir_all(data_dir)?;
    quic::block_quic(data_dir).map(QuicBlock)
}

impl QuicBlock {
    pub fn release(self) -> Result<()> {
        self.0.release()
    }
}

/// Windows' elevated half of [`block_quic`]:
/// `block-quic --parent PID --name NAME --data-dir DIR`.
#[cfg(windows)]
pub fn quic_helper(parent: u32, name: &str, data_dir: &Path) -> Result<()> {
    quic::quic_helper(parent, name, data_dir)
}

#[cfg(not(windows))]
pub fn quic_helper(_parent: u32, _name: &str, _data_dir: &Path) -> Result<()> {
    bail!("only Windows blocks QUIC through a helper")
}

/// The system proxy is ours until this is released or dropped with the process.
pub struct TakenOver {
    data_dir: PathBuf,
    /// The watchdog waits for this pipe to close, which happens however this
    /// process ends.
    watchdog: Option<(Child, ChildStdin)>,
    /// The lock that tells the timer a running app from a dead one. `None`
    /// where it could not be taken, and then no timer is armed either.
    alive: Option<Alive>,
}

/// Points the system proxy at `addr`, journal and recovery first.
pub fn take_over(data_dir: &Path, addr: SocketAddr, watchdog: bool) -> Result<TakenOver> {
    recover(data_dir)?;
    fs::create_dir_all(data_dir)?;

    let saved = os::read_proxy()?;
    // The lock before the journal, so that whatever finds a journal can trust
    // the lock to say whether the app that wrote it is still here.
    let alive = match hold_alive(data_dir) {
        Ok(alive) => Some(alive),
        Err(e) => {
            tracing::warn!("no lock to tell a live app from a dead one: {e:#}");
            None
        }
    };
    write_journal(data_dir, addr, &saved)?;
    os::arm_recovery(&recovery_command(data_dir)?)?;
    // Without the lock the timer would have nothing to judge by, so it stays
    // off: the watchdog and the next login are then the whole of the safety
    // net, as they were before the timer existed.
    if alive.is_some()
        && let Err(e) = os::arm_timer(&timer_command(data_dir)?, &data_dir.join(JOURNAL_FILE))
    {
        tracing::warn!("no recovery timer, so a kill waits for the next login: {e:#}");
    }
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
        alive,
    })
}

impl TakenOver {
    pub fn release(mut self) -> Result<Recovered> {
        let recovered = recover(&self.data_dir)?;
        if let Some((mut child, stdin)) = self.watchdog.take() {
            drop(stdin);
            let _ = child.wait();
        }
        // The lock goes last. A timer that wakes in the middle of this finds
        // an app still running and leaves the work to the app, which is doing
        // it.
        drop(self.alive.take());
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
    /// The app is still running and the setting is still its own: there is
    /// nothing to put back, and taking the network from a working app would
    /// be the opposite of help. What the timer finds every minute of a
    /// perfectly ordinary session.
    StillRunning,
}

/// Puts back the settings in the journal, if there is one.
pub fn recover(data_dir: &Path) -> Result<Recovered> {
    let path = data_dir.join(JOURNAL_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        // Nothing of ours is pending, so neither net has anything to put
        // back. A run that ended badly can still have left them armed, and a
        // timer nobody disarms wakes the machine once a minute for ever.
        let _ = os::disarm_timer(&path);
        let _ = os::disarm_recovery();
        return Ok(Recovered::Nothing);
    };
    let (addr, saved) = parse_journal(&text).with_context(|| format!("read {}", path.display()))?;

    // Reading it can fail outright on a machine with no desktop to read, and
    // then the journal stays where it is: it is the only record of what the
    // settings were.
    let recovered = if os::proxy_is(addr).context("read this computer's proxy setting")? {
        os::restore_proxy(&saved, addr).context("put the system proxy back")?;
        Recovered::PutBack
    } else {
        Recovered::LeftAlone
    };
    os::disarm_recovery()?;
    // The settings are what matter: a timer left on is worth saying and not
    // worth failing over, having just done the one thing it was there for.
    if let Err(e) = os::disarm_timer(&path) {
        tracing::warn!("the recovery timer stays on: {e:#}");
    }
    fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    Ok(recovered)
}

/// Puts the settings back, but only once the app that changed them has gone.
/// This is what the timer runs, a minute at a time, and it is the one way to
/// ask for a recovery without having to know whether the app is running.
pub fn recover_if_gone(data_dir: &Path) -> Result<Recovered> {
    if !app_is_gone(data_dir) {
        return Ok(Recovered::StillRunning);
    }
    recover(data_dir)
}

/// Waits for the app that changed the settings to go, then puts them back.
/// This is what the launchd agent runs on macOS: it costs a process that
/// sleeps in the kernel, and gives the network back the moment the app has
/// gone rather than up to a minute later.
pub fn recover_when_gone(data_dir: &Path) -> Result<Recovered> {
    let path = data_dir.join(alive::ALIVE_FILE);
    wait_until_gone(data_dir).with_context(|| format!("wait on {}", path.display()))?;
    recover(data_dir)
}

/// The watchdog's body: wait for the app to go away, then clean up after it.
pub fn watchdog(data_dir: &Path) -> Result<()> {
    name_itself();
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

/// The watchdog takes a name of its own, because the whole of its job is to
/// outlive the app and `pkill tracewing` matches every name with tracewing in
/// it — the app's safety net included. It hides from nothing: `ps` shows the
/// same command line as before, `pkill -f tracewing` still finds it, and it
/// ends of its own accord a second after the app does. What it no longer does
/// is die in the same breath as the app it is there to clean up after.
///
/// Windows and macOS come by the same name another way, in
/// [`watchdog_program`].
#[cfg(target_os = "linux")]
fn name_itself() {
    // SAFETY: prctl takes a name of at most 16 bytes, ending in a zero, and
    // this one is written as such.
    unsafe { libc::prctl(libc::PR_SET_NAME, c"trw-watchdog".as_ptr()) };
}

/// Only Linux has a name apart from the program's.
#[cfg(not(target_os = "linux"))]
fn name_itself() {}

/// What to start the watchdog as.
///
/// On Linux it renames itself once it is running ([`name_itself`]). Windows
/// and macOS have no such call: a process answers to the name of the file it
/// was started from, so `taskkill /IM tracewing.exe` or `pkill tracewing`
/// would take the watchdog with the app it is there to clean up after. It is started from a second name for the
/// same file instead — a hard link where the volume allows one, a copy where
/// it does not — kept beside the journal and made again whenever the program
/// it names has changed.
///
/// Where none of that can be done, the program's own name: a watchdog that
/// answers to `tracewing` is still a watchdog, and the timer stands behind it.
///
/// On macOS the link is to the program inside the app bundle. It runs from
/// outside it as well: the signature is the file's own, and tracewing has no
/// entitlements that would need the bundle's provisioning profile. A file a
/// running watchdog was started from can be removed there, so the link is
/// always the program as it is now.
#[cfg(any(windows, target_os = "macos"))]
fn watchdog_program(data_dir: &Path) -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let named = data_dir.join(WATCHDOG_EXE);
    let same = |a: &fs::Metadata, b: &fs::Metadata| {
        a.len() == b.len() && a.modified().ok() == b.modified().ok()
    };
    match (fs::metadata(&named), fs::metadata(&exe)) {
        (Ok(there), Ok(now)) if same(&there, &now) => return Ok(named),
        // There, but of another version: a watchdog that is still running
        // from it holds the file, and then it is left as it is — old code
        // doing the one thing it has always done.
        (Ok(_), _) if fs::remove_file(&named).is_err() => return Ok(named),
        _ => {}
    }
    if fs::hard_link(&exe, &named).is_ok() {
        return Ok(named);
    }
    // Another volume, or a filesystem that keeps no links: the program's size
    // once, which is the price of a name of its own.
    match fs::copy(&exe, &named) {
        Ok(_) => Ok(named),
        Err(e) => {
            tracing::warn!("the watchdog goes by tracewing's own name: {e}");
            Ok(exe)
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn watchdog_program(_data_dir: &Path) -> Result<PathBuf> {
    Ok(std::env::current_exe()?)
}

fn spawn_watchdog(data_dir: &Path) -> Result<(Child, ChildStdin)> {
    let mut command = Command::new(watchdog_program(data_dir)?);
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

/// What the timer runs. The systemd timer starts it once a minute, and it
/// answers at once; the launchd agent starts it once, and it waits.
fn timer_command(data_dir: &Path) -> Result<Vec<String>> {
    let when = if cfg!(target_os = "macos") {
        "--wait"
    } else {
        "--only-if-gone"
    };
    Ok(vec![
        std::env::current_exe()?.display().to_string(),
        "recover".to_owned(),
        "--data-dir".to_owned(),
        data_dir.display().to_string(),
        when.to_owned(),
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

    /// The whole point of the lock: the timer runs `recover_if_gone` once a
    /// minute, and a run of it against a healthy app must be a no-op. If it
    /// were not, turning the system proxy on would take the network away a
    /// minute later.
    #[test]
    fn a_running_app_is_left_alone() {
        let dir = std::env::temp_dir().join(format!("alive-journal-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let journal = dir.join(JOURNAL_FILE);
        write_journal(&dir, "127.0.0.1:9090".parse().unwrap(), &Saved::default()).unwrap();

        let held = hold_alive(&dir).unwrap();
        assert_eq!(recover_if_gone(&dir).unwrap(), Recovered::StillRunning);
        assert!(
            journal.exists(),
            "and the journal is still there to be used"
        );

        // The other half — gone, as a kill leaves it, and the settings put
        // back — reaches into this machine's own autostart and timer, so it
        // is the command's test that runs it, in folders of its own.
        drop(held);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_broken_journal_is_an_error() {
        assert!(parse_journal("saved.a=1\n").is_err());
        assert!(parse_journal("what\n").is_err());
    }
}
