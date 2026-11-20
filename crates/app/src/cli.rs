//! `tracewing capture`, `export` and `sessions`: recording with no window, for
//! a server, CI or a terminal, and what it recorded as HAR. The same data
//! folder as the window's, with its CA, settings, rules, relay and client
//! certificates. `tracewing env` takes a terminal through whichever of the two
//! is running. With no command, `tracewing` opens the window.

use crate::{
    devices::Devices,
    engine::{Engine, EngineOptions, PORTS},
    filter::Filter,
};
use anyhow::{Context as _, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use rusqlite::{Connection, OpenFlags};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tracewing_proxy::default_data_dir;
use tracewing_store::{
    har::{self, ExportOptions},
    search, session,
};
use tracewing_system::{Recovered, launch::Shell};

#[derive(Parser)]
#[command(
    name = "tracewing",
    version,
    about = "An HTTP(S) debugging proxy. With no command, it opens its window."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Copy)]
enum ExtensionAction {
    /// Install it, or bring the installed one to this version. macOS asks
    /// for it to be allowed the first time.
    Install,
    /// Whether it is installed and allowed, and whether its proxy runs.
    Status,
    /// Remove its proxy configuration, then the extension itself.
    Uninstall,
}

#[derive(Subcommand)]
enum Command {
    /// Record traffic with no window, until Ctrl+C or --seconds. Point a
    /// client at the address it prints (HTTP_PROXY, HTTPS_PROXY) and have it
    /// trust the CA it prints. Its rules, relay and client certificates are
    /// the window's. The system proxy is left alone.
    Capture {
        /// Listen on this port; the window's first free one if left out.
        #[arg(long)]
        port: Option<u16>,
        /// Stop after this many seconds.
        #[arg(long)]
        seconds: Option<u64>,
        /// Then write what was recorded to this HAR file.
        #[arg(long)]
        har: Option<PathBuf>,
        /// Only the flows this finds, as the window's search box finds them.
        #[arg(long, requires = "har")]
        filter: Option<String>,
        /// Leave cookies and the values of credential headers out of the HAR.
        #[arg(long, requires = "har")]
        without_secrets: bool,
        /// Keep the CA's key in a file in the data folder, not in the system's
        /// keychain: for a server or CI with no keychain.
        #[arg(long)]
        ca_key_in_file: bool,
        /// The data folder [default: the window's].
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Take this app's traffic too, with no proxy settings of its own,
        /// through tracewing's network extension: its signing id, which
        /// `codesign -dv <its program>` says (com.apple.curl,
        /// com.anthropic.claude-code). As often as there are apps. macOS,
        /// from tracewing.app in /Applications; the extension is installed
        /// the first time, and macOS asks for it to be allowed.
        #[arg(long = "app", value_name = "SIGNING_ID")]
        apps: Vec<String>,
        /// Take this program's traffic the same way, by where it is rather
        /// than by a signing id: for what has none of its own, such as what
        /// `go build` or `cargo build` makes. A folder takes every program
        /// under it. As often as there are programs.
        #[arg(long = "program", value_name = "PATH")]
        programs: Vec<PathBuf>,
    },
    /// macOS's network extension, which takes apps' traffic with no proxy
    /// settings of their own (`capture --app`): install it, say how it is,
    /// or remove it.
    Extension {
        #[command(subcommand)]
        action: ExtensionAction,
    },
    /// Print what takes a terminal's programs through tracewing — curl, git,
    /// Python, Go, AI agents — for a terminal it did not open: an editor's,
    /// iTerm's, a script's. `eval "$(tracewing env)"` sets it in sh, bash or
    /// zsh; `tracewing env | Invoke-Expression` in PowerShell. It is what
    /// Open a terminal on Set up sets: the proxy, and tracewing's CA beside
    /// the system's roots for programs that read a file of them. The window,
    /// or `tracewing capture`, must be running.
    Env {
        /// The proxy's port; the running one's if left out.
        #[arg(long)]
        port: Option<u16>,
        /// The shell the lines are for [default: powershell on Windows, sh
        /// elsewhere, which bash and zsh read too].
        #[arg(long, value_enum)]
        shell: Option<ShellName>,
        /// The data folder [default: the window's].
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    /// Write a session's flows to a HAR file.
    Export {
        #[arg(long)]
        har: PathBuf,
        /// A session's id, as `tracewing sessions` lists it; the newest if
        /// left out.
        #[arg(long)]
        session: Option<String>,
        /// Only the flows this finds, as the window's search box finds them.
        #[arg(long)]
        filter: Option<String>,
        /// Leave cookies and the values of credential headers out.
        #[arg(long)]
        without_secrets: bool,
        /// The data folder [default: the window's].
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    /// Put the system proxy back, after tracewing was killed and left it
    /// pointing at a port nothing listens on — which takes the network away
    /// from everything on this computer.
    ///
    /// It restores the settings tracewing wrote down before it changed them,
    /// and does nothing if they are not tracewing's any more.
    Recover {
        /// The data folder [default: the window's].
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// Put it back only if tracewing has gone, and leave a running app
        /// alone. This is what the minute timer behind the setting runs on
        /// Linux, and it is the safe one to put in a script of your own.
        #[arg(long)]
        only_if_gone: bool,
        /// Wait for tracewing to go, however long that takes, and put it back
        /// the moment it has. This is what the agent behind the setting runs
        /// on macOS.
        #[arg(long, conflicts_with = "only_if_gone")]
        wait: bool,
    },
    /// Undo what tracewing did to this computer: take the certificate
    /// authority out of every store that trusts it, and put the system proxy
    /// back.
    ///
    /// What to run before removing tracewing, and what its installer for
    /// Windows runs when it is uninstalled. The sessions recorded and the
    /// settings stay where they are: this says where.
    Uninstall {
        /// The data folder [default: the window's].
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    /// List the sessions recorded, the newest first.
    Sessions {
        /// The data folder [default: the window's].
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

/// Whether `arg` asks for one of these, not for the window.
pub fn is_command(arg: &str) -> bool {
    matches!(
        arg,
        "capture"
            | "env"
            | "extension"
            | "export"
            | "recover"
            | "uninstall"
            | "sessions"
            | "help"
            | "--help"
            | "-h"
            | "--version"
            | "-V"
    )
}

/// A shell `tracewing env` writes for.
#[derive(Clone, Copy, ValueEnum)]
enum ShellName {
    Sh,
    Fish,
    Powershell,
    Cmd,
}

impl From<ShellName> for Shell {
    fn from(name: ShellName) -> Self {
        match name {
            ShellName::Sh => Shell::Sh,
            ShellName::Fish => Shell::Fish,
            ShellName::Powershell => Shell::PowerShell,
            ShellName::Cmd => Shell::Cmd,
        }
    }
}

/// Runs the command `args` ask for, the program's name first.
pub fn run(args: &[String]) -> Result<()> {
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            // Help and the version are said, and are no failure.
            e.print()?;
            if e.use_stderr() {
                std::process::exit(2);
            }
            return Ok(());
        }
    };
    match cli.command {
        Command::Capture {
            port,
            seconds,
            har,
            filter,
            without_secrets,
            ca_key_in_file,
            data_dir,
            apps,
            programs,
        } => {
            let data_dir = data_dir.unwrap_or_else(default_data_dir);
            let har = har.map(|path| HarOut {
                path,
                filter,
                without_secrets,
            });
            let seconds = seconds.map(Duration::from_secs);
            let taken = Taken { apps, programs };
            if taken.is_empty() {
                capture(&data_dir, ca_key_in_file, port, seconds, har, taken)
            } else {
                // macOS answers the extension's calls on the main queue,
                // which this thread serves while another records.
                tracewing_system::extension::serve_main_thread_while(move || {
                    capture(&data_dir, ca_key_in_file, port, seconds, har, taken)
                })
            }
        }
        Command::Extension { action } => extension(action),
        Command::Env {
            port,
            shell,
            data_dir,
        } => env(
            &data_dir.unwrap_or_else(default_data_dir),
            port,
            shell.map_or_else(Shell::native, Shell::from),
        ),
        Command::Export {
            har,
            session,
            filter,
            without_secrets,
            data_dir,
        } => {
            let data_dir = data_dir.unwrap_or_else(default_data_dir);
            let dir = session_dir(&data_dir, session.as_deref())?;
            let out = HarOut {
                path: har,
                filter,
                without_secrets,
            };
            let n = out.write(&data_dir, &dir)?;
            println!("{} written to {}", flows(n), out.path.display());
            Ok(())
        }
        Command::Recover {
            data_dir,
            only_if_gone,
            wait,
        } => {
            let dir = data_dir.unwrap_or_else(default_data_dir);
            let outcome = if wait {
                tracewing_system::recover_when_gone(&dir)?
            } else if only_if_gone {
                tracewing_system::recover_if_gone(&dir)?
            } else {
                tracewing_system::recover(&dir)?
            };
            match outcome {
                Recovered::PutBack => println!("the system proxy is back as it was"),
                Recovered::LeftAlone => {
                    println!("the system proxy is not tracewing's, so it was left alone")
                }
                Recovered::Nothing => {
                    println!("tracewing has not changed the system proxy: nothing to put back")
                }
                Recovered::StillRunning => {
                    println!("tracewing is still running, so the system proxy was left as it is")
                }
            }
            Ok(())
        }
        Command::Uninstall { data_dir } => {
            let dir = data_dir.unwrap_or_else(default_data_dir);
            // The system proxy first: of the two, it is the one that takes
            // the network away while it is wrong. A machine with no desktop
            // to read has none of ours to put back either, so what it says is
            // worth hearing and is no reason to stop.
            match tracewing_system::recover(&dir) {
                Ok(Recovered::PutBack) => println!("the system proxy is back as it was"),
                Ok(_) => {}
                Err(e) => eprintln!("the system proxy was left as it is: {e:#}"),
            }
            let ca = dir.join("ca.pem");
            let mut failed = None;
            if ca.exists() {
                // Which stores hold it, said before they are emptied: a line
                // that names them is the only way somebody can check this
                // afterwards, and an authority nobody trusts is not news.
                match tracewing_system::ca_locations(&ca) {
                    Ok(places) if places.is_empty() => {
                        println!("no store on this computer trusts tracewing's authority")
                    }
                    Ok(places) => {
                        let names: Vec<_> = places.iter().map(|place| place.name.clone()).collect();
                        match tracewing_system::ca_remove(&ca) {
                            Ok(()) => println!(
                                "tracewing's authority is out of {}: this computer no longer trusts it",
                                names.join(", ")
                            ),
                            Err(e) => failed = Some(e),
                        }
                    }
                    Err(e) => failed = Some(e),
                }
                // Said, not undone: a store of tracewing's by name may hold
                // the CA of a data directory that is still in use, and
                // nothing here can tell. Taking it out on the strength of
                // the name alone was tried, and `uninstall --data-dir <a
                // throwaway folder>` took this computer's shared anchor out
                // from under the real install.
                if failed.is_none() && tracewing_system::an_older_ca_is_trusted() {
                    println!(
                        "an authority of tracewing's from another data directory is still \
                         trusted here: uninstall from that data directory to take it out"
                    );
                }
            } else {
                println!(
                    "no certificate authority of tracewing's in {}",
                    dir.display()
                );
            }
            println!(
                "what tracewing recorded is still in {}: remove the folder to be rid of it",
                dir.display()
            );
            // Said last, where somebody running this by hand will see it, and
            // after everything that could be done has been.
            match failed {
                Some(e) => Err(e.context("take the authority out of this computer's stores")),
                None => Ok(()),
            }
        }
        Command::Sessions { data_dir } => {
            sessions(&data_dir.unwrap_or_else(default_data_dir));
            Ok(())
        }
    }
}

/// Where to write a HAR, and of what.
struct HarOut {
    path: PathBuf,
    filter: Option<String>,
    without_secrets: bool,
}

impl HarOut {
    /// Writes the session's flows that the filter finds. Returns how many.
    fn write(&self, data_dir: &Path, session: &Path) -> Result<u64> {
        let ids = match self.filter.as_deref().filter(|f| !f.trim().is_empty()) {
            Some(filter) => {
                let path = session::flows_db(session);
                let conn = Connection::open_with_flags(
                    &path,
                    OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
                )
                .with_context(|| format!("open {}", path.display()))?;
                let devices = Devices::load(&data_dir.join("devices.toml"));
                let asked = Filter::parse_with(filter, &devices);
                let mut ids = asked.find(&conn, session)?;
                ids.sort_unstable();
                // A word in the filter is answered by the search index, and
                // a thread of its own writes that after the flow it covers.
                // An export is the worst place for that to pass unsaid: the
                // file looks complete and is missing the newest requests.
                if asked.text.is_some() {
                    let newest: i64 =
                        conn.query_row("SELECT coalesce(max(id), 0) FROM flows", [], |r| r.get(0))?;
                    match search::reach(session, newest)? {
                        search::Reach::Whole => {}
                        search::Reach::Behind(through) => eprintln!(
                            "tracewing: the search index has reached flow {through} of {newest}, \
                             so this export may be missing the newest requests a word matches. \
                             Run it again in a moment to include them."
                        ),
                        search::Reach::NoBodies => eprintln!(
                            "tracewing: this session has no body index, so a word is matched \
                             against URLs only and this export leaves out flows that have it in \
                             a body. Running it again will not change that; `index_bodies` in \
                             settings is what turns it on."
                        ),
                    }
                }
                Some(ids)
            }
            None => None,
        };
        let file = std::fs::File::create(&self.path)
            .with_context(|| format!("create {}", self.path.display()))?;
        let mut out = std::io::BufWriter::new(file);
        let options = ExportOptions {
            without_secrets: self.without_secrets,
        };
        har::export(session, ids.as_deref(), options, &mut out)
    }
}

fn capture(
    data_dir: &Path,
    ca_key_in_file: bool,
    port: Option<u16>,
    seconds: Option<Duration>,
    har: Option<HarOut>,
    taken: Taken,
) -> Result<()> {
    std::fs::create_dir_all(data_dir).with_context(|| format!("create {}", data_dir.display()))?;
    let ports = port.map_or(PORTS, |port| port..port.saturating_add(1));
    let (mut options, problem) = EngineOptions::from_data_dir(data_dir, ports);
    if ca_key_in_file {
        options.key_store = tracewing_proxy::ca_key::KeyStore::File;
    }
    if let Some(problem) = problem {
        eprintln!("tracewing: left out, as it cannot be read: {problem}");
    }
    let programs = taken
        .programs
        .iter()
        .map(|path| tracewing_system::extension::program_rule(path))
        .collect::<Result<Vec<_>>>()?;
    let wanted = !taken.is_empty();
    let engine = Engine::start_with_extension(options, wanted.then(|| taken.apps.clone()))?;
    // The CA is made, or read, as the proxy starts.
    let ca = data_dir.join("ca.pem");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ca.exists() {
        if let Some(failure) = engine
            .failure
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            bail!("the proxy did not start: {failure}");
        }
        if Instant::now() > deadline {
            bail!("the proxy did not start in 30 s");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let session = engine.session.clone();
    eprintln!("tracewing is recording on http://{}", engine.addr);
    eprintln!("  its CA: {} (have the client trust it)", ca.display());
    eprintln!("  session: {}", session.display());
    eprintln!(
        "  try: curl --proxy http://{} --cacert {} https://example.com",
        engine.addr,
        ca.display()
    );
    let extension = match (&engine.extension, &engine.extension_problem) {
        (Some(config), _) => Some(ExtensionRunning::start(
            &tracewing_system::extension::Config {
                programs,
                ..config.clone()
            },
        )?),
        (None, Some(problem)) => bail!("no port for the network extension: {problem}"),
        (None, None) => None,
    };
    eprintln!(
        "{}",
        match seconds {
            Some(s) => format!("Stops in {} s, or at Ctrl+C.", s.as_secs()),
            None => "Ctrl+C stops it.".to_owned(),
        }
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        tokio::select! {
            // The only signal handler tracewing installs, and it belongs to
            // this command. The watchdog deliberately has none: a handler
            // there would let a signal cut its read of stdin short, which it
            // would take for the app going. If you give the watchdog one, read
            // the loop in `tracewing_system::watchdog` first.
            _ = tokio::signal::ctrl_c() => {}
            _ = async {
                match seconds {
                    Some(s) => tokio::time::sleep(s).await,
                    None => std::future::pending().await,
                }
            } => {}
        }
    });
    // The apps' flows go out as they would have before tracewing stops.
    drop(extension);
    // Every flow written before anything reads them.
    drop(engine);
    let recorded = session::list(&data_dir.join("sessions"))
        .into_iter()
        .find(|s| s.dir == session)
        .map_or(0, |s| s.flows);
    eprintln!("Stopped: {} recorded.", flows(recorded));
    if let Some(har) = har {
        let n = har.write(data_dir, &session)?;
        eprintln!("{} written to {}", flows(n), har.path.display());
    }
    Ok(())
}

/// macOS's network extension, running for `capture --app`: stopped when this
/// goes, however the capture ends.
struct ExtensionRunning;

/// What `capture` has the extension take: apps by signing id, programs by
/// where they are.
struct Taken {
    apps: Vec<String>,
    programs: Vec<PathBuf>,
}

impl Taken {
    fn is_empty(&self) -> bool {
        self.apps.is_empty() && self.programs.is_empty()
    }
}

impl ExtensionRunning {
    fn start(config: &tracewing_system::extension::Config) -> Result<Self> {
        use tracewing_system::extension;
        extension::install(&|| {
            eprintln!(
                "  macOS waits for tracewing's network extension to be allowed: {}",
                extension::WHERE_TO_ALLOW
            );
        })?;
        extension::start(config)?;
        eprintln!(
            "  and, with no proxy settings of their own: {}",
            config
                .apps
                .iter()
                .chain(&config.programs)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(Self)
    }
}

impl Drop for ExtensionRunning {
    fn drop(&mut self) {
        if let Err(e) = tracewing_system::extension::stop() {
            eprintln!("tracewing: the network extension's proxy did not stop: {e:#}");
        }
    }
}

fn extension(action: ExtensionAction) -> Result<()> {
    use tracewing_system::extension::{self, Installed, Running};
    let waiting = || {
        eprintln!(
            "macOS waits for this to be allowed: {}",
            extension::WHERE_TO_ALLOW
        )
    };
    extension::serve_main_thread_while(move || match action {
        ExtensionAction::Install => {
            extension::install(&waiting)?;
            println!("tracewing's network extension is installed and allowed");
            Ok(())
        }
        ExtensionAction::Uninstall => {
            extension::uninstall(&waiting)?;
            println!("tracewing's network extension is removed");
            Ok(())
        }
        ExtensionAction::Status => {
            let status = extension::status()?;
            println!(
                "extension: {}",
                match status.installed {
                    Installed::Yes => "installed and allowed".to_owned(),
                    Installed::WaitingForApproval => format!(
                        "installed, waiting to be allowed in {}",
                        extension::WHERE_TO_ALLOW
                    ),
                    Installed::No => "not installed".to_owned(),
                    Installed::Unknown => "not known: only tracewing.app may ask macOS".to_owned(),
                }
            );
            println!(
                "proxy: {}",
                match status.running {
                    Running::None => "no configuration saved",
                    Running::Off => "off",
                    Running::Starting => "starting",
                    Running::On => "on",
                    Running::Stopping => "stopping",
                }
            );
            Ok(())
        }
    })
}

/// The lines that take a shell through the proxy, on stdout for the shell to
/// run; what is worth knowing about them on stderr, where `eval` leaves it.
fn env(data_dir: &Path, port: Option<u16>, shell: Shell) -> Result<()> {
    use std::io::IsTerminal as _;
    let addr: std::net::SocketAddr = match port {
        Some(port) => ([127, 0, 0, 1], port).into(),
        None => std::fs::read_to_string(data_dir.join(crate::engine::PROXY_ADDRESS_FILE))
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .with_context(|| {
                format!(
                    "tracewing is not running on {}: open it, or start `tracewing capture`, \
                     then run this again",
                    data_dir.display()
                )
            })?,
    };
    // A file a killed run left says where nothing listens any more.
    if std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(1)).is_err() {
        bail!("nothing listens at {addr}: open tracewing, or start `tracewing capture`");
    }
    let ca = data_dir.join("ca.pem");
    if !ca.is_file() {
        bail!("no CA in {} yet: open tracewing once", data_dir.display());
    }
    let vars = tracewing_system::launch::terminal_vars(addr, &ca, data_dir)?;
    print!("{}", shell.set(&vars));
    if let Some(version) = tracewing_system::launch::node_version_here()
        .filter(|v| tracewing_system::launch::node_reads_proxy_env(v) == Some(false))
    {
        eprintln!(
            "tracewing: Node.js {version} here goes straight out, past tracewing: \
             Node reads HTTPS_PROXY from 22.21 and 24.5 on"
        );
    }
    if std::io::stdout().is_terminal() {
        eprintln!(
            "tracewing: {}",
            match shell {
                Shell::Sh => "eval \"$(tracewing env)\" sets these",
                Shell::Fish => "tracewing env --shell fish | source sets these",
                Shell::PowerShell => "tracewing env | Invoke-Expression sets these",
                Shell::Cmd => "run these lines in the console to set them",
            }
        );
    }
    Ok(())
}

/// `1 flow`, `2 flows`. The terminal speaks English, as the commands do.
fn flows(n: u64) -> String {
    if n == 1 {
        "1 flow".to_owned()
    } else {
        format!("{n} flows")
    }
}

/// The session of that id, or the newest.
fn session_dir(data_dir: &Path, id: Option<&str>) -> Result<PathBuf> {
    let all = session::list(&data_dir.join("sessions"));
    let chosen = match id {
        Some(id) => all
            .into_iter()
            .find(|s| s.dir.file_name().is_some_and(|n| n == id))
            .with_context(|| {
                format!("there is no session {id}: `tracewing sessions` lists them")
            })?,
        None => all
            .into_iter()
            .next()
            .context("nothing has been recorded yet")?,
    };
    if let Some(trouble) = &chosen.trouble {
        bail!(
            "the session {} cannot be read: {trouble}",
            chosen.dir.display()
        );
    }
    Ok(chosen.dir)
}

fn sessions(data_dir: &Path) {
    let all = session::list(&data_dir.join("sessions"));
    if all.is_empty() {
        println!("No sessions yet in {}", data_dir.display());
        return;
    }
    let time = |us: Option<i64>| {
        us.and_then(chrono::DateTime::from_timestamp_micros)
            .map(|t| {
                t.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "-".to_owned())
    };
    println!(
        "{:<28} {:>8}  {:<19}  {:<19}  name",
        "id", "flows", "first", "last"
    );
    for s in all {
        let id = s
            .dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = match (&s.name, s.readable) {
            (_, false) => "(cannot be read by this version)".to_owned(),
            (Some(name), true) => name.clone(),
            (None, true) => String::new(),
        };
        println!(
            "{id:<28} {:>8}  {:<19}  {:<19}  {name}",
            s.flows,
            time(s.first_us),
            time(s.last_us)
        );
    }
}
