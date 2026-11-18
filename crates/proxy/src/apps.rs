//! Which program on this machine is behind each connection to the proxy.
//!
//! There are three questions here, not one. A client that **asked** for the
//! proxy holds a socket whose far end is a port of tracewing's, and that is
//! what picks it out of the thousands a machine has open. A connection
//! **taken on its way** -- off a TUN adapter, where the operating system
//! routed it to us and the client believes it is talking to the server --
//! has the real server at its far end, so that rule finds nothing at all;
//! what names it is all four addresses, which a source of packets has in
//! hand. And a **datagram** -- a QUIC attempt the proxy refused, which no
//! connection of ours follows -- is in the other table a system keeps, and
//! is found by the end on this machine alone. [`Look`] is the question,
//! [`Apps::of`] asks the first, [`Apps::took`] the second and
//! [`Apps::sent`] the third.
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
    pub fn owners(looks: &[super::Look], _: &[u16], _: &[u32]) -> Vec<Option<u32>> {
        vec![None; looks.len()]
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
    by_client: Mutex<HashMap<Kept, (Instant, Arc<AppAnswer>)>>,
    /// Whether an address is this machine's: only those have a program to find.
    local: Mutex<HashMap<IpAddr, bool>>,
}

/// One socket to find the program behind.
///
/// A client that asked for the proxy is at one end of a socket whose other
/// end is a port of tracewing's, and that is what picks its socket out of a
/// machine's thousands. A connection **taken on its way** has the real server
/// at its far end, so that rule finds nothing: what identifies it is all four
/// addresses, which a source that deals in packets has in hand.
///
/// A datagram is a narrower question than either. A refused QUIC attempt is
/// one packet and no connection, so there is nothing whose two ends could be
/// asked for: what is asked for is the socket the packet left, by the end on
/// this machine. Not by choice -- Windows's table of UDP sockets has no far
/// end in it at all, and an unconnected socket has none to have -- but it is
/// enough, because that end is what the system handed out.
///
/// Which it is decides how a backend looks, so it is a variant rather than a
/// field: a system told about datagrams has to say what it does with them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Look {
    /// A connection, by its ends.
    Connection {
        /// The end on this machine.
        client: SocketAddr,
        /// Where it was going, for one taken on its way. `None` says the far
        /// end is tracewing's own, and `ours` is what to look for.
        to: Option<SocketAddr>,
    },
    /// A datagram's socket, by the end it left.
    Datagram {
        /// The end on this machine: the address and port the datagram was
        /// sent from, as the packet carried them.
        client: SocketAddr,
    },
}

impl Look {
    /// The end on this machine, which every one of the three questions has.
    pub(crate) fn client(self) -> SocketAddr {
        match self {
            Self::Connection { client, .. } | Self::Datagram { client } => client,
        }
    }

    /// Addresses as a packet carries them, so that one socket is one key
    /// whichever family the socket holding it was opened for.
    pub(crate) fn plain(self) -> Self {
        match self {
            Self::Connection { client, to } => Self::Connection {
                client: canonical(client),
                to: to.map(canonical),
            },
            Self::Datagram { client } => Self::Datagram {
                client: canonical(client),
            },
        }
    }

    /// What the answer is kept by. The two flows of one connection share an
    /// answer, which is why this is the client's end rather than the whole
    /// question -- but a machine can hold a connection and a datagram socket
    /// on one address and port at the same time, and they are two programs'
    /// as often as one's, so that much is part of the key.
    fn kept_by(self) -> Kept {
        Kept {
            client: self.client(),
            datagram: matches!(self, Self::Datagram { .. }),
        }
    }
}

/// What an answer is kept by: see [`Look::kept_by`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Kept {
    client: SocketAddr,
    datagram: bool,
}

/// An address as a packet carries it. A socket open for both families reports
/// an IPv4 peer mapped into IPv6, and nothing else does, so the two spellings
/// have to meet somewhere.
pub(crate) fn canonical(addr: SocketAddr) -> SocketAddr {
    SocketAddr::new(addr.ip().to_canonical(), addr.port())
}

struct Ask {
    look: Look,
    answer: Arc<AppAnswer>,
}

impl Apps {
    /// `ours` is every port of tracewing's that clients connect to, which is
    /// what tells their sockets apart from the rest: the proxy port, and a
    /// port a source of its own opened, such as SOCKS5's. The port most
    /// clients come in on goes first, because that is the one looked for
    /// first.
    pub fn spawn(ours: Vec<u16>) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("apps".into())
            .spawn(move || look_for_apps(rx, &ours))?;
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
        self.looking(Look::Connection { client, to: None }, opened)
    }

    /// The program behind a connection **taken on its way**, going to `to`
    /// rather than to tracewing. Always a new connection: a source of packets
    /// asks as the `SYN` arrives, and that port was somebody else's before.
    ///
    /// Why this is not [`Self::of`] with the destination thrown away: it would
    /// find nothing. See [`Look`].
    pub fn took(&self, client: SocketAddr, to: SocketAddr) -> Option<Arc<AppAnswer>> {
        self.looking(
            Look::Connection {
                client,
                to: Some(to),
            },
            true,
        )
    }

    /// The program that sent a datagram from `client`, when that is an address
    /// of this machine. A refused QUIC attempt is the only one asked for.
    ///
    /// Always a new socket as far as this is concerned: the attempt is told
    /// about once and the next one is a question of its own, and a UDP port
    /// is handed out again sooner than a connection's is.
    ///
    /// Why this is not [`Self::of`] or [`Self::took`]: see [`Look`].
    pub fn sent(&self, client: SocketAddr) -> Option<Arc<AppAnswer>> {
        self.looking(Look::Datagram { client }, true)
    }

    fn looking(&self, look: Look, opened: bool) -> Option<Arc<AppAnswer>> {
        let kept = look.kept_by();
        if !self.is_local(kept.client.ip()) {
            return None;
        }
        let now = Instant::now();
        let mut by_client = lock(&self.inner.by_client);
        if !opened
            && let Some((asked, answer)) = by_client.get(&kept)
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
        by_client.insert(kept, (now, Arc::clone(&answer)));
        drop(by_client);
        let ask = Ask {
            look,
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
fn look_for_apps(asks: Receiver<Ask>, ours: &[u16]) {
    let mut processes: HashMap<u32, (Instant, App)> = HashMap::new();
    let mut recent: Vec<u32> = Vec::new();
    while let Ok(first) = asks.recv() {
        let mut batch = vec![first];
        batch.extend(asks.try_iter());
        let mut looks: Vec<Look> = batch.iter().map(|ask| ask.look).collect();
        looks.sort_unstable();
        looks.dedup();
        let owners: HashMap<Look, u32> = looks
            .iter()
            .zip(os::owners(&looks, ours, &recent))
            .filter_map(|(look, owner)| Some((*look, owner?)))
            .collect();
        let now = Instant::now();
        processes.retain(|_, (seen, _)| now.duration_since(*seen) < PROCESS_FRESH_FOR);
        for ask in &batch {
            let app = owners.get(&ask.look).map(|&pid| {
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

/// The program behind `pid`: its file, and what to call it.
pub fn describe(pid: u32) -> App {
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
        let apps = Apps::spawn(vec![port]).unwrap();
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

    /// A datagram's socket is found in the other table the system keeps. The
    /// test is its own sender, so the program found must be this one.
    #[test]
    fn a_datagram_sent_from_here_is_this_program() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mine = socket.local_addr().unwrap();
        // No port of tracewing's is given: a datagram was refused rather than
        // taken, so there is no port of ours it ever reached.
        let apps = Apps::spawn(Vec::new()).unwrap();
        let answer = apps.sent(mine).unwrap();
        let app = answer
            .wait_until(Instant::now() + Duration::from_secs(10))
            .expect("found");
        assert_eq!(app.pid, std::process::id());

        // A port nothing holds: answered, and the answer is that there is no
        // program -- which is not the same as a question still in flight.
        let nobody = apps.sent("127.0.0.1:9".parse().unwrap()).unwrap();
        assert_eq!(
            nobody.wait_until(Instant::now() + Duration::from_secs(10)),
            None
        );
        assert_eq!(nobody.now(), Some(None), "answered, not timed out");
    }

    /// One machine can hold a connection and a datagram socket on the same
    /// address and port, and they are two different programs' as often as
    /// one's. So the two are two questions, with two answers kept.
    ///
    /// What makes this assertable is that the address here holds *only* a
    /// datagram socket: asked as a datagram it is this program, asked as a
    /// connection it is nobody. A key that left the difference out would hand
    /// the second ask the first one's answer, and the second line would read
    /// as this program as well.
    #[test]
    fn a_datagram_and_a_connection_on_one_port_are_two_questions() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mine = socket.local_addr().unwrap();
        let apps = Apps::spawn(Vec::new()).unwrap();
        let sent = apps.sent(mine).unwrap();
        let connection = apps.of(mine, false).unwrap();
        assert!(!Arc::ptr_eq(&sent, &connection), "two asks, not one");
        let soon = || Instant::now() + Duration::from_secs(10);
        assert_eq!(
            sent.wait_until(soon()).map(|app| app.pid),
            Some(std::process::id())
        );
        assert_eq!(
            connection.wait_until(soon()),
            None,
            "nothing on this machine holds a connection with that end"
        );
        assert_eq!(connection.now(), Some(None), "answered, not timed out");
    }

    /// A connection that came in on another of tracewing's ports, as a SOCKS5
    /// client's does, is found as one to the proxy port is.
    #[test]
    fn a_connection_on_another_of_our_ports_is_found() {
        let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
        let socks = TcpListener::bind("127.0.0.1:0").unwrap();
        let socks_port = socks.local_addr().unwrap().port();
        let apps = Apps::spawn(vec![proxy.local_addr().unwrap().port(), socks_port]).unwrap();
        let client = TcpStream::connect(("127.0.0.1", socks_port)).unwrap();
        let _server = socks.accept().unwrap();
        let answer = apps.of(client.local_addr().unwrap(), true).unwrap();
        let app = answer
            .wait_until(Instant::now() + Duration::from_secs(10))
            .expect("found");
        assert_eq!(app.pid, std::process::id());
    }

    /// A connection taken on its way is found by all four addresses. The rule
    /// for a client of tracewing's cannot find it, which is the whole reason
    /// [`Apps::took`] exists, so that is asserted first and in the same test:
    /// apart they would both pass against a look-up that answered nothing.
    #[test]
    fn a_connection_taken_on_its_way_is_found_by_where_it_was_going() {
        let elsewhere = TcpListener::bind("127.0.0.1:0").unwrap();
        let to = elsewhere.local_addr().unwrap();
        let ours = TcpListener::bind("127.0.0.1:0").unwrap();
        let apps = Apps::spawn(vec![ours.local_addr().unwrap().port()]).unwrap();
        let client = TcpStream::connect(to).unwrap();
        let _server = elsewhere.accept().unwrap();
        let from = client.local_addr().unwrap();
        let soon = || Instant::now() + Duration::from_secs(10);

        let by_client = apps.of(from, true).unwrap();
        assert_eq!(
            by_client.wait_until(soon()),
            None,
            "its far end is not ours"
        );
        // Without this the line above would pass on a look-up that answers
        // nothing at all, timeout and emptiness being the same `None`.
        assert_eq!(by_client.now(), Some(None), "answered, not timed out");

        let taken = apps.took(from, to).unwrap();
        let app = taken.wait_until(soon()).expect("found");
        assert_eq!(app.pid, std::process::id());
    }

    /// And a connection to a port that is none of ours is not: this program's
    /// own traffic goes to plenty of them, and none of it is being proxied.
    #[test]
    fn a_connection_to_a_port_that_is_not_ours_is_not_found() {
        let ours = TcpListener::bind("127.0.0.1:0").unwrap();
        let elsewhere = TcpListener::bind("127.0.0.1:0").unwrap();
        let apps = Apps::spawn(vec![ours.local_addr().unwrap().port()]).unwrap();
        let client = TcpStream::connect(elsewhere.local_addr().unwrap()).unwrap();
        let _server = elsewhere.accept().unwrap();
        let answer = apps.of(client.local_addr().unwrap(), true).unwrap();
        assert_eq!(
            answer.wait_until(Instant::now() + Duration::from_secs(10)),
            None
        );
    }

    #[test]
    fn many_connections_at_once_are_all_found() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let apps = Apps::spawn(vec![port]).unwrap();
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
        let apps = Apps::spawn(vec![port]).unwrap();
        let answer = apps.of("127.0.0.1:9".parse().unwrap(), true).unwrap();
        assert_eq!(
            answer.wait_until(Instant::now() + Duration::from_secs(10)),
            None
        );
        assert_eq!(answer.now(), Some(None));
    }

    #[test]
    fn another_machine_is_not_looked_for() {
        let apps = Apps::spawn(vec![1]).unwrap();
        // TEST-NET-1: never this machine's.
        assert!(apps.of("192.0.2.7:5000".parse().unwrap(), true).is_none());
        assert!(apps.is_local("127.0.0.1".parse().unwrap()));
        assert!(apps.is_local("::ffff:127.0.0.1".parse().unwrap()));
    }
}
