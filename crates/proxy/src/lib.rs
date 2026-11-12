//! The intercepting proxy: TLS through a local certificate authority, and flows
//! captured without buffering and handed to the store.

pub mod apps;
pub mod ca;
pub mod ca_key;
pub mod capture;
pub mod key_log;
pub mod lan;
pub mod pause;
pub mod relay;
pub mod rules;
pub mod sockets;
pub mod throttle;
pub mod traffic_rules;
pub mod upstream;

/// Moved to the store, which needs it to read HAR files too.
pub use tracewing_store::graphql;

use anyhow::{Context, Result};
use ca::LocalCa;
use capture::{Check, Controls, Crl, FlowHandler};
use hudsucker::{
    Proxy,
    rustls::{
        ClientConfig, RootCertStore,
        client::{WebPkiServerVerifier, danger::ServerCertVerifier},
        crypto::{CryptoProvider, aws_lc_rs},
        pki_types::{CertificateDer, pem::PemObject},
    },
};
use rules::HostRules;
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::net::TcpListener;
use tracewing_store::{
    search,
    session::{self, Shared},
    settings::{Live, StorageSettings},
    spool::{Spool, SpoolStats},
    store::{Recorder, Stats},
};
use traffic_rules::TrafficRules;

/// Where the root CA and other per-machine state live by default.
pub fn default_data_dir() -> PathBuf {
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    base.unwrap_or_else(|| PathBuf::from(".")).join("tracewing")
}

pub struct ProxyOptions {
    pub listener: Option<TcpListener>,
    /// The same, bound by a thread with no runtime. Used if `listener` is `None`.
    pub std_listener: Option<std::net::TcpListener>,
    /// Where the root CA lives.
    pub data_dir: PathBuf,
    /// Where its key lives: the app's in the keychain, tests' and tools' in a file.
    pub key_store: ca_key::KeyStore,
    /// The session's directory, which must not hold one yet.
    pub session: PathBuf,
    pub settings: StorageSettings,
    /// Extra roots trusted for upstream connections, on top of webpki-roots.
    pub extra_roots: Vec<PathBuf>,
    /// How many finished flows may wait for the writer.
    pub queue: usize,
    pub stats_every: Option<Duration>,
    /// What the session's threads share, for whoever wants to watch them.
    pub shared: Arc<Shared>,
    /// Whether flows are being recorded; traffic passes either way.
    pub recording: Arc<AtomicBool>,
    /// What not to decrypt. Changes apply to the next connection.
    pub rules: Arc<RwLock<HostRules>>,
    /// What to change on traffic's way. Changes apply to the next request.
    pub traffic_rules: Arc<RwLock<TrafficRules>>,
    /// A slow network on purpose. Changes apply to the next request.
    pub network: Arc<throttle::Network>,
    /// Who has reached the test page.
    pub check: Arc<Check>,
    /// What breakpoints hold, for the window to let go.
    pub pauses: Arc<pause::Pauses>,
    /// Set once the CA is loaded: see `LocalCa::leaf_spki_sha256`.
    pub leaf_spki: Arc<OnceLock<String>>,
    /// Stop without waiting for the search index to catch up.
    pub leave_index: bool,
    /// Whether to look for the program on this machine behind each connection.
    pub find_apps: bool,
    /// How long a stop waits for connections to end. A stream or a WebSocket that
    /// is still open would otherwise hold it up for as long as it stays open.
    pub stop_limit: Duration,
    /// The settings that apply as they change. They start as `settings` has them.
    pub live: Arc<Live>,
    /// Whether other devices on the network may use the proxy, switched while
    /// it runs. `None` keeps them out.
    pub other_devices: Option<Arc<lan::OtherDevices>>,
    /// Another proxy that connections to servers go through, as a company's.
    /// Changes apply to the next connection.
    pub relay: Arc<relay::Relay>,
    /// Where TLS secrets are written for Wireshark, while they are.
    pub tls_keys: Arc<key_log::TlsKeys>,
    /// The WebSockets open now, for the window to send messages into.
    pub sockets: Arc<sockets::Sockets>,
}

#[derive(Debug)]
pub struct ProxySummary {
    pub submitted: u64,
    pub written: u64,
    pub dropped: u64,
    pub shed: u64,
    pub queue_peak: u64,
    pub shed_low_disk: u64,
    pub rows_lost: u64,
    pub log_errors: u64,
    pub trimmed: u64,
    pub bytes_in: u64,
    pub bytes_logged: u64,
    pub peak_session: u64,
    pub spooled_files: u64,
    pub spooled_bytes: u64,
    pub spool_lost: u64,
    pub spool_errors: u64,
    pub spool_peak_queued: u64,
    /// Flows indexed by the time the proxy had stopped, and how long the rest
    /// took after the traffic ended.
    pub indexed: u64,
    pub index_lag: Duration,
    pub index_errors: u64,
}

pub fn crypto_provider() -> CryptoProvider {
    aws_lc_rs::default_provider()
}

/// Runs the proxy until `shutdown` resolves, then waits for every flow to be
/// written.
pub async fn run_proxy(
    opts: ProxyOptions,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<ProxySummary> {
    let listener = match (opts.listener, opts.std_listener) {
        (Some(listener), _) => listener,
        (None, Some(listener)) => TcpListener::from_std(listener)?,
        (None, None) => anyhow::bail!("nothing to listen on"),
    };
    let mut ca = LocalCa::load_or_create(&opts.data_dir, crypto_provider(), opts.key_store)?;
    let listen_addr = listener.local_addr()?;
    // Listening everywhere, this machine still reaches it on its own address.
    let own_addr = if listen_addr.ip().is_unspecified() {
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), listen_addr.port())
    } else {
        listen_addr
    };
    ca.serve_crl_at(own_addr);
    ca.log_keys_to(Arc::clone(&opts.tls_keys) as Arc<dyn hudsucker::rustls::KeyLog>);
    let _ = opts.leaf_spki.set(ca.leaf_spki_sha256());
    let crl = Crl {
        host: own_addr.to_string(),
        der: ca.crl_der()?.into(),
        ca_der: ca.cert_der()?.into(),
    };
    opts.settings.check()?;
    opts.live.apply(&opts.settings);
    session::create(&opts.session)?;
    let shared = opts.shared;
    let (spool, spool_thread) = Spool::spawn(
        &session::files_dir(&opts.session),
        opts.settings.spool_budget(),
        Arc::clone(&opts.live),
        Arc::clone(&shared),
    )?;
    let spool_stats = spool.stats();
    let (recorder, writer) = Recorder::spawn(
        &opts.session,
        &opts.settings,
        Arc::clone(&shared),
        Some(spool.clone()),
        opts.queue,
    )?;
    let stats = recorder.stats();
    let indexer = (opts.settings.index_urls || opts.settings.index_bodies)
        .then(|| search::Indexer::spawn(&opts.session, &opts.settings, Arc::clone(&shared)))
        .transpose()?;

    // Without TCP_NODELAY, small writes wait for delayed ACKs and the tail
    // latency jumps to ~40 ms. Linux copies the flag to accepted sockets.
    socket2::SockRef::from(&listener).set_tcp_nodelay(true)?;
    let (mut upstream_tls, verifier) = upstream_tls(&opts.extra_roots, opts.settings.os_roots)?;
    upstream_tls.key_log = Arc::clone(&opts.tls_keys) as Arc<dyn hudsucker::rustls::KeyLog>;
    let connector = upstream::TimedConnector::new(upstream_tls, verifier, Arc::clone(&opts.relay));

    let handler = FlowHandler::new(
        recorder,
        opts.settings.log_max(),
        crl,
        spool.clone(),
        Controls {
            recording: opts.recording,
            rules: opts.rules,
            traffic_rules: opts.traffic_rules,
            check: opts.check,
            pauses: Arc::clone(&opts.pauses),
            apps: opts
                .find_apps
                .then(|| apps::Apps::spawn(listen_addr.port()))
                .transpose()?,
            live: Arc::clone(&opts.live),
            network: opts.network,
            relay: Arc::clone(&opts.relay),
            connector: connector.clone(),
            sockets: Arc::clone(&opts.sockets),
        },
    );
    let stop_limit = opts.stop_limit;
    let pauses = opts.pauses;
    let (stopping_tx, stopping) = tokio::sync::oneshot::channel();
    let ca = ca::SharedCa(Arc::new(ca));
    // The listeners for other devices stop with this one.
    let (lan_stop_tx, lan_stop) = tokio::sync::watch::channel(false);
    let lan = opts.other_devices.map(|devices| {
        let serve = LanServe {
            ca: ca.clone(),
            connector: connector.clone(),
            handler: handler.clone(),
        };
        tokio::spawn(serve_other_devices(
            serve,
            devices,
            own_addr.port(),
            lan_stop,
        ))
    });
    let proxy = Proxy::builder()
        .with_listener(listener)
        .with_ca(ca)
        .with_http_connector(connector)
        .with_http_handler(handler.clone())
        .with_graceful_shutdown(async move {
            shutdown.await;
            let _ = lan_stop_tx.send(true);
            // Held, they would keep their connections, and the stop, waiting.
            pauses.release_all();
            let _ = stopping_tx.send(());
        })
        .build()?;

    let reporter = opts
        .stats_every
        .map(|every| tokio::spawn(report(Arc::clone(&stats), every)));
    tokio::select! {
        result = proxy.start() => result?,
        _ = async {
            let _ = stopping.await;
            tokio::time::sleep(stop_limit).await;
        } => {}
    }
    if let Some(reporter) = reporter {
        reporter.abort();
    }
    if let Some(lan) = lan {
        let _ = tokio::time::timeout(stop_limit, lan).await;
    }

    // The index goes first, with the writer still there to keep the session to its
    // limit: a flood leaves the index far behind, and it counts towards that limit.
    let settled = Arc::clone(&stats);
    tokio::task::spawn_blocking(move || {
        settled.wait_until_written();
        spool.barrier();
    })
    .await?;
    let mut indexed = 0;
    let mut index_errors = 0;
    let mut index_lag = Duration::ZERO;
    if let Some(indexer) = indexer {
        let progress = indexer.stats();
        let started = std::time::Instant::now();
        let leave_index = opts.leave_index;
        tokio::task::spawn_blocking(move || {
            if leave_index {
                indexer.leave()
            } else {
                indexer.finish()
            }
        })
        .await??;
        index_lag = started.elapsed();
        indexed = progress.indexed.load(Ordering::Relaxed);
        index_errors = progress.errors.load(Ordering::Relaxed);
    }
    tokio::task::spawn_blocking(move || writer.close()).await??;
    tokio::task::spawn_blocking(move || spool_thread.close()).await??;

    let spooled = |read: fn(&SpoolStats) -> &AtomicU64| read(&spool_stats).load(Ordering::Relaxed);
    Ok(ProxySummary {
        submitted: stats.submitted.load(Ordering::Relaxed),
        written: stats.written.load(Ordering::Relaxed),
        dropped: stats.dropped.load(Ordering::Relaxed),
        shed: stats.shed.load(Ordering::Relaxed),
        queue_peak: stats.peak_queued.load(Ordering::Relaxed),
        shed_low_disk: stats.shed_low_disk.load(Ordering::Relaxed),
        rows_lost: stats.rows_lost.load(Ordering::Relaxed),
        log_errors: stats.log_errors.load(Ordering::Relaxed),
        trimmed: stats.trimmed.load(Ordering::Relaxed),
        bytes_in: stats.bytes_in.load(Ordering::Relaxed),
        bytes_logged: stats.bytes_logged.load(Ordering::Relaxed),
        peak_session: stats.peak_session.load(Ordering::Relaxed),
        spooled_files: spooled(|s| &s.files),
        spooled_bytes: spooled(|s| &s.bytes),
        spool_lost: spooled(|s| &s.lost),
        spool_errors: spooled(|s| &s.write_errors),
        spool_peak_queued: spooled(|s| &s.peak_queued),
        indexed,
        index_lag,
        index_errors,
    })
}

/// What a listener for other devices is served with: the same as the
/// loopback's, so its flows go to the same session under the same rules.
struct LanServe {
    ca: ca::SharedCa,
    connector: upstream::TimedConnector,
    handler: FlowHandler,
}

/// How often the addresses this machine has on its networks are looked at
/// while other devices may use the proxy.
const LAN_LOOK_EVERY: Duration = Duration::from_secs(3);

/// Listens for other devices on each of this machine's addresses, on the
/// loopback's port, while they may: an address that comes gets a listener,
/// and one that goes, or all of them when the switch goes off, has its
/// listener stopped. Connections it took are left to end on their own, as
/// a phone's may be in the middle of something.
async fn serve_other_devices(
    serve: LanServe,
    devices: Arc<lan::OtherDevices>,
    port: u16,
    mut stop: tokio::sync::watch::Receiver<bool>,
) {
    let mut running: std::collections::BTreeMap<
        std::net::Ipv4Addr,
        tokio::sync::oneshot::Sender<()>,
    > = std::collections::BTreeMap::new();
    loop {
        let wanted: Vec<std::net::Ipv4Addr> = if devices.is_on() {
            lan::addresses()
        } else {
            Vec::new()
        };
        running.retain(|ip, _| wanted.contains(ip));
        let mut problem = None;
        for ip in wanted {
            if running.contains_key(&ip) {
                continue;
            }
            match TcpListener::bind((ip, port)).await {
                Ok(listener) => {
                    let _ = socket2::SockRef::from(&listener).set_tcp_nodelay(true);
                    let (stop_one, stopped) = tokio::sync::oneshot::channel::<()>();
                    let built = Proxy::builder()
                        .with_listener(listener)
                        .with_ca(serve.ca.clone())
                        .with_http_connector(serve.connector.clone())
                        .with_http_handler(serve.handler.clone())
                        .with_graceful_shutdown(async move {
                            let _ = stopped.await;
                        })
                        .build();
                    match built {
                        Ok(proxy) => {
                            tokio::spawn(async move {
                                if let Err(e) = proxy.start().await {
                                    tracing::warn!("listening on {ip}: {e:#}");
                                }
                            });
                            running.insert(ip, stop_one);
                        }
                        Err(e) => problem = Some(format!("{ip}:{port}: {e:#}")),
                    }
                }
                Err(e) => problem = Some(format!("{ip}:{port}: {e}")),
            }
        }
        devices.listen_at(
            running
                .keys()
                .map(|ip| SocketAddr::new((*ip).into(), port))
                .collect(),
            problem,
        );
        tokio::select! {
            _ = tokio::time::sleep(LAN_LOOK_EVERY), if devices.is_on() => {}
            _ = devices.changed().notified() => {}
            _ = stop.changed() => break,
        }
    }
    // Dropping each sender stops its listener.
    running.clear();
    devices.listen_at(Vec::new(), None);
}

/// How the proxy checks servers, and the checker on its own, which the
/// connector wraps to keep the certificates it is shown.
/// The authorities whose servers the proxy trusts: the public ones; with
/// `os_roots`, those this computer trusts too, as a company's own or that of a
/// proxy in front of this one; and those in `extra_roots`.
pub fn upstream_roots(extra_roots: &[PathBuf], os_roots: bool) -> Result<RootCertStore> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    if os_roots {
        // One unreadable store or certificate leaves the rest.
        let found = rustls_native_certs::load_native_certs();
        for error in &found.errors {
            tracing::warn!("the system's certificates: {error}");
        }
        let (added, ignored) = roots.add_parsable_certificates(found.certs);
        tracing::debug!(added, ignored, "the system's certificates");
    }
    for path in extra_roots {
        for cert in CertificateDer::pem_file_iter(path)
            .with_context(|| format!("read {}", path.display()))?
        {
            roots.add(cert.with_context(|| format!("parse {}", path.display()))?)?;
        }
    }
    Ok(roots)
}

fn upstream_tls(
    extra_roots: &[PathBuf],
    os_roots: bool,
) -> Result<(ClientConfig, Arc<dyn ServerCertVerifier>)> {
    let roots = upstream_roots(extra_roots, os_roots)?;
    let provider = Arc::new(crypto_provider());
    let verifier: Arc<dyn ServerCertVerifier> =
        WebPkiServerVerifier::builder_with_provider(Arc::new(roots), Arc::clone(&provider))
            .build()?;
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::clone(&verifier))
        .with_no_client_auth();
    Ok((config, verifier))
}

async fn report(stats: Arc<Stats>, every: Duration) {
    let mut ticker = tokio::time::interval(every);
    ticker.tick().await;
    let mut last = 0;
    loop {
        ticker.tick().await;
        let written = stats.written.load(Ordering::Relaxed);
        eprintln!(
            "flows: {written} written ({:.0}/s), {} dropped, rss {}",
            (written - last) as f64 / every.as_secs_f64(),
            stats.dropped.load(Ordering::Relaxed),
            rss_mib().map_or("n/a".to_owned(), |m| format!("{m:.0} MiB")),
        );
        last = written;
    }
}

/// Resident memory of this process, Linux only.
pub fn rss_mib() -> Option<f64> {
    rss_mib_of("self")
}

pub fn rss_mib_of(pid: &str) -> Option<f64> {
    status_mib(pid, "VmRSS:")
}

/// The kernel's own record of the highest the memory has been, which sampling
/// `VmRSS` can miss between two looks.
pub fn peak_rss_mib_of(pid: &str) -> Option<f64> {
    status_mib(pid, "VmHWM:")
}

fn status_mib(pid: &str, field: &str) -> Option<f64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let kib: f64 = status
        .lines()
        .find_map(|l| l.strip_prefix(field))?
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .ok()?;
    Some(kib / 1024.0)
}
