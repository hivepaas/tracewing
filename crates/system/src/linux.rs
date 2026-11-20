//! Linux has no one system proxy and no one trust store.
//!
//! - The proxy: GNOME and its relatives keep it in `gsettings`, KDE in
//!   `kioslaverc`. Other desktops have nothing to set.
//! - Trust: the distribution's store (`Scope::Machine`) serves curl, Python and
//!   most command-line programs. Chrome and Firefox do not read it: they read
//!   NSS databases, one per browser and profile (`Scope::User`).
//!
//! Recovery at the next login is an XDG autostart entry, and the minute timer
//! behind it is a systemd user timer.

use anyhow::{Context, Result, bail};
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
};

use hudsucker::rustls::pki_types::{CertificateDer, pem::PemObject};

use super::{
    BrowserTrust, Environment, Place, Saved, Scope,
    launch::{self, Browser, Engine},
};

const NICKNAME: &str = "tracewing CA";
/// More certificates under `NICKNAME` than `nss_forget` will take out of one
/// database before it says the database is not making progress.
const MOST_UNDER_NICKNAME: usize = 32;
const ANCHOR_NAME: &str = "tracewing-ca";
const AUTOSTART_FILE: &str = "tracewing-proxy-recovery.desktop";
const TIMER_SERVICE: &str = "tracewing-proxy-recovery.service";
const TIMER_UNIT: &str = "tracewing-proxy-recovery.timer";
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

fn config_home() -> Result<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .map_or_else(|| Ok::<_, anyhow::Error>(home()?.join(".config")), Ok)
}

fn autostart_path() -> Result<PathBuf> {
    Ok(config_home()?.join("autostart").join(AUTOSTART_FILE))
}

fn units_dir() -> Result<PathBuf> {
    Ok(config_home()?.join("systemd").join("user"))
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

/// Quotes one argument of a unit file's `ExecStart=`, by systemd's rules: a
/// double-quoted word, backslashes and quotes escaped inside it, and `%`
/// doubled so that systemd does not read what follows as a specifier of its
/// own. A data folder may be anywhere and may be called anything.
fn unit_quote(part: &str) -> String {
    let inner = part
        .replace('\\', r"\\")
        .replace('"', "\\\"")
        .replace('%', "%%");
    format!("\"{inner}\"")
}

/// Runs `command` once a minute, for as long as the system proxy is ours.
///
/// A systemd user timer, which goes with the user's session: it starts when
/// they log in and stops when they log out, and it survives `pkill tracewing`
/// because systemd, not the app, is what starts it. The service does nothing
/// at all while there is no journal, and says so as a condition, so most
/// minutes cost not even a process.
pub fn arm_timer(command: &[String], journal: &Path) -> Result<()> {
    if !on_path("systemctl") {
        bail!("systemd is what runs the timer, and systemctl is not on the path");
    }
    let dir = units_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    // What systemd knows of the session is not always what the session knows:
    // the desktop's name may never reach the user manager, and then the
    // command would not know where this machine keeps its proxy setting.
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
    for (name, body) in unit_files(command, journal, desktop.as_deref()) {
        let path = dir.join(name);
        fs::write(&path, &body).with_context(|| format!("write {}", path.display()))?;
    }
    run("systemctl", &["--user", "daemon-reload"])?;
    run("systemctl", &["--user", "enable", "--now", TIMER_UNIT])?;
    Ok(())
}

/// The two unit files, by name: what to run, and when to run it.
fn unit_files(
    command: &[String],
    journal: &Path,
    desktop: Option<&str>,
) -> [(&'static str, String); 2] {
    let exec = command
        .iter()
        .map(|part| unit_quote(part))
        .collect::<Vec<_>>()
        .join(" ");
    // A path is written as it is, except for the per-cent sign, which systemd
    // would read as the start of a name of its own.
    let journal = journal.display().to_string().replace('%', "%%");
    let desktop = desktop
        .map(|name| {
            format!(
                "Environment={}\n",
                unit_quote(&format!("XDG_CURRENT_DESKTOP={name}"))
            )
        })
        .unwrap_or_default();
    [
        (
            TIMER_SERVICE,
            // LogLevelMax keeps systemd from saying it has started and
            // finished this, once a minute, for the whole of a session. What
            // the command itself has to say still goes to the journal.
            format!(
                "[Unit]\n\
                 Description=Put the system proxy back if tracewing is gone\n\
                 ConditionPathExists={journal}\n\
                 \n\
                 [Service]\n\
                 Type=oneshot\n\
                 LogLevelMax=warning\n\
                 {desktop}\
                 ExecStart={exec}\n"
            ),
        ),
        (
            TIMER_UNIT,
            // OnActiveSec for the first look after the timer is armed,
            // OnUnitActiveSec for every one after that. Without the first
            // there is no second: a minute after the last run, with no last
            // run, is never.
            "[Unit]\n\
             Description=Look once a minute for a tracewing that left the system proxy behind\n\
             \n\
             [Timer]\n\
             OnActiveSec=1min\n\
             OnUnitActiveSec=1min\n\
             AccuracySec=10s\n\
             \n\
             [Install]\n\
             WantedBy=timers.target\n"
                .to_owned(),
        ),
    ]
}

pub fn disarm_timer(_journal: &Path) -> Result<()> {
    let dir = units_dir()?;
    let units = [dir.join(TIMER_SERVICE), dir.join(TIMER_UNIT)];
    // Nothing was armed: say so without waking systemd to ask.
    if units.iter().all(|path| !path.exists()) {
        return Ok(());
    }
    let stopped = run("systemctl", &["--user", "disable", "--now", TIMER_UNIT]);
    for path in &units {
        match fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(anyhow::Error::from(e).context(format!("remove {}", path.display())));
            }
            _ => {}
        }
    }
    let _ = run("systemctl", &["--user", "daemon-reload"]);
    stopped.map(|_| ())
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

/// Runs a shell script as root: directly if we are, else through polkit's
/// dialog. `what` is what the script is for, in a few words, and it is what
/// a failure is reported as.
///
/// **The script must not reach the error**, which is why `what` exists.
/// Measured: the adapter step failing left a message of 971 characters over
/// 17 lines -- the whole script, comments and all, with the one useful
/// sentence after the last of them. Worse, a person who answers No to the
/// dialog gets the same wall with **nothing** after it, since `pkexec` says
/// that in its exit code rather than on its error stream.
pub(crate) fn run_as_admin(what: &str, script: &str) -> Result<()> {
    if run("id", &["-u"]).is_ok_and(|id| id == "0") {
        run("sh", &["-c", script]).with_context(|| what.to_owned())?;
        return Ok(());
    }
    let out = Command::new("pkexec")
        .args(["sh", "-c", script])
        .output()
        .with_context(|| format!("{what}: run pkexec"))?;
    if out.status.success() {
        return Ok(());
    }
    // The case travels, and `what` is the context in front of it, so
    // `{e:#}` still reads as the one sentence it did before.
    Err(anyhow::Error::new(why_pkexec_failed(
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).trim(),
    ))
    .context(what.to_owned()))
}

/// Why a privileged step did not run.
///
/// **Cases rather than a sentence, and that is measured rather than
/// preferred**: this crate contains no `tr!` at all, so words carried from
/// here to the window are untranslated English in somebody's Vietnamese or
/// Russian tab. A caller that had to tell apart the one case somebody can
/// act on -- rights not given -- would be matching on these words and then
/// showing them.
///
/// It travels inside [`anyhow::Error`], so every caller that only wanted a
/// message is unchanged and the one that cares asks
/// `err.downcast_ref::<Denied>()`. The [`std::fmt::Display`] is still the
/// line a log gets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Denied {
    /// The dialog was dismissed, or answered No. **Nothing ran**, so there
    /// is nothing to put back and nothing to warn about -- which is what
    /// lets a caller turn its switch off again rather than report a fault.
    NotGiven,
    /// There was no way to ask: polkit answered, and no agent is running to
    /// show a dialog with. A different remedy from the above, which is the
    /// whole reason it is a different case.
    NoAgent,
    /// Anything else, in the words that happened: the script's own last
    /// line, or a status with nothing said. Prose, and the only one that is
    /// -- an unexpected failure has no sentence written for it in fourteen
    /// languages, and inventing one that says nothing would be worse.
    Said(String),
}

impl std::fmt::Display for Denied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotGiven => write!(f, "administrator rights were not given"),
            Self::NoAgent => {
                write!(
                    f,
                    "no password dialog could be shown (no polkit agent is running)"
                )
            }
            Self::Said(said) => write!(f, "{said}"),
        }
    }
}

impl std::error::Error for Denied {}

/// What to say about a `pkexec` that did not run the script.
///
/// The two that are its own rather than the script's are worth telling
/// apart, because what somebody does next differs: answering the dialog
/// again, or starting an agent. The strings and the two exit codes are the
/// ones [`crate::quic`]'s own reader already matches -- `pkexec(1)` gives 126
/// for "not authorized" and 127 for "the dialog was dismissed", and says
/// nothing on its error stream for either.
///
/// Anything else is handed on as the script's own complaint, which is the
/// case that was already readable.
fn why_pkexec_failed(code: Option<i32>, said: &str) -> Denied {
    let first = said.lines().next().unwrap_or_default().trim();
    if first.contains("Not authorized")
        || first.contains("dismissed")
        || (said.is_empty() && matches!(code, Some(126 | 127)))
    {
        return Denied::NotGiven;
    }
    if first.contains("authentication agent") {
        return Denied::NoAgent;
    }
    if said.is_empty() {
        return Denied::Said(match code {
            Some(code) => format!("it stopped with status {code} and said nothing"),
            None => "it was killed before it could finish".to_owned(),
        });
    }
    // The last line, not the first: `set -e` scripts say what went wrong
    // where they stopped, and what comes before it is progress.
    Denied::Said(
        said.lines()
            .map(str::trim)
            .rev()
            .find(|line| !line.is_empty())
            .unwrap_or(first)
            .to_owned(),
    )
}

fn quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

/// Where Chromium's family reads NSS: `~/.pki/nssdb`, or the same in the home
/// a snap or a Flatpak gives it.
fn chromium_databases(home: &Path) -> Vec<PathBuf> {
    let mut homes = vec![home.to_owned(), home.join("snap/chromium/current")];
    homes.extend(
        FLATPAKS
            .iter()
            .filter(|(_, _, engine)| *engine == Engine::Chromium)
            .map(|(id, _, _)| home.join(".var/app").join(id)),
    );
    homes.iter().map(|home| home.join(".pki/nssdb")).collect()
}

/// The folders Firefox keeps its profiles in: the old one and XDG's, as a
/// package, a snap and a Flatpak have them.
fn firefox_folders(home: &Path, snap: bool, flatpak: bool) -> Vec<PathBuf> {
    let mut folders = vec![
        home.join(".mozilla/firefox"),
        home.join(".config/mozilla/firefox"),
    ];
    if snap {
        folders.push(home.join("snap/firefox/common/.mozilla/firefox"));
    }
    if flatpak {
        folders.extend(flatpak_firefox_folders(home));
    }
    folders
}

fn flatpak_firefox_folders(home: &Path) -> [PathBuf; 2] {
    let sandbox = home.join(".var/app/org.mozilla.firefox");
    [
        sandbox.join(".mozilla/firefox"),
        sandbox.join("config/mozilla/firefox"),
    ]
}

/// The profiles in `folders`: those with a database, which Firefox makes as it
/// first opens one.
fn firefox_profiles(folders: &[PathBuf]) -> Vec<PathBuf> {
    let mut profiles = Vec::new();
    for folder in folders {
        let Ok(entries) = fs::read_dir(folder) else {
            continue;
        };
        let mut here: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.join("cert9.db").exists())
            .collect();
        here.sort();
        profiles.extend(here);
    }
    profiles
}

/// Every NSS database of this user that a browser would read. `create` makes
/// the ones a Chromium would read but has not made yet: always the shared one,
/// and in a sandbox's home once the browser has made that home.
fn nss_databases(create: bool) -> Result<Vec<PathBuf>> {
    let home = home()?;
    let mut found = Vec::new();
    for db in chromium_databases(&home) {
        let shared = db.parent().and_then(Path::parent) == Some(home.as_path());
        let sandbox_made = db.parent().and_then(Path::parent).is_some_and(Path::is_dir);
        if create && !db.join("cert9.db").exists() && (shared || sandbox_made) {
            fs::create_dir_all(&db)?;
            run(
                "certutil",
                &[
                    "-N",
                    "--empty-password",
                    "-d",
                    &format!("sql:{}", db.display()),
                ],
            )?;
        }
        if db.join("cert9.db").exists() {
            found.push(db);
        }
    }
    found.extend(firefox_profiles(&firefox_folders(&home, true, true)));
    Ok(found)
}

/// Makes a Firefox profile of tracewing's trust the CA: NSS, as in every
/// profile on Linux, made before Firefox first opens it.
pub fn trust_in_profile(profile: &Path, cert_pem: &Path) -> Result<()> {
    if !on_path("certutil") {
        bail!("Firefox needs certutil (libnss3-tools or nss-tools) to trust the CA");
    }
    let db = format!("sql:{}", profile.display());
    if !profile.join("cert9.db").exists() {
        run("certutil", &["-N", "--empty-password", "-d", &db])?;
    }
    nss_add(profile, cert_pem)
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

fn sql(db: &Path) -> String {
    format!("sql:{}", db.display())
}

/// Whether `db` has anything at all under tracewing's nickname, whichever
/// certificate it is. Install and uninstall act on this, because the nickname
/// is tracewing's own: whatever answers to it is ours to replace or take out.
fn nss_has_nickname(db: &Path) -> bool {
    run("certutil", &["-L", "-d", &sql(db), "-n", NICKNAME]).is_ok()
}

/// Whether `db` holds *this* CA, by its bytes.
///
/// **The nickname cannot answer this.** Every tracewing CA is named the same,
/// so a database left holding the CA of another data directory, or of a build
/// of your own, answers to the nickname and is not this one. Asked by name,
/// the Trusted page said the browsers trusted this CA while they were
/// rejecting the leaves it signs, and nothing said why.
///
/// `-a` prints one certificate: the one the nickname resolves to. A database
/// an older tracewing left holding two can therefore hide this CA behind the
/// other, and this says no. That under-reports in the one direction that
/// mends itself, since trusting the CA again goes through `nss_add`, which
/// leaves exactly one.
fn nss_holds(db: &Path, der: &[u8]) -> bool {
    run("certutil", &["-L", "-d", &sql(db), "-n", NICKNAME, "-a"])
        .ok()
        .and_then(|pem| CertificateDer::from_pem_slice(pem.as_bytes()).ok())
        .is_some_and(|held| held.as_ref() == der)
}

/// Takes out whichever certificate tracewing's nickname in `db` names, so
/// that `-A` can put this CA there alone, and says how many went.
///
/// **`certutil -A` does not replace.** Adding under a nickname a certificate
/// of other content leaves both, and the nickname then names whichever went
/// in last. So installing again left two, and the one the nickname did not
/// name was reachable by nothing certutil offers. The loop is why: two
/// certificates of different content accumulate, and clearing the nickname
/// means clearing it until it names nothing.
///
/// Measured with certutil: both adds exit 0, one `-D` leaves one behind, a
/// second takes it, and with none left `-D` fails with
/// `SEC_ERROR_INVALID_ARGS`. The loop asks before each `-D` rather than
/// reading that failure, so it ends because nothing is left, and a certutil
/// that refuses a certificate still there says so in its own words.
///
/// **This clears by name, which only install may do**: it is putting a CA in
/// the one slot the nickname is, at somebody's asking. Uninstall must not --
/// see [`ca_uninstall`].
fn nss_clear(db: &Path) -> Result<usize> {
    let mut gone = 0;
    while nss_has_nickname(db) {
        if gone == MOST_UNDER_NICKNAME {
            bail!(
                "{} still has a certificate named {NICKNAME} after taking out {gone}",
                db.display()
            );
        }
        run("certutil", &["-D", "-d", &sql(db), "-n", NICKNAME])
            .with_context(|| format!("take tracewing's certificate out of {}", db.display()))?;
        gone += 1;
    }
    Ok(gone)
}

/// Makes `db` trust the CA, and only it: whatever the nickname names goes
/// first, since `-A` on its own would leave both ([`nss_clear`]).
fn nss_add(db: &Path, cert_pem: &Path) -> Result<()> {
    nss_clear(db)?;
    let pem = cert_pem.display().to_string();
    run(
        "certutil",
        &[
            "-A",
            "-d",
            &sql(db),
            "-t",
            "C,,",
            "-n",
            NICKNAME,
            "-i",
            &pem,
        ],
    )
    .map(drop)
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
            scope: Scope::Machine,
        });
    }
    if on_path("certutil") {
        for db in nss_databases(false)? {
            if nss_holds(&db, der) {
                places.push(Place {
                    name: format!("NSS: {}", db.display()),
                    browsers: true,
                    scope: Scope::User,
                });
            }
        }
    }
    Ok(places)
}

/// Whether a store for `scope` holds something of tracewing's that
/// [`ca_locations`](crate::ca_locations) does not report, because what it
/// holds is a tracewing CA other than this one: another data directory's,
/// left where only the name says whose it is.
///
/// Both stores here are tracewing's by name: the anchor file is at a path of
/// ours -- which `ca_locations` already says may hold an older CA -- and in
/// NSS the nickname is ours. That is enough to say something of tracewing's is
/// trusted, and not enough to take it out: see
/// [`an_older_ca_is_trusted`](crate::an_older_ca_is_trusted).
pub fn ours_by_name(scope: Scope) -> bool {
    match scope {
        Scope::Machine => system_store().is_ok_and(|(dir, _)| anchor_path(dir).exists()),
        Scope::User => {
            on_path("certutil")
                && nss_databases(false).is_ok_and(|dbs| dbs.iter().any(|db| nss_has_nickname(db)))
        }
    }
}

pub fn ca_install(cert_pem: &Path, _der: &[u8], scope: Scope) -> Result<()> {
    match scope {
        Scope::Machine => {
            let (dir, apply) = system_store()?;
            run_as_admin(
                "add tracewing's certificate to this computer's store",
                &format!(
                    "install -m 0644 {} {} && {}",
                    quote(cert_pem),
                    quote(&anchor_path(dir)),
                    apply.join(" ")
                ),
            )
        }
        Scope::User => {
            if !on_path("certutil") {
                bail!("browsers need certutil (libnss3-tools or nss-tools) to trust the CA");
            }
            each_database(nss_databases(true)?, |db| nss_add(db, cert_pem))
        }
    }
}

pub fn ca_uninstall(_cert_pem: &Path, der: &[u8], scope: Scope) -> Result<()> {
    match scope {
        Scope::Machine => {
            let (dir, apply) = system_store()?;
            run_as_admin(
                "take tracewing's certificate out of this computer's store",
                &format!("rm -f {} && {}", quote(&anchor_path(dir)), apply.join(" ")),
            )
        }
        Scope::User => {
            // The databases holding *this* CA, and one `-D` in each.
            //
            // **One, not a loop.** NSS keeps one copy of a given certificate:
            // measured, adding the same bytes again under the same nickname
            // leaves one, and only certificates of different content
            // accumulate. `-D` takes out the one the nickname names, which is
            // the one `nss_holds` just read and compared.
            //
            // **And by the bytes, not by the name.** The nickname may also
            // name another data directory's CA, and that one is not ours to
            // take out: the directory may still be in use, and nothing here
            // can tell. `tracewing uninstall` says it is still trusted
            // instead -- see `crate::an_older_ca_is_trusted`.
            let holding = nss_databases(false)?
                .into_iter()
                .filter(|db| nss_holds(db, der));
            each_database(holding.collect(), |db| {
                run("certutil", &["-D", "-d", &sql(db), "-n", NICKNAME])
                    .with_context(|| {
                        format!("take tracewing's certificate out of {}", db.display())
                    })
                    .map(drop)
            })
        }
    }
}

pub fn environment() -> Environment {
    Environment {
        no_system_proxy: desktop()
            .is_err()
            .then(|| std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default()),
        no_certutil: (!on_path("certutil"))
            .then(|| certutil_package(&fs::read_to_string("/etc/os-release").unwrap_or_default())),
        old_node: None,
    }
}

/// The command that installs `certutil` on the distribution `os-release`
/// describes, by its id or the ids it is like.
fn certutil_package(os_release: &str) -> Option<&'static str> {
    let ids: Vec<&str> = os_release
        .lines()
        .filter_map(|line| {
            line.strip_prefix("ID=")
                .or_else(|| line.strip_prefix("ID_LIKE="))
        })
        .flat_map(|value| value.trim_matches(['"', '\'']).split_whitespace())
        .collect();
    const COMMANDS: &[(&[&str], &str)] = &[
        (&["debian", "ubuntu"], "sudo apt install libnss3-tools"),
        (&["fedora", "rhel", "centos"], "sudo dnf install nss-tools"),
        (&["arch"], "sudo pacman -S nss"),
        (
            &["suse", "opensuse", "opensuse-leap", "opensuse-tumbleweed"],
            "sudo zypper install mozilla-nss-tools",
        ),
        (&["alpine"], "sudo apk add nss-tools"),
    ];
    ids.iter().find_map(|id| {
        COMMANDS
            .iter()
            .find(|(names, _)| names.contains(id))
            .map(|(_, command)| *command)
    })
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

pub fn open_terminal(
    env: &[(&str, String)],
    banner: &str,
    _data_dir: &Path,
    level: crate::child::Level,
) -> Result<()> {
    let (program, flags) = find_terminal().context("no terminal found: set $TERMINAL to one")?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
    let mut command = Command::new(&program);
    command.args(terminal_command(flags, env, banner, &shell));
    if let Ok(home) = home() {
        command.current_dir(home);
    }
    launch::spawn(command, level).with_context(|| format!("open {}", program.display()))
}

/// Browsers on the `PATH`, by program, and what they are.
const BROWSERS: &[(&str, &str, Engine)] = &[
    ("google-chrome", "Chrome", Engine::Chromium),
    ("google-chrome-stable", "Chrome", Engine::Chromium),
    ("chromium", "Chromium", Engine::Chromium),
    ("chromium-browser", "Chromium", Engine::Chromium),
    ("microsoft-edge", "Edge", Engine::Chromium),
    ("microsoft-edge-stable", "Edge", Engine::Chromium),
    ("brave-browser", "Brave", Engine::Chromium),
    ("opera", "Opera", Engine::Chromium),
    ("firefox", "Firefox", Engine::Firefox),
];

/// Browsers installed as Flatpaks, by app id.
const FLATPAKS: &[(&str, &str, Engine)] = &[
    ("com.google.Chrome", "Chrome", Engine::Chromium),
    ("org.chromium.Chromium", "Chromium", Engine::Chromium),
    ("com.microsoft.Edge", "Edge", Engine::Chromium),
    ("com.brave.Browser", "Brave", Engine::Chromium),
    ("org.mozilla.firefox", "Firefox", Engine::Firefox),
];

pub fn find_browsers(profiles: &Path) -> Vec<Browser> {
    let Ok(home) = home() else {
        return Vec::new();
    };
    let mut found: Vec<Browser> = Vec::new();
    for &(program, name, engine) in BROWSERS {
        if found.iter().any(|b| b.name == name) {
            continue;
        }
        if let Some(path) = launch::on_path(program) {
            found.push(on_the_path(&home, profiles, program, path, name, engine));
        }
    }
    // Flatpak puts its launchers where the PATH seldom looks.
    let exports = [
        PathBuf::from("/var/lib/flatpak/exports/bin"),
        home.join(".local/share/flatpak/exports/bin"),
    ];
    for &(id, name, engine) in FLATPAKS {
        if found.iter().any(|b| b.name == name) {
            continue;
        }
        let Some(program) = exports.iter().map(|dir| dir.join(id)).find(|p| p.is_file()) else {
            continue;
        };
        // A Flatpak writes only in its own home.
        let sandbox = home.join(".var/app").join(id);
        found.push(Browser {
            name,
            engine,
            program,
            profile: sandbox.join("tracewing-profile"),
            nss: match engine {
                Engine::Firefox => flatpak_firefox_folders(&home).to_vec(),
                _ => vec![sandbox.join(".pki/nssdb")],
            },
        });
    }
    found
}

fn on_the_path(
    home: &Path,
    profiles: &Path,
    program: &str,
    path: PathBuf,
    name: &'static str,
    engine: Engine,
) -> Browser {
    // A snap may write only in its own folder of the home. Ubuntu's
    // chromium-browser and firefox are scripts that start the snap.
    let snap_name = if program.starts_with("chromium") {
        "chromium"
    } else {
        program
    };
    let snap = path.starts_with("/snap/")
        || matches!(snap_name, "chromium" | "firefox")
            && Path::new("/snap/bin").join(snap_name).exists();
    let profile = if snap {
        home.join("snap")
            .join(snap_name)
            .join("common/tracewing-profile")
    } else {
        profiles.join(name.to_lowercase())
    };
    let nss = match engine {
        Engine::Firefox => firefox_folders(home, snap, false),
        _ if snap => vec![home.join("snap").join(snap_name).join("current/.pki/nssdb")],
        _ => vec![home.join(".pki/nssdb")],
    };
    Browser {
        name,
        engine,
        program: path,
        profile,
        nss,
    }
}

pub fn browser_trust(browser: &Browser, der: &[u8]) -> BrowserTrust {
    let dbs: Vec<PathBuf> = match browser.engine {
        Engine::Firefox => firefox_profiles(&browser.nss),
        // A snap or a Flatpak never started has no home to put one in yet.
        // One not made, in a home there is, holds nothing until it is made.
        _ => browser
            .nss
            .iter()
            .filter(|db| db.parent().and_then(Path::parent).is_some_and(Path::is_dir))
            .cloned()
            .collect(),
    };
    let tool = on_path("certutil");
    let holding = dbs
        .iter()
        .filter(|db| tool && db.join("cert9.db").exists() && nss_holds(db, der))
        .count();
    BrowserTrust::Own {
        holding,
        of: dbs.len(),
    }
}

#[cfg(test)]
mod tests {
    /// Which case a failed privileged step is, and what it says.
    ///
    /// The two `pkexec` answers are told apart because what somebody does
    /// next differs, and the dismissed one says **nothing**: it is the exit
    /// code or it is a wall of script with an empty line after it. They are
    /// cases rather than sentences because a window has to show a translated
    /// one -- see [`Denied`] -- so both are asserted here: the case a caller
    /// matches on, and the line a log gets.
    #[test]
    fn a_privileged_step_that_failed_says_which_and_why() {
        // Its own refusals, with nothing on the error stream.
        assert_eq!(why_pkexec_failed(Some(126), ""), Denied::NotGiven);
        assert_eq!(why_pkexec_failed(Some(127), ""), Denied::NotGiven);
        // And when it does say so.
        assert_eq!(
            why_pkexec_failed(
                Some(1),
                "Error executing command as another user: Not authorized"
            ),
            Denied::NotGiven
        );
        assert_eq!(
            why_pkexec_failed(
                Some(1),
                "polkit-agent-helper-1: no authentication agent found"
            ),
            Denied::NoAgent
        );
        // The script's own complaint, which is the case that already read
        // well -- and the **last** line of it, because `set -e` stops where
        // it went wrong and what comes before is progress.
        assert_eq!(
            why_pkexec_failed(
                Some(255),
                "RTNETLINK answers: File exists\nError: either \"name\" is duplicate."
            ),
            Denied::Said("Error: either \"name\" is duplicate.".to_owned())
        );
        // A status with nothing said at all, which is not the same as being
        // refused: 126 and 127 are `pkexec`'s own, anything else is the
        // script's silence.
        assert_eq!(
            why_pkexec_failed(Some(2), ""),
            Denied::Said("it stopped with status 2 and said nothing".to_owned())
        );
        assert_eq!(
            why_pkexec_failed(None, ""),
            Denied::Said("it was killed before it could finish".to_owned())
        );
        // The sentences themselves, which are what a log keeps and what the
        // one case with no translation shows.
        assert_eq!(
            Denied::NotGiven.to_string(),
            "administrator rights were not given"
        );
        assert_eq!(
            Denied::NoAgent.to_string(),
            "no password dialog could be shown (no polkit agent is running)"
        );
        assert_eq!(
            Denied::Said("as it came".to_owned()).to_string(),
            "as it came"
        );
    }

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
    fn a_browser_reads_the_stores_of_how_it_is_installed() {
        let home = Path::new("/h");
        let profiles = Path::new("/d/browser");
        let chrome = on_the_path(
            home,
            profiles,
            "google-chrome",
            "/usr/bin/google-chrome".into(),
            "Chrome",
            Engine::Chromium,
        );
        assert_eq!(chrome.nss, [PathBuf::from("/h/.pki/nssdb")]);
        assert_eq!(chrome.profile, Path::new("/d/browser/chrome"));
        let snap = on_the_path(
            home,
            profiles,
            "chromium",
            "/snap/bin/chromium".into(),
            "Chromium",
            Engine::Chromium,
        );
        assert_eq!(
            snap.nss,
            [PathBuf::from("/h/snap/chromium/current/.pki/nssdb")]
        );
        assert_eq!(
            snap.profile,
            Path::new("/h/snap/chromium/common/tracewing-profile")
        );
        let firefox = on_the_path(
            home,
            profiles,
            "firefox",
            "/snap/bin/firefox".into(),
            "Firefox",
            Engine::Firefox,
        );
        assert!(
            firefox
                .nss
                .contains(&PathBuf::from("/h/snap/firefox/common/.mozilla/firefox"))
        );
        assert!(firefox.nss.contains(&PathBuf::from("/h/.mozilla/firefox")));
    }

    #[test]
    fn a_firefox_profile_counts_once_it_has_a_database() {
        let dir = std::env::temp_dir().join(format!("tracewing-ff-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for (profile, db) in [("b.work", true), ("a.default", true), ("c.new", false)] {
            fs::create_dir_all(dir.join(profile)).unwrap();
            if db {
                fs::write(dir.join(profile).join("cert9.db"), "").unwrap();
            }
        }
        fs::write(dir.join("profiles.ini"), "").unwrap();
        assert_eq!(
            firefox_profiles(&[dir.clone(), dir.join("missing")]),
            [dir.join("a.default"), dir.join("b.work")]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn certutil_comes_from_the_distributions_package() {
        let ubuntu = "NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\n";
        assert_eq!(
            certutil_package(ubuntu),
            Some("sudo apt install libnss3-tools")
        );
        let mint = "ID=linuxmint\nID_LIKE=\"ubuntu debian\"\n";
        assert_eq!(
            certutil_package(mint),
            Some("sudo apt install libnss3-tools")
        );
        let rocky = "ID=\"rocky\"\nID_LIKE=\"rhel centos fedora\"\n";
        assert_eq!(certutil_package(rocky), Some("sudo dnf install nss-tools"));
        assert_eq!(certutil_package("ID=arch\n"), Some("sudo pacman -S nss"));
        assert_eq!(certutil_package("ID=nixos\n"), None);
    }

    #[test]
    fn the_timer_looks_once_a_minute_and_only_with_something_to_put_back() {
        let command = [
            "/opt/t w/tracewing",
            "recover",
            "--data-dir",
            "/d",
            "--only-if-gone",
        ]
        .map(str::to_owned);
        let [(service_name, service), (timer_name, timer)] = unit_files(
            &command,
            Path::new("/d/sysproxy.journal"),
            Some("ubuntu:GNOME"),
        );
        assert_eq!(service_name, "tracewing-proxy-recovery.service");
        assert_eq!(timer_name, "tracewing-proxy-recovery.timer");
        assert!(
            service.contains(
                "ExecStart=\"/opt/t w/tracewing\" \"recover\" \"--data-dir\" \"/d\" \"--only-if-gone\""
            ),
            "{service}"
        );
        // Nothing to put back, nothing to start: most minutes of most days.
        assert!(
            service.contains("ConditionPathExists=/d/sysproxy.journal"),
            "{service}"
        );
        // systemd is not told what desktop this is unless the desktop tells
        // it, and without that the command cannot know where the setting is.
        assert!(
            service.contains("Environment=\"XDG_CURRENT_DESKTOP=ubuntu:GNOME\"\n"),
            "{service}"
        );
        let [(_, bare), _] = unit_files(&command, Path::new("/d/sysproxy.journal"), None);
        assert!(!bare.contains("Environment="), "{bare}");

        // The first look as well as the ones after it, or there are none.
        assert!(timer.contains("OnActiveSec=1min"), "{timer}");
        assert!(timer.contains("OnUnitActiveSec=1min"), "{timer}");
        assert!(timer.contains("WantedBy=timers.target"), "{timer}");
    }

    /// systemd reads `%` as the start of a specifier of its own and `\\` as an
    /// escape, so a data folder with either in its name would start the wrong
    /// command, or none.
    #[test]
    fn unit_lines_quote_what_systemd_reserves() {
        assert_eq!(unit_quote("/home/a b/tracewing"), "\"/home/a b/tracewing\"");
        assert_eq!(unit_quote("/home/100%/t"), "\"/home/100%%/t\"");
        assert_eq!(unit_quote(r"/home/a\b"), r#""/home/a\\b""#);
        assert_eq!(unit_quote(r#"/home/a"b"#), r#""/home/a\"b""#);
    }

    #[test]
    fn exec_lines_quote_what_the_spec_reserves() {
        assert_eq!(exec_quote("/a b/c"), "\"/a b/c\"");
        assert_eq!(exec_quote("100%"), "\"100%%\"");
        assert_eq!(exec_quote("a$b"), "\"a\\\\$b\"");
    }
}
