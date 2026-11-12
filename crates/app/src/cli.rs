//! `tracewing capture`, `export` and `sessions`: recording with no window, for
//! a server, CI or a terminal, and what it recorded as HAR. The same data
//! folder as the window's, with its CA, settings, rules, relay and client
//! certificates. With no command, `tracewing` opens the window.

use crate::{
    devices::Devices,
    engine::{Engine, EngineOptions, PORTS},
    filter::Filter,
};
use anyhow::{Context as _, Result, bail};
use clap::{Parser, Subcommand};
use rusqlite::{Connection, OpenFlags};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tracewing_proxy::default_data_dir;
use tracewing_store::{
    har::{self, ExportOptions},
    session,
};

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
        "capture" | "export" | "sessions" | "help" | "--help" | "-h" | "--version" | "-V"
    )
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
        } => capture(
            &data_dir.unwrap_or_else(default_data_dir),
            ca_key_in_file,
            port,
            seconds.map(Duration::from_secs),
            har.map(|path| HarOut {
                path,
                filter,
                without_secrets,
            }),
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
                let mut ids = Filter::parse_with(filter, &devices).find(&conn, session)?;
                ids.sort_unstable();
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
    let engine = Engine::start(options)?;
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
            _ = tokio::signal::ctrl_c() => {}
            _ = async {
                match seconds {
                    Some(s) => tokio::time::sleep(s).await,
                    None => std::future::pending().await,
                }
            } => {}
        }
    });
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
    if !chosen.readable {
        bail!(
            "this version cannot read the session {}",
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
