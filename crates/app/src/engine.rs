//! The proxy, running beside the window on threads of its own.

use anyhow::{Context as _, Result};
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};
use tokio::sync::oneshot;
use tracewing_proxy::{
    ProxyOptions,
    ca_key::KeyStore,
    capture::Check,
    client_certs::{ClientCertSettings, ClientCerts},
    key_log::TlsKeys,
    lan::OtherDevices,
    pause::Pauses,
    relay::{Relay, RelaySettings},
    rules::HostRules,
    run_proxy,
    sockets::Sockets,
    throttle::{Conditions, Network},
    traffic_rules::TrafficRules,
};
use tracewing_store::{
    session::{self, Shared},
    settings::{Live, StorageSettings},
};

/// Where the proxy tries to listen, in this order.
pub const PORTS: std::ops::Range<u16> = 9090..9100;

pub struct EngineOptions {
    pub data_dir: PathBuf,
    /// Where the CA's key is kept: the app's in the keychain, tests' in a file.
    pub key_store: KeyStore,
    pub settings: StorageSettings,
    pub rules: HostRules,
    pub traffic_rules: TrafficRules,
    pub ports: std::ops::Range<u16>,
    /// Extra roots to trust upstream. Only tests have any.
    pub extra_roots: Vec<PathBuf>,
    /// Look for the program on this machine behind each connection. Off, every
    /// flow is as if it came from another device, which is how tests get one.
    pub find_apps: bool,
}

/// Where the slow network's conditions are kept, in the data folder.
pub const THROTTLE_FILE: &str = "throttle.toml";

pub struct Engine {
    pub addr: SocketAddr,
    pub key_store: KeyStore,
    pub session: PathBuf,
    pub settings: StorageSettings,
    pub shared: Arc<Shared>,
    pub recording: Arc<AtomicBool>,
    pub rules: Arc<RwLock<HostRules>>,
    pub traffic_rules: Arc<RwLock<TrafficRules>>,
    /// A slow network on purpose, as `throttle.toml` keeps it.
    pub network: Arc<Network>,
    pub check: Arc<Check>,
    /// What a browser opened from here needs when it does not trust the CA.
    pub leaf_spki: Arc<OnceLock<String>>,
    /// What breakpoints hold.
    pub pauses: Arc<Pauses>,
    /// Why the proxy stopped, if it did by itself.
    pub failure: Arc<Mutex<Option<String>>>,
    /// Whether other devices may use the proxy, and where it listens for them.
    pub other_devices: Arc<OtherDevices>,
    /// The other settings that apply as they change.
    pub live: Arc<Live>,
    /// The authorities trusted for servers besides the public and the
    /// system's, as the proxy was started with them.
    pub extra_roots: Vec<PathBuf>,
    /// Another proxy that connections to servers go through, as a company's.
    pub relay: Arc<Relay>,
    /// Where TLS secrets are written for Wireshark: off at each start, unless
    /// `SSLKEYLOGFILE` says where.
    pub tls_keys: Arc<TlsKeys>,
    /// The WebSockets open now, to send messages into.
    pub sockets: Arc<Sockets>,
    /// Certificates to show servers that ask the client for one.
    pub client_certs: Arc<ClientCerts>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Engine {
    pub fn start(opts: EngineOptions) -> Result<Self> {
        let sessions = opts.data_dir.join("sessions");
        session::forget_old(&sessions, opts.settings.sessions_kept);
        let session = sessions.join(next_session_name(&sessions));
        let (listener, addr) = bind(opts.ports)?;
        let other_devices = Arc::new(OtherDevices::new(opts.settings.other_devices));
        let live = Arc::new(Live::new(&opts.settings));

        let shared = Arc::new(Shared::default());
        let recording = Arc::new(AtomicBool::new(true));
        let failure = Arc::new(Mutex::new(None));
        let check = Arc::new(Check::default());
        let leaf_spki = Arc::new(OnceLock::new());
        let pauses = Arc::new(Pauses::default());
        let rules = Arc::new(RwLock::new(opts.rules));
        let traffic_rules = Arc::new(RwLock::new(opts.traffic_rules));
        // A file that cannot be read leaves the network as it is.
        let conditions = Conditions::load(&opts.data_dir.join(THROTTLE_FILE)).unwrap_or_default();
        // The same for a relay: connections go straight, and Settings says why.
        let relay = Arc::new(
            RelaySettings::load_kept(
                &opts.data_dir,
                opts.key_store,
                &tracewing_proxy::ca_key::OsKeychain,
            )
            .and_then(|(settings, _)| Relay::new(&settings))
            .unwrap_or_default(),
        );
        let network = Arc::new(Network::new(conditions));
        let tls_keys = Arc::new(TlsKeys::from_env());
        let sockets = Arc::new(Sockets::default());
        // A file that cannot be read means none, and Settings says why.
        let client_certs = Arc::new(ClientCerts::new(
            &ClientCertSettings::load(&opts.data_dir).unwrap_or_default(),
        ));
        let (stop, stopped) = oneshot::channel::<()>();
        let proxy = ProxyOptions {
            listener: None,
            std_listener: Some(listener),
            data_dir: opts.data_dir,
            key_store: opts.key_store,
            session: session.clone(),
            settings: opts.settings.clone(),
            extra_roots: opts.extra_roots.clone(),
            queue: 100_000,
            stats_every: None,
            shared: Arc::clone(&shared),
            recording: Arc::clone(&recording),
            rules: Arc::clone(&rules),
            traffic_rules: Arc::clone(&traffic_rules),
            network: Arc::clone(&network),
            check: Arc::clone(&check),
            leaf_spki: Arc::clone(&leaf_spki),
            pauses: Arc::clone(&pauses),
            leave_index: true,
            find_apps: opts.find_apps,
            stop_limit: std::time::Duration::from_secs(2),
            live: Arc::clone(&live),
            other_devices: Some(Arc::clone(&other_devices)),
            relay: Arc::clone(&relay),
            tls_keys: Arc::clone(&tls_keys),
            sockets: Arc::clone(&sockets),
            client_certs: Arc::clone(&client_certs),
        };
        let thread = {
            let failure = Arc::clone(&failure);
            std::thread::Builder::new()
                .name("proxy".into())
                .spawn(move || {
                    let result = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .map_err(anyhow::Error::from)
                        .and_then(|runtime| {
                            runtime.block_on(run_proxy(proxy, async {
                                let _ = stopped.await;
                            }))
                        });
                    if let Err(e) = result {
                        *failure.lock().unwrap_or_else(|e| e.into_inner()) = Some(format!("{e:#}"));
                    }
                })
                .context("start the proxy")?
        };
        Ok(Self {
            addr,
            key_store: opts.key_store,
            session,
            settings: opts.settings,
            shared,
            recording,
            rules,
            traffic_rules,
            network,
            check,
            leaf_spki,
            pauses,
            failure,
            other_devices,
            live,
            extra_roots: opts.extra_roots,
            relay,
            tls_keys,
            sockets,
            client_certs,
            stop: Some(stop),
            thread: Some(thread),
        })
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Relaxed)
    }

    pub fn set_recording(&self, on: bool) {
        self.recording.store(on, Ordering::Relaxed);
    }

    /// Takes what applies at once from settings just written; the rest wait
    /// for the next start.
    pub fn apply_settings(&mut self, written: &StorageSettings) {
        self.settings.take_live(written);
        self.other_devices.set(written.other_devices);
        self.live.apply(written);
    }

    pub fn failure(&self) -> Option<String> {
        self.failure
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Stops the proxy and waits for what is queued to be written.
    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Listens on this machine's own address, which is what the app and the system
/// proxy use. Other devices get listeners of their own on the same port.
fn bind(ports: std::ops::Range<u16>) -> Result<(std::net::TcpListener, SocketAddr)> {
    let mut last = None;
    for port in ports {
        match std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
            Ok(listener) => {
                listener.set_nonblocking(true)?;
                let port = listener.local_addr()?.port();
                return Ok((listener, SocketAddr::from((Ipv4Addr::LOCALHOST, port))));
            }
            Err(e) => last = Some(e),
        }
    }
    Err(last.map_or_else(
        || anyhow::anyhow!("no port to listen on"),
        |e| anyhow::Error::from(e).context("no free port to listen on"),
    ))
}

/// Seconds since 1970, and a suffix if two starts fall in one second.
pub(crate) fn next_session_name(sessions: &Path) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut name = now.to_string();
    let mut n = 1;
    while sessions.join(&name).exists() {
        n += 1;
        name = format!("{now}-{n}");
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_taken_port_means_the_next_one() {
        let (held, first) = bind(49_321..49_331).unwrap();
        let (_, second) = bind(first.port()..first.port() + 10).unwrap();
        assert_ne!(first.port(), second.port());
        drop(held);
    }

    #[test]
    fn it_listens_on_this_machine_only() {
        let (listener, addr) = bind(49_331..49_341).unwrap();
        assert!(listener.local_addr().unwrap().ip().is_loopback());
        assert_eq!(addr, listener.local_addr().unwrap());
    }
}
