//! The intercepting proxy: TLS through a local certificate authority, and flows
//! captured without buffering and handed to the store.

pub mod apps;
pub mod ca;
pub mod ca_key;
pub mod capture;
pub mod client_certs;
pub mod company_ca;
pub mod datagrams;
pub mod dns;
pub mod extension;
pub mod handover;
pub mod icmp;
pub mod key_log;
pub mod lan;
pub mod name;
pub mod ntlm;
pub mod pause;
pub mod quic;
pub mod refused;
pub mod relay;
pub mod roads;
pub mod rules;
pub mod script;
pub mod sieve;
pub mod sockets;
pub mod socks;
pub mod source;
pub mod stack;
pub mod throttle;
pub mod traffic_rules;
pub mod tun;
#[cfg(target_os = "linux")]
pub mod tun_linux;
pub mod upstream;
#[cfg(windows)]
pub mod wintun;
pub mod wireguard;
pub mod wireguard_key;

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
use source::{By, Found, Port, Source, Stop, Took};
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{net::TcpListener, sync::mpsc};
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
    /// What has arrived by each way in, which the window reads to say that a
    /// way in is on and nothing is coming through it.
    pub arrivals: Arc<source::Arrivals>,
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
    /// Where traffic comes from, besides the proxy port itself: other devices
    /// on the network today, and whatever is added after. See [`source`].
    pub sources: Vec<Box<dyn Source>>,
    /// Sources that arrive **after** the proxy is running, if anything is to
    /// send any.
    ///
    /// A way in that needs a privileged step cannot be built before the
    /// window is on screen -- the step asks the person for a password, and a
    /// password asked for by a program with no window yet is a password
    /// nobody can place. So the window builds that source when somebody asks
    /// for it and hands it over here.
    ///
    /// **It changes when the proxy ends.** Without this, the proxy stops once
    /// its last source has ended, because nothing else can arrive; with it,
    /// something else can, so it runs until it is stopped. That is the right
    /// reading of "a source may arrive later" and it is worth saying out loud,
    /// since the old behaviour was load-bearing for nothing else.
    pub later: Option<tokio::sync::mpsc::Receiver<Box<dyn Source>>>,
    /// Another proxy that connections to servers go through, as a company's.
    /// Changes apply to the next connection.
    pub relay: Arc<relay::Relay>,
    /// Where TLS secrets are written for Wireshark, while they are.
    pub tls_keys: Arc<key_log::TlsKeys>,
    /// The WebSockets open now, for the window to send messages into.
    pub sockets: Arc<sockets::Sockets>,
    /// Certificates to show servers that ask the client for one.
    pub client_certs: Arc<client_certs::ClientCerts>,
    /// The scripts rules run, with what each has logged.
    pub scripts: Arc<script::Scripts>,
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
/// Starts each source as it arrives, until the sender is dropped.
///
/// **One that cannot start is said and stepped over.** By the time anything
/// arrives here the proxy is serving, and a way in that failed is not a
/// reason to take down the ones that are working. Whoever sent it -- the
/// window -- reports it in its own words; this is the log's copy.
async fn start_as_they_arrive(
    mut later: tokio::sync::mpsc::Receiver<Box<dyn Source>>,
    found: Found,
    proxy: SocketAddr,
    stop: Stop,
) {
    while let Some(source) = later.recv().await {
        let name = source.name();
        match found.start(source, proxy, stop.clone()) {
            Ok(()) => tracing::info!("{name} started"),
            Err(e) => tracing::warn!("{e:#}"),
        }
    }
}

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
    // A client that turns the certificate down is seen through `tracing`.
    refused::watch();
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
    let connector = upstream::TimedConnector::new(
        upstream_tls,
        verifier,
        Arc::clone(&opts.relay),
        Arc::clone(&opts.client_certs),
        Arc::clone(&opts.traffic_rules),
    );

    // The proxy port goes first, from the address already resolved above: it
    // is where most clients come in, which is the order `apps` asks in. Taken
    // from here rather than from the source, so that being first depends on
    // nothing about how the sources are ordered, and so that an address that
    // could not be read would have stopped the proxy at `listen_addr` instead
    // of quietly leaving `apps` with no port to look for.
    let mut ways_in: Vec<u16> = vec![listen_addr.port()];
    let ours = Box::new(source::ProxyPort::new(listener)) as Box<dyn Source>;
    let sources: Vec<Box<dyn Source>> = std::iter::once(ours).chain(opts.sources).collect();
    // The proxy port's own answer is swallowed here, having been seeded.
    for port in sources.iter().flat_map(|source| source.own_ports()) {
        if !ways_in.contains(&port) {
            ways_in.push(port);
        }
    }

    // What a source knew of each connection it handed over, for the handler.
    let handed = Arc::new(handover::Handed::default());
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
                .then(|| apps::Apps::spawn(ways_in))
                .transpose()?,
            live: Arc::clone(&opts.live),
            network: opts.network,
            relay: Arc::clone(&opts.relay),
            connector: connector.clone(),
            sockets: Arc::clone(&opts.sockets),
            scripts: Arc::clone(&opts.scripts),
            handed: Arc::clone(&handed),
            arrivals: Arc::clone(&opts.arrivals),
        },
    );
    let stop_limit = opts.stop_limit;
    let pauses = opts.pauses;
    let serving = Serve {
        ca: ca::SharedCa(Arc::new(ca)),
        connector,
        handler: handler.clone(),
        proxy: own_addr,
        handed,
    };
    // Every source, and every listener they opened, stops with this.
    let (stop_tx, stop) = tokio::sync::watch::channel(false);
    let (stopping_tx, stopping) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        shutdown.await;
        // Held, they would keep their connections, and the stop, waiting.
        pauses.release_all();
        let _ = stop_tx.send(true);
        let _ = stopping_tx.send(());
    });

    let (found, took) = Found::channel();
    for source in sources {
        // `Found::start` stamps what the source hands over with its way in:
        // see `Took` for why no source fills that in itself.
        found.start(source, own_addr, stop.clone())?;
    }
    // One more starter for whatever arrives later, which also keeps the
    // channel open: see `ProxyOptions::later`.
    let arriving = opts.later.map(|later| {
        tokio::spawn(start_as_they_arrive(
            later,
            found.again(),
            own_addr,
            stop.clone(),
        ))
    });
    // With ours gone, the channel closes once the last source has ended --
    // unless something may still arrive, which is what `arriving` holds.
    drop(found);

    let reporter = opts
        .stats_every
        .map(|every| tokio::spawn(report(Arc::clone(&stats), every)));
    tokio::select! {
        result = serve(took, serving, stop) => result?,
        _ = async {
            let _ = stopping.await;
            tokio::time::sleep(stop_limit).await;
        } => {}
    }
    if let Some(reporter) = reporter {
        reporter.abort();
    }
    // Nothing more can arrive once the proxy is stopping, and a task waiting
    // on a channel the window still holds would keep the runtime from
    // finishing.
    if let Some(arriving) = arriving {
        arriving.abort();
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

/// What every listener is served with, so that each one's flows go to the
/// same session under the same rules.
#[derive(Clone)]
struct Serve {
    ca: ca::SharedCa,
    connector: upstream::TimedConnector,
    handler: FlowHandler,
    /// Where a taken connection is handed to: the proxy port itself.
    proxy: SocketAddr,
    handed: Arc<handover::Handed>,
}

/// Serves what the sources find, until they have all ended and nothing they
/// left behind is still being served.
///
/// A listener and a single taken connection are the two shapes traffic
/// arrives in; [`source`] says why the first cannot be reduced to the second.
async fn serve(mut took: mpsc::Receiver<Took>, serving: Serve, stop: Stop) -> Result<()> {
    let mut running = tokio::task::JoinSet::new();
    let mut taking = true;
    while taking || !running.is_empty() {
        tokio::select! {
            found = took.recv(), if taking => match found {
                Some(Took::Port(port, by)) => {
                    running.spawn(serve_port(serving.clone(), port, by, stop.clone()));
                }
                Some(Took::Flow(flow, by)) => {
                    let serving = serving.clone();
                    running.spawn(async move {
                        // One taken connection going wrong is not the proxy's end.
                        if let Err(e) =
                            handover::give(flow, by, serving.proxy, &serving.handed).await
                        {
                            tracing::warn!("{e:#}");
                        }
                        Ok(())
                    });
                }
                Some(Took::Quic(attempt, by)) => serving.handler.quic(attempt, by),
                None => taking = false,
            },
            Some(ended) = running.join_next(), if !running.is_empty() => match ended {
                Ok(result) => result?,
                Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic()),
                Err(_) => {}
            },
        }
    }
    Ok(())
}

/// Serves one listener of clients that say where they are going, until it is
/// taken away on its own or everything stops.
async fn serve_port(serving: Serve, port: Port, by: By, stop: Stop) -> Result<()> {
    let Port {
        listener,
        name,
        until,
        essential,
    } = port;
    let built = Proxy::builder()
        .with_listener(listener)
        .with_ca(serving.ca)
        .with_http_connector(serving.connector)
        // Which way in this listener is, so that what arrives on it is
        // counted as its own. The alternative was to guess from the client's
        // address — loopback or not — which would have called a program on
        // this machine that used the LAN address another device.
        .with_http_handler(serving.handler.serving(by))
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = taken_away(until) => {}
                _ = source::stopped(stop) => {}
            }
        })
        .build();
    let served = match built {
        Ok(proxy) => proxy.start().await.map_err(anyhow::Error::new),
        Err(e) => Err(anyhow::Error::new(e)),
    };
    match served {
        Ok(()) => Ok(()),
        // Without the port clients are told to use there is no proxy at all;
        // one address of many that other devices reach is a warning.
        Err(e) if essential => Err(e).with_context(|| format!("serving {name}")),
        Err(e) => {
            tracing::warn!("{name}: {e:#}");
            Ok(())
        }
    }
}

/// This listener alone being taken away: an address other devices reached
/// this machine at, which has gone. A listener with nothing to take it away
/// waits here for ever, and the stop ends it instead.
async fn taken_away(until: Option<tokio::sync::oneshot::Receiver<()>>) {
    match until {
        Some(until) => {
            let _ = until.await;
        }
        None => std::future::pending().await,
    }
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

#[cfg(test)]
mod arriving_tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    /// A source that records that it was started, and can refuse to.
    struct Counts {
        started: Arc<AtomicUsize>,
        refuses: bool,
    }

    impl Source for Counts {
        fn name(&self) -> String {
            "a counted way in".to_owned()
        }

        fn by(&self) -> source::By {
            source::By::Given
        }

        fn start(self: Box<Self>, _proxy: SocketAddr, _found: Found, _stop: Stop) -> Result<()> {
            if self.refuses {
                anyhow::bail!("this one will not start");
            }
            self.started.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    /// A way in handed over after the proxy is running is started, and one
    /// that cannot start does not take the next one with it.
    ///
    /// The second half is the one worth a test: by the time anything arrives
    /// here the proxy is serving other ways in, so a `?` in that loop would
    /// end them all because one that was added late said no.
    #[tokio::test]
    async fn sources_that_arrive_later_are_started_and_a_refusal_is_stepped_over() {
        let started = Arc::new(AtomicUsize::new(0));
        let (found, _took) = Found::channel();
        let (stop_tx, stop) = tokio::sync::watch::channel(false);
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();
        let task = tokio::spawn(start_as_they_arrive(rx, found, addr, stop));

        for refuses in [false, true, false] {
            tx.send(Box::new(Counts {
                started: Arc::clone(&started),
                refuses,
            }) as Box<dyn Source>)
                .await
                .expect("the proxy is listening for more");
        }
        // Dropping the sender ends the loop, which is also how the window
        // closing ends it.
        drop(tx);
        task.await
            .expect("the loop ends when nothing more can arrive");
        assert_eq!(
            started.load(Ordering::SeqCst),
            2,
            "both of the two that could start did, with the refusal between them"
        );
        let _ = stop_tx.send(true);
    }
}
