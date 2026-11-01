use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::{
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::net::TcpListener;
use tracewing_dev::benchkit::{self, LoadOptions};
use tracewing_proxy::{ProxyOptions, ca::LocalCa, crypto_provider, default_data_dir, run_proxy};
use tracewing_store::{search, session, settings::StorageSettings};
use tracewing_system::{self as system, Scope};

#[derive(Parser)]
#[command(about = "The tracewing core without a window")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the intercepting proxy.
    Proxy {
        #[arg(long, default_value = "127.0.0.1:9090")]
        listen: SocketAddr,
        /// Where the root CA is kept [default: ~/.local/share/tracewing].
        #[arg(long)]
        data_dir: Option<PathBuf>,
        /// A new directory for this session [default: one under --data-dir].
        #[arg(long)]
        session: Option<PathBuf>,
        /// How to store it, as TOML. `settings` prints what can be set.
        #[arg(long)]
        settings: Option<PathBuf>,
        /// Extra PEM roots to trust upstream (e.g. the bench origin's CA).
        #[arg(long)]
        extra_root: Vec<PathBuf>,
        #[arg(long, default_value_t = 100_000)]
        queue: usize,
        /// Point the system proxy here while running.
        #[arg(long)]
        system_proxy: bool,
        /// With --system-proxy: no watchdog, to test the recovery at login.
        #[arg(long)]
        no_watchdog: bool,
        /// Stop cleanly after this many seconds.
        #[arg(long)]
        exit_after: Option<u64>,
    },
    /// Manage the root CA in the OS's trust stores.
    Ca {
        #[command(subcommand)]
        action: CaAction,
        #[arg(long, global = true)]
        data_dir: Option<PathBuf>,
    },
    /// Inspect or recover the system proxy setting.
    Sysproxy {
        #[command(subcommand)]
        action: SysproxyAction,
        #[arg(long, global = true)]
        data_dir: Option<PathBuf>,
    },
    /// Wait for stdin to close, then put the system proxy back.
    #[command(hide = true)]
    Watchdog {
        #[arg(long)]
        data_dir: PathBuf,
    },
    /// Print the storage settings: the app's defaults, or those of a file.
    Settings {
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Write one body of a stored flow to a file, unpacked, for viewing.
    Body {
        #[arg(long)]
        session: PathBuf,
        #[arg(long)]
        flow: i64,
        /// The request's body rather than the response's.
        #[arg(long)]
        request: bool,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        settings: Option<PathBuf>,
    },
    /// Find flows in a stored session's index.
    Find {
        #[arg(long)]
        session: PathBuf,
        /// The URL rather than the bodies.
        #[arg(long)]
        url: bool,
        text: String,
    },
    /// Run the HTTPS benchmark origin.
    Origin {
        #[arg(long, default_value = "127.0.0.1:8443")]
        listen: SocketAddr,
        /// Where to write the origin's CA certificate.
        #[arg(long, default_value = "origin-ca.pem")]
        ca_out: PathBuf,
    },
    /// Send load through the proxy, trusting only the proxy's CA.
    Load {
        #[arg(long, default_value = "http://127.0.0.1:9090")]
        proxy: String,
        #[arg(long, default_value = "https://localhost:8443/json?size=1024")]
        url: String,
        /// The proxy's root CA [default: ~/.local/share/tracewing/ca.pem].
        #[arg(long)]
        proxy_ca: Option<PathBuf>,
        #[arg(short = 'n', long, default_value_t = 10_000)]
        requests: u64,
        #[arg(short = 'c', long, default_value_t = 32)]
        concurrency: usize,
        /// Use HTTP/1.1 between client and proxy instead of negotiating HTTP/2.
        #[arg(long)]
        http1: bool,
        /// Report the peak resident memory of this process (the proxy).
        #[arg(long)]
        watch_pid: Option<u32>,
    },
    /// Fill a new session with synthetic flows, for UI benchmarks.
    Seed {
        #[arg(long)]
        session: PathBuf,
        #[arg(long, default_value_t = 1_000_000)]
        rows: u64,
        /// Bodies of about 2 KB of varied words, for search benchmarks.
        #[arg(long)]
        rich: bool,
    },
}

#[derive(Subcommand)]
enum CaAction {
    /// Where the CA is installed, and whether the OS accepts it.
    Status,
    Install {
        #[arg(long, value_enum, default_value = "user")]
        scope: Scope,
    },
    Uninstall {
        #[arg(long, value_enum, default_value = "user")]
        scope: Scope,
    },
}

#[derive(Subcommand)]
enum SysproxyAction {
    /// Show the current setting.
    Status,
    /// Point the system proxy at an address, with journal and recovery, and leave it.
    On {
        #[arg(long, default_value = "127.0.0.1:9090")]
        addr: SocketAddr,
    },
    /// Put back whatever the journal holds.
    Recover,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    let _ = crypto_provider().install_default();

    match Cli::parse().command {
        Command::Proxy {
            listen,
            data_dir,
            session,
            settings,
            extra_root,
            queue,
            system_proxy,
            no_watchdog,
            exit_after,
        } => {
            let data_dir = data_dir.unwrap_or_else(default_data_dir);
            let settings = load_settings(settings.as_deref())?;
            let session = session.unwrap_or_else(|| {
                data_dir
                    .join("sessions")
                    .join(time::OffsetDateTime::now_utc().unix_timestamp().to_string())
            });
            let listener = TcpListener::bind(listen).await?;
            let taken_over = if system_proxy {
                let found = system::recover(&data_dir)?;
                eprintln!("left over from last time: {found:?}");
                Some(system::take_over(
                    &data_dir,
                    listener.local_addr()?,
                    !no_watchdog,
                )?)
            } else {
                None
            };
            eprintln!(
                "proxy on http://{}, CA {}, session in {}",
                listener.local_addr()?,
                data_dir.join("ca.pem").display(),
                session.display()
            );
            let summary = run_proxy(
                ProxyOptions {
                    listener,
                    data_dir,
                    session,
                    settings,
                    extra_roots: extra_root,
                    queue,
                    stats_every: Some(Duration::from_secs(5)),
                },
                async move {
                    match exit_after {
                        Some(secs) => {
                            tokio::select! {
                                _ = shutdown_signal() => {}
                                _ = tokio::time::sleep(Duration::from_secs(secs)) => {}
                            }
                        }
                        None => shutdown_signal().await,
                    }
                },
            )
            .await;
            if let Some(taken_over) = taken_over {
                eprintln!("system proxy: {:?}", taken_over.release()?);
            }
            eprintln!("{:?}", summary?);
        }
        Command::Ca { action, data_dir } => {
            let data_dir = data_dir.unwrap_or_else(default_data_dir);
            let ca = LocalCa::load_or_create(&data_dir, crypto_provider())?;
            match action {
                CaAction::Status => {}
                CaAction::Install { scope } => system::ca_install(ca.cert_path(), scope)?,
                CaAction::Uninstall { scope } => system::ca_uninstall(ca.cert_path(), scope)?,
            }
            println!("certificate: {}", ca.cert_path().display());
            println!("installed: {:?}", system::ca_locations(ca.cert_path())?);
            match system::verify_ca(&ca, crypto_provider()) {
                Ok(()) => println!("the OS accepts it: yes"),
                Err(e) => println!("the OS accepts it: no ({e:#})"),
            }
        }
        Command::Sysproxy { action, data_dir } => {
            let data_dir = data_dir.unwrap_or_else(default_data_dir);
            match action {
                SysproxyAction::Status => {}
                SysproxyAction::On { addr } => {
                    // Left on with no watchdog, as if the app had gone with the power.
                    std::mem::forget(system::take_over(&data_dir, addr, false)?);
                }
                SysproxyAction::Recover => println!("{:?}", system::recover(&data_dir)?),
            }
            println!("{:?}", system::proxy_status()?);
        }
        Command::Watchdog { data_dir } => system::watchdog(&data_dir)?,
        Command::Settings { file } => {
            print!("{}", load_settings(file.as_deref())?.to_toml()?);
        }
        Command::Body {
            session: dir,
            flow,
            request,
            out,
            settings,
        } => {
            let settings = load_settings(settings.as_deref())?;
            let flows = rusqlite::Connection::open_with_flags(
                session::flows_db(&dir),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            let side = if request { "req" } else { "resp" };
            let body = session::stored_body(&flows, flow, side)?;
            let started = std::time::Instant::now();
            let mut file = std::io::BufWriter::new(std::fs::File::create(&out)?);
            let written = session::BodyReader::new(&dir)
                .unpack_into(&body, settings.view_max(), &mut file)
                .context("this body is gone, or cannot be read")?;
            file.flush()?;
            eprintln!(
                "{written} bytes in {:.0} ms → {}",
                started.elapsed().as_secs_f64() * 1000.0,
                out.display()
            );
        }
        Command::Find {
            session: dir,
            url,
            text,
        } => {
            let flows = rusqlite::Connection::open_with_flags(
                session::flows_db(&dir),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            let first_live: i64 =
                flows.query_row("SELECT coalesce(min(id), 1) FROM flows", [], |r| r.get(0))?;
            let table = if url { "urls" } else { "bodies" };
            let started = std::time::Instant::now();
            let found = search::find(&dir, table, &text, first_live)?;
            eprintln!(
                "{} flows in {:.1} ms",
                found.len(),
                started.elapsed().as_secs_f64() * 1000.0
            );
            for id in found.iter().take(20) {
                println!("{id}");
            }
        }
        Command::Origin { listen, ca_out } => {
            let listener = TcpListener::bind(listen).await?;
            eprintln!(
                "origin on https://{}, CA {}",
                listener.local_addr()?,
                ca_out.display()
            );
            benchkit::serve_origin(listener, &ca_out).await?;
        }
        Command::Load {
            proxy,
            url,
            proxy_ca,
            requests,
            concurrency,
            http1,
            watch_pid,
        } => {
            let report = benchkit::run_load(LoadOptions {
                proxy,
                url,
                proxy_ca: proxy_ca.unwrap_or_else(|| default_data_dir().join("ca.pem")),
                requests,
                concurrency,
                http1_only: http1,
                watch_pid,
            })
            .await?;
            report.print();
        }
        Command::Seed {
            session: dir,
            rows,
            rich,
        } => {
            let started = std::time::Instant::now();
            let written = tracewing_dev::seed::seed(&dir, rows, rich)?;
            eprintln!(
                "{written} flows in {:.1}s → {}",
                started.elapsed().as_secs_f64(),
                dir.display()
            );
        }
    }
    Ok(())
}

fn load_settings(path: Option<&Path>) -> Result<StorageSettings> {
    match path {
        Some(path) => StorageSettings::load(path),
        None => Ok(StorageSettings::default()),
    }
}

/// Ctrl-C, or SIGTERM on Unix.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(windows)]
    {
        use tokio::signal::windows::{ctrl_close, ctrl_logoff, ctrl_shutdown};
        let mut close = ctrl_close().expect("install console close handler");
        let mut logoff = ctrl_logoff().expect("install logoff handler");
        let mut off = ctrl_shutdown().expect("install shutdown handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = close.recv() => {}
            _ = logoff.recv() => {}
            _ = off.recv() => {}
        }
    }
}
