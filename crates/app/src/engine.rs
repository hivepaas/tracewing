//! The proxy, running beside the window on threads of its own.

use anyhow::{Context as _, Result};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};
use tokio::sync::oneshot;
use tracewing_proxy::{ProxyOptions, capture::Check, rules::HostRules, run_proxy};
use tracewing_store::{session::Shared, settings::StorageSettings};

/// Where the proxy tries to listen, in this order.
pub const PORTS: std::ops::Range<u16> = 9090..9100;

pub struct EngineOptions {
    pub data_dir: PathBuf,
    pub settings: StorageSettings,
    pub rules: HostRules,
    pub ports: std::ops::Range<u16>,
    /// Extra roots to trust upstream. Only tests have any.
    pub extra_roots: Vec<PathBuf>,
}

pub struct Engine {
    pub addr: SocketAddr,
    pub session: PathBuf,
    pub settings: StorageSettings,
    pub shared: Arc<Shared>,
    pub recording: Arc<AtomicBool>,
    pub rules: Arc<RwLock<HostRules>>,
    pub check: Arc<Check>,
    /// Why the proxy stopped, if it did by itself.
    pub failure: Arc<Mutex<Option<String>>>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Engine {
    pub fn start(opts: EngineOptions) -> Result<Self> {
        let sessions = opts.data_dir.join("sessions");
        forget_old_sessions(&sessions, opts.settings.sessions_kept);
        let session = sessions.join(next_session_name(&sessions));
        let (listener, addr) = bind(opts.ports)?;

        let shared = Arc::new(Shared::default());
        let recording = Arc::new(AtomicBool::new(true));
        let failure = Arc::new(Mutex::new(None));
        let check = Arc::new(Check::default());
        let rules = Arc::new(RwLock::new(opts.rules));
        let (stop, stopped) = oneshot::channel::<()>();
        let proxy = ProxyOptions {
            listener: None,
            std_listener: Some(listener),
            data_dir: opts.data_dir,
            session: session.clone(),
            settings: opts.settings.clone(),
            extra_roots: opts.extra_roots,
            queue: 100_000,
            stats_every: None,
            shared: Arc::clone(&shared),
            recording: Arc::clone(&recording),
            rules: Arc::clone(&rules),
            check: Arc::clone(&check),
            leave_index: true,
            stop_limit: std::time::Duration::from_secs(2),
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
            session,
            settings: opts.settings,
            shared,
            recording,
            rules,
            check,
            failure,
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

fn bind(ports: std::ops::Range<u16>) -> Result<(std::net::TcpListener, SocketAddr)> {
    let mut last = None;
    for port in ports {
        match std::net::TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => {
                listener.set_nonblocking(true)?;
                let addr = listener.local_addr()?;
                return Ok((listener, addr));
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
fn next_session_name(sessions: &Path) -> String {
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

/// Sessions cannot be opened again yet, but they will be, so a few stay.
fn forget_old_sessions(sessions: &Path, kept: u64) {
    let Ok(entries) = std::fs::read_dir(sessions) else {
        return;
    };
    let mut names: Vec<PathBuf> = entries
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.join("flows.db").exists())
        .collect();
    names.sort_unstable();
    let extra = names.len().saturating_sub(kept as usize);
    for old in &names[..extra] {
        let _ = std::fs::remove_dir_all(old);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_newest_sessions_stay() {
        let dir = std::env::temp_dir().join(format!("sessions-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for name in ["100", "200", "300"] {
            std::fs::create_dir_all(dir.join(name)).unwrap();
            std::fs::write(dir.join(name).join("flows.db"), b"").unwrap();
        }
        // Not a session: left alone, whatever it is.
        std::fs::create_dir_all(dir.join("000-notes")).unwrap();
        forget_old_sessions(&dir, 2);
        assert!(!dir.join("100").exists());
        assert!(dir.join("200").exists() && dir.join("300").exists());
        assert!(dir.join("000-notes").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_taken_port_means_the_next_one() {
        let (held, first) = bind(49_321..49_331).unwrap();
        let (_, second) = bind(first.port()..first.port() + 10).unwrap();
        assert_ne!(first.port(), second.port());
        drop(held);
    }
}
