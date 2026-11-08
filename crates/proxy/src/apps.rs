//! Which program on this machine is behind each connection to the proxy.
//!
//! The OS knows which process holds each socket, but asking takes a look through
//! every process's open files on Linux and macOS: far too slow for the handler.
//! So one thread does it. A connection is looked for once, as it opens, and its
//! flows share the answer; asks that come while the thread is busy wait and are
//! looked for together, in one pass. The processes found last are looked in
//! first, as a browser makes many connections.

#[cfg(target_os = "linux")]
#[path = "apps/linux.rs"]
mod os;
#[cfg(target_os = "macos")]
#[path = "apps/macos.rs"]
mod os;
#[cfg(windows)]
#[path = "apps/windows.rs"]
mod os;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod os {
    pub fn owners(clients: &[std::net::SocketAddr], _: u16, _: &[u32]) -> Vec<Option<u32>> {
        vec![None; clients.len()]
    }

    pub fn path_of(_: u32) -> Option<String> {
        None
    }
}

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr, UdpSocket},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};
use tracewing_store::app::{App, AppAnswer, name_of};

/// An answer is for one connection, but nothing says when that ends. Past this, a
/// flow from the same address and port asks again: the port may be someone else's.
const FRESH_FOR: Duration = Duration::from_secs(30);
/// Answers kept before the old ones go.
const KEPT_MAX: usize = 4_096;
/// How long what is known of a process is trusted: its id may be given again.
const PROCESS_FRESH_FOR: Duration = Duration::from_secs(60);
/// Processes looked in first.
const RECENT_MAX: usize = 16;

#[derive(Clone)]
pub struct Apps {
    inner: Arc<Inner>,
}

struct Inner {
    asks: Mutex<Sender<Ask>>,
    by_client: Mutex<HashMap<SocketAddr, (Instant, Arc<AppAnswer>)>>,
    /// Whether an address is this machine's: only those have a program to find.
    local: Mutex<HashMap<IpAddr, bool>>,
}

struct Ask {
    client: SocketAddr,
    answer: Arc<AppAnswer>,
}

impl Apps {
    /// `proxy_port` is where clients connect, which tells their sockets apart
    /// from the rest.
    pub fn spawn(proxy_port: u16) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("apps".into())
            .spawn(move || look_for_apps(rx, proxy_port))?;
        Ok(Self {
            inner: Arc::new(Inner {
                asks: Mutex::new(tx),
                by_client: Mutex::default(),
                local: Mutex::default(),
            }),
        })
    }

    /// The program behind the connection from `client`, when it is on this
    /// machine. `opened` says the connection is new, so what was known of the
    /// address and port is someone else's.
    pub fn of(&self, client: SocketAddr, opened: bool) -> Option<Arc<AppAnswer>> {
        if !self.is_local(client.ip()) {
            return None;
        }
        let now = Instant::now();
        let mut by_client = lock(&self.inner.by_client);
        if !opened
            && let Some((asked, answer)) = by_client.get(&client)
            && now.duration_since(*asked) < FRESH_FOR
        {
            return Some(Arc::clone(answer));
        }
        if by_client.len() >= KEPT_MAX {
            by_client.retain(|_, (asked, _)| now.duration_since(*asked) < FRESH_FOR);
            if by_client.len() >= KEPT_MAX {
                by_client.clear();
            }
        }
        let answer = Arc::new(AppAnswer::default());
        by_client.insert(client, (now, Arc::clone(&answer)));
        drop(by_client);
        let ask = Ask {
            client,
            answer: Arc::clone(&answer),
        };
        if lock(&self.inner.asks).send(ask).is_err() {
            answer.give(None);
        }
        Some(answer)
    }

    fn is_local(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        if ip.is_loopback() {
            return true;
        }
        if ip.is_unspecified() || ip.is_multicast() {
            return false;
        }
        *lock(&self.inner.local)
            .entry(ip)
            // Only an address of this machine can be bound to.
            .or_insert_with(|| UdpSocket::bind((ip, 0)).is_ok())
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// The thread: until the proxy lets go of [`Apps`].
fn look_for_apps(asks: Receiver<Ask>, proxy_port: u16) {
    let mut processes: HashMap<u32, (Instant, App)> = HashMap::new();
    let mut recent: Vec<u32> = Vec::new();
    while let Ok(first) = asks.recv() {
        let mut batch = vec![first];
        batch.extend(asks.try_iter());
        let mut clients: Vec<SocketAddr> = batch.iter().map(|ask| ask.client).collect();
        clients.sort_unstable();
        clients.dedup();
        let owners: HashMap<SocketAddr, u32> = clients
            .iter()
            .zip(os::owners(&clients, proxy_port, &recent))
            .filter_map(|(client, owner)| Some((*client, owner?)))
            .collect();
        let now = Instant::now();
        processes.retain(|_, (seen, _)| now.duration_since(*seen) < PROCESS_FRESH_FOR);
        for ask in &batch {
            let app = owners.get(&ask.client).map(|&pid| {
                recent.retain(|p| *p != pid);
                recent.insert(0, pid);
                recent.truncate(RECENT_MAX);
                processes
                    .entry(pid)
                    .or_insert_with(|| (now, describe(pid)))
                    .1
                    .clone()
            });
            ask.answer.give(app);
        }
    }
}

fn describe(pid: u32) -> App {
    let path = os::path_of(pid);
    App {
        pid,
        name: path
            .as_deref()
            .and_then(name_of)
            .unwrap_or_else(|| format!("pid {pid}")),
        path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};

    /// The test is its own client: the program found must be this one.
    #[test]
    fn a_connection_from_here_is_this_program() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let apps = Apps::spawn(port).unwrap();
        let client = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let _server = listener.accept().unwrap();
        let answer = apps.of(client.local_addr().unwrap(), true).unwrap();
        let app = answer
            .wait_until(Instant::now() + Duration::from_secs(10))
            .expect("found");
        assert_eq!(app.pid, std::process::id());
        let same = |path: &str| std::fs::canonicalize(path).unwrap();
        let exe = std::env::current_exe().unwrap();
        assert_eq!(
            app.path.as_deref().map(same),
            Some(std::fs::canonicalize(exe).unwrap())
        );
        // Flows over the same connection share the answer.
        let again = apps.of(client.local_addr().unwrap(), false).unwrap();
        assert!(Arc::ptr_eq(&answer, &again));
    }

    #[test]
    fn many_connections_at_once_are_all_found() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let apps = Apps::spawn(port).unwrap();
        let clients: Vec<TcpStream> = (0..20)
            .map(|_| TcpStream::connect(("127.0.0.1", port)).unwrap())
            .collect();
        let answers: Vec<_> = clients
            .iter()
            .map(|c| apps.of(c.local_addr().unwrap(), true).unwrap())
            .collect();
        let deadline = Instant::now() + Duration::from_secs(10);
        for answer in answers {
            assert_eq!(
                answer.wait_until(deadline).map(|app| app.pid),
                Some(std::process::id())
            );
        }
    }

    #[test]
    fn a_connection_that_is_not_there_has_no_program() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let apps = Apps::spawn(port).unwrap();
        let answer = apps.of("127.0.0.1:9".parse().unwrap(), true).unwrap();
        assert_eq!(
            answer.wait_until(Instant::now() + Duration::from_secs(10)),
            None
        );
        assert_eq!(answer.now(), Some(None));
    }

    #[test]
    fn another_machine_is_not_looked_for() {
        let apps = Apps::spawn(1).unwrap();
        // TEST-NET-1: never this machine's.
        assert!(apps.of("192.0.2.7:5000".parse().unwrap(), true).is_none());
        assert!(apps.is_local("127.0.0.1".parse().unwrap()));
        assert!(apps.is_local("::ffff:127.0.0.1".parse().unwrap()));
    }
}
