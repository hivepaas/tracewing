//! The intercepting proxy: TLS through a local certificate authority, and flows
//! captured without buffering and handed to the store.

pub mod ca;
pub mod capture;

use anyhow::{Context, Result};
use ca::LocalCa;
use capture::{Crl, FlowHandler};
use hudsucker::{
    Proxy,
    rustls::{
        ClientConfig, RootCertStore,
        crypto::{CryptoProvider, aws_lc_rs},
        pki_types::{CertificateDer, pem::PemObject},
    },
    tokio_tungstenite::Connector,
};
use hyper_util::client::legacy::connect::HttpConnector;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::net::TcpListener;
use tracewing_store::{
    search,
    session::{self, Shared},
    settings::StorageSettings,
    spool::{Spool, SpoolStats},
    store::{Recorder, Stats},
};

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
    pub listener: TcpListener,
    /// Where the root CA lives.
    pub data_dir: PathBuf,
    /// The session's directory, which must not hold one yet.
    pub session: PathBuf,
    pub settings: StorageSettings,
    /// Extra roots trusted for upstream connections, on top of webpki-roots.
    pub extra_roots: Vec<PathBuf>,
    /// How many finished flows may wait for the writer.
    pub queue: usize,
    pub stats_every: Option<Duration>,
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
    let mut ca = LocalCa::load_or_create(&opts.data_dir, crypto_provider())?;
    let listen_addr = opts.listener.local_addr()?;
    ca.serve_crl_at(listen_addr);
    let crl = Crl {
        host: listen_addr.to_string(),
        der: ca.crl_der()?.into(),
    };
    opts.settings.check()?;
    session::create(&opts.session)?;
    let shared = Arc::new(Shared::default());
    let (spool, spool_thread) = Spool::spawn(
        &session::files_dir(&opts.session),
        opts.settings.spool_budget(),
        opts.settings.body_max(),
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
    socket2::SockRef::from(&opts.listener).set_tcp_nodelay(true)?;
    let mut http = HttpConnector::new();
    http.enforce_http(false);
    http.set_nodelay(true);

    let upstream_tls = upstream_tls(&opts.extra_roots)?;
    let connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(upstream_tls.clone())
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .wrap_connector(http);

    let handler = FlowHandler::new(recorder, opts.settings.log_max(), crl, spool.clone());
    let proxy = Proxy::builder()
        .with_listener(opts.listener)
        .with_ca(ca)
        .with_http_connector(connector)
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler)
        .with_websocket_connector(Connector::Rustls(Arc::new(upstream_tls)))
        .with_graceful_shutdown(shutdown)
        .build()?;

    let reporter = opts
        .stats_every
        .map(|every| tokio::spawn(report(Arc::clone(&stats), every)));
    proxy.start().await?;
    if let Some(reporter) = reporter {
        reporter.abort();
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
        tokio::task::spawn_blocking(move || indexer.finish()).await??;
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

fn upstream_tls(extra_roots: &[PathBuf]) -> Result<ClientConfig> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    for path in extra_roots {
        for cert in CertificateDer::pem_file_iter(path)
            .with_context(|| format!("read {}", path.display()))?
        {
            roots.add(cert.with_context(|| format!("parse {}", path.display()))?)?;
        }
    }
    Ok(
        ClientConfig::builder_with_provider(Arc::new(crypto_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
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
