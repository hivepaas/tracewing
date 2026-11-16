//! Where traffic comes from.
//!
//! A source brings connections to tracewing. See
//! [capture-modes.md](../../../docs/design/capture-modes.md).
//!
//! What any source has to deliver is the same two things: where the connection
//! was going, and who sent it. What differs is how much the client keeps
//! saying about the first:
//!
//! - **It says so with every request.** An HTTP proxy's client sends an
//!   absolute URI, or a `CONNECT` before each tunnel: the proxy port
//!   ([`ProxyPort`]) and the addresses other devices reach ([`Lan`]). These
//!   hand over a [`Took::Port`] and the core reads the rest off the wire.
//! - **It says so once and then stops.** A SOCKS5 client names its
//!   destination in the handshake and after that the socket carries nothing
//!   but the conversation ([`crate::socks`]).
//! - **It never says at all,** because it was never told a proxy exists: a
//!   connection the operating system handed over. Nothing of these is built
//!   yet, and [`Given`] is where they will arrive.
//!
//! The last two both hand over a [`Took::Flow`], with the destination the
//! source knew. Which is why the shape of this interface is not "told or
//! taken", as it looked when there were two sources: it is whether the
//! destination is on the wire or in the source's hand.
//!
//! Who sent it is looked up by address for the ones that keep asking
//! ([`crate::apps`]), and known in advance by a source the OS told.

use crate::lan::OtherDevices;
use anyhow::{Context, Result};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    sync::{mpsc, oneshot, watch},
};
use tracewing_store::app::App;

/// How many found connections wait to be served before a source has to wait
/// with them. Every one of them is handed on to a task of its own at once, so
/// this fills only under a flood.
const WAITING_MAX: usize = 256;

/// How often the addresses this machine has on its networks are looked at
/// while other devices may use the proxy.
const LAN_LOOK_EVERY: Duration = Duration::from_secs(3);

/// Set once, to stop every source and every listener they opened.
pub type Stop = watch::Receiver<bool>;

/// The two ends of a connection, whatever carries them. A source that is not
/// a listener has no `TcpStream` to give: a flow from a macOS network
/// extension arrives over a unix socket, and one from WireGuard or a TUN
/// device comes out of a TCP stack of our own.
pub trait Duplex: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T: AsyncRead + AsyncWrite + Send + Unpin> Duplex for T {}

/// Which way in a source is.
///
/// Not [`Source::name`], which is a line for a human and carries an address
/// in two cases. This is a word that does not change, because the window
/// looks a count up by it and a tab's sentence turns on the answer.
///
/// Three ways in share [`By::ProxyPort`] — this machine's proxy setting, a
/// browser tracewing started with a profile of its own, and a terminal it
/// set the variables in — because all three are told the same port and the
/// same listener accepts them. Nothing here can tell those three apart, and
/// a window that claims to is wrong; everything else has a listener or a
/// bridge of its own and is exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum By {
    /// The port on this machine's loopback that clients are told to use.
    ProxyPort,
    /// A listener on one of this machine's network addresses, for a phone or
    /// another computer ([`Lan`]).
    OtherDevices,
    Socks5,
    /// macOS's network extension, over the port it is given.
    Extension,
    WireGuard,
    /// A TUN device this machine's traffic is routed into.
    Tun,
    /// Handed over from elsewhere in the program, which is what [`Given`] is
    /// for, and what a test uses.
    Given,
}

impl By {
    pub const ALL: [Self; 7] = [
        Self::ProxyPort,
        Self::OtherDevices,
        Self::Socks5,
        Self::Extension,
        Self::WireGuard,
        Self::Tun,
        Self::Given,
    ];
}

/// What has arrived by each way in, and when the last of it did.
///
/// The question this is here to answer is "is this way in on and doing
/// nothing", which is the state tracewing holds and has never said: a switch
/// that is on, a program that ignores it, and no line anywhere saying so.
///
/// `Instant`, not a wall clock, because the only thing ever asked of these is
/// whether something arrived **after** a moment the window remembers — the
/// moment a switch went on — and a clock that can jump would answer that
/// wrongly for anybody whose machine sleeps.
#[derive(Default)]
pub struct Arrivals(Mutex<[Arrival; By::ALL.len()]>);

/// What has arrived by one way in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Arrival {
    /// Requests seen, which is what the proxy counts as it serves them. Zero
    /// or not zero is the whole of what the window asks.
    pub count: u64,
    pub last: Option<Instant>,
}

impl Arrivals {
    /// Something came in by `by`, now.
    pub fn came(&self, by: By) {
        if let Ok(mut slots) = self.0.lock() {
            let slot = &mut slots[by as usize];
            slot.count += 1;
            slot.last = Some(Instant::now());
        }
    }

    pub fn of(&self, by: By) -> Arrival {
        self.0
            .lock()
            .map(|slots| slots[by as usize])
            .unwrap_or_default()
    }

    /// Whether anything has arrived by `by` since `since`.
    ///
    /// This and not a bare count, because a count that was not zero
    /// yesterday is not zero today: somebody who has just turned a switch on
    /// would be told that their way in works, on the strength of traffic from
    /// last week.
    pub fn anything_since(&self, by: By, since: Instant) -> bool {
        self.of(by).last.is_some_and(|last| last >= since)
    }
}

/// Something that brings traffic to tracewing.
pub trait Source: Send + 'static {
    /// What it is, for the logs.
    fn name(&self) -> String;

    /// Which way in it is, for counting what arrives by it.
    ///
    /// No default: a source that did not say would be counted as something
    /// else, and the window would then tell somebody that their way in is
    /// quiet when it is not, or busy when it is not. A missing answer is
    /// better as a compiler error than as a wrong sentence.
    fn by(&self) -> By;

    /// Starts it, and returns once it is running rather than once it is done.
    /// What it finds goes to `found`; it stops when `stop` is set.
    ///
    /// `proxy` is where clients reach tracewing on this machine: the port a
    /// taken connection is handed to, and the one a source has to leave out
    /// of what it captures, or it will capture tracewing itself.
    fn start(self: Box<Self>, proxy: SocketAddr, found: Found, stop: Stop) -> Result<()>;

    /// The ports of tracewing's own that programs on this machine connect to.
    /// [`crate::apps`] finds the program behind a connection by looking for a
    /// socket whose far end is one of these, so a source that opens a port
    /// clients reach has to say which it is, or the flows that come in there
    /// carry no program's name.
    ///
    /// A source that hands over flows taken elsewhere has none: nothing on
    /// this machine connected to tracewing, so there is no such socket to
    /// find, and what the operating system told it about the program belongs
    /// in [`Flow::who`].
    fn own_ports(&self) -> Vec<u16> {
        Vec::new()
    }
}

/// What a source found: either a way for clients to reach the proxy, or one
/// connection that is already on its way somewhere — and which way in it
/// came by.
///
/// The way in rides here rather than on [`Port`] or [`Flow`] so that no
/// source sets it: [`Found`] stamps it on the way through. A field each
/// implementor had to remember is a field the next implementor forgets, and
/// the forgetting would be silent — a new source would simply never be
/// counted, and nobody would notice until somebody asked why a tab said
/// nothing had arrived.
pub enum Took {
    Port(Port, By),
    Flow(Flow, By),
}

/// A listener whose clients say where they are going, with `CONNECT` or an
/// absolute URI.
pub struct Port {
    pub listener: TcpListener,
    /// What this one is, for the logs.
    pub name: String,
    /// Stops this listener alone, while the rest carry on: the addresses other
    /// devices reach this machine at come and go as networks do. Dropping the
    /// sender is what stops it. `None` lasts as long as the proxy.
    pub until: Option<oneshot::Receiver<()>>,
    /// Whether the proxy is pointless without it. The port clients are told to
    /// use is: a failure there stops tracewing, while one on a single LAN
    /// address is a warning and the others go on.
    pub essential: bool,
}

/// One connection taken on its way, which never asked for a proxy.
pub struct Flow {
    pub stream: Box<dyn Duplex>,
    /// The other end, as the OS sees it.
    pub client: SocketAddr,
    /// Where it was going before tracewing took it.
    pub to: Target,
    /// The program behind it, where the source was told by the OS. `None`
    /// leaves it to be looked for by `client`, as a client that asked is.
    pub who: Option<App>,
}

/// Where a taken connection was going.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// An address, which is all a packet carries.
    Addr(SocketAddr),
    /// A name, where the source saw one: a TLS ClientHello's `server_name`,
    /// or a `Host` header. Worth the trouble — a flow named by its address
    /// says `140.82.121.4` where this one says `github.com`.
    Named { host: String, port: u16 },
}

impl Target {
    /// What goes in the `CONNECT`, and so what the flow is named by.
    pub fn authority(&self) -> String {
        match self {
            Self::Addr(addr) => addr.to_string(),
            Self::Named { host, port } => format!("{host}:{port}"),
        }
    }
}

/// Where a source hands over what it found, and which way in it is.
///
/// Each source is given its own clone, stamped with its way in by
/// [`Found::by`], so that what it sends is counted as its own without it
/// having to say so again.
#[derive(Clone)]
pub struct Found(mpsc::Sender<Took>, By);

impl Found {
    /// Starts `source`, with its way in stamped on everything it hands over.
    ///
    /// The stamping lives here, with the channel, rather than at the one call
    /// in the proxy: a test that starts a source goes through this too, so
    /// what it sees is what the proxy sees. The alternative — stamping at the
    /// caller — makes every test either repeat the wiring or test something
    /// the proxy does not do.
    pub fn start(&self, source: Box<dyn Source>, proxy: SocketAddr, stop: Stop) -> Result<()> {
        let name = source.name();
        let found = Self(self.0.clone(), source.by());
        source
            .start(proxy, found, stop)
            .with_context(|| format!("start {name}"))
    }

    /// The channel a source sends to, and the end the proxy serves from.
    pub fn channel() -> (Self, mpsc::Receiver<Took>) {
        let (tx, rx) = mpsc::channel(WAITING_MAX);
        // `Given` until a source is stamped with its own: the proxy does
        // that per source, and what reaches here unstamped is what a test
        // hands over itself.
        (Self(tx, By::Given), rx)
    }

    /// Hands over a listener. Returns once it has been taken, or at once if
    /// the proxy has stopped, which is not a source's problem to report.
    pub async fn port(&self, port: Port) {
        let _ = self.0.send(Took::Port(port, self.1)).await;
    }

    /// Hands over one connection.
    pub async fn flow(&self, flow: Flow) {
        let _ = self.0.send(Took::Flow(flow, self.1)).await;
    }
}

/// Waits until the stop is set, and returns at once if it is already. A
/// sender that is gone means the proxy is going down anyway.
pub async fn stopped(mut stop: Stop) {
    loop {
        if *stop.borrow_and_update() {
            return;
        }
        if stop.changed().await.is_err() {
            return;
        }
    }
}

/// The port clients are told to use: a loopback listener, bound before the
/// runtime starts so that the window can say the address at once.
pub struct ProxyPort(TcpListener);

impl ProxyPort {
    pub fn new(listener: TcpListener) -> Self {
        Self(listener)
    }
}

impl Source for ProxyPort {
    fn name(&self) -> String {
        "the proxy port".to_owned()
    }

    fn by(&self) -> By {
        By::ProxyPort
    }

    fn own_ports(&self) -> Vec<u16> {
        self.0
            .local_addr()
            .map_or_else(|_| Vec::new(), |at| vec![at.port()])
    }

    fn start(self: Box<Self>, _proxy: SocketAddr, found: Found, _stop: Stop) -> Result<()> {
        let port = Port {
            name: self.name(),
            listener: self.0,
            until: None,
            essential: true,
        };
        tokio::spawn(async move { found.port(port).await });
        Ok(())
    }
}

/// A source fed from elsewhere in the program: whatever reads flows out of a
/// bridge hands them over here — a macOS network extension's unix socket, a
/// Windows redirector's pipe, a TCP stack of our own above WireGuard. Each of
/// those has an operating system's worth of work in front of it, and none of
/// that work is in this file; this is the end they all arrive at. It is also
/// how a taken connection is tested without an OS to do the taking.
pub struct Given(mpsc::Receiver<Flow>);

impl Given {
    /// The source, and the end that flows are put into.
    pub fn channel() -> (Self, mpsc::Sender<Flow>) {
        let (tx, rx) = mpsc::channel(WAITING_MAX);
        (Self(rx), tx)
    }
}

impl Source for Given {
    fn name(&self) -> String {
        "taken connections".to_owned()
    }

    fn by(&self) -> By {
        By::Given
    }

    fn start(self: Box<Self>, _proxy: SocketAddr, found: Found, stop: Stop) -> Result<()> {
        let mut flows = self.0;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    given = flows.recv() => match given {
                        Some(flow) => found.flow(flow).await,
                        None => break,
                    },
                    _ = stopped(stop.clone()) => break,
                }
            }
        });
        Ok(())
    }
}

/// Other devices on the network, while they are let in.
///
/// The proxy always listens on this machine's loopback. For other devices it
/// listens, on the same port, on each address this machine has on its networks
/// too: listening on them all at once (`0.0.0.0`) would clash with the
/// loopback's socket. Those addresses come and go as Wi-Fi changes, so they
/// are looked at again every few seconds while the switch is on, which takes
/// tens of microseconds. An address that goes has its listener stopped; the
/// connections it took are left to end on their own, as a phone's may be in
/// the middle of something.
pub struct Lan(Arc<OtherDevices>);

impl Lan {
    pub fn new(devices: Arc<OtherDevices>) -> Self {
        Self(devices)
    }
}

impl Source for Lan {
    fn name(&self) -> String {
        "other devices".to_owned()
    }

    fn by(&self) -> By {
        By::OtherDevices
    }

    fn start(self: Box<Self>, proxy: SocketAddr, found: Found, stop: Stop) -> Result<()> {
        tokio::spawn(watch_addresses(self.0, proxy.port(), found, stop));
        Ok(())
    }
}

/// Other devices reach the proxy on the port the loopback does.
async fn watch_addresses(devices: Arc<OtherDevices>, port: u16, found: Found, mut stop: Stop) {
    // A sender per address: dropping it stops that address's listener.
    let mut running: std::collections::BTreeMap<std::net::Ipv4Addr, oneshot::Sender<()>> =
        std::collections::BTreeMap::new();
    loop {
        let wanted: Vec<std::net::Ipv4Addr> = if devices.is_on() {
            crate::lan::addresses()
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
                    let (stop_one, until) = oneshot::channel();
                    found
                        .port(Port {
                            listener,
                            name: format!("other devices at {ip}"),
                            until: Some(until),
                            essential: false,
                        })
                        .await;
                    running.insert(ip, stop_one);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_a_connect_says() {
        assert_eq!(
            Target::Addr("140.82.121.4:443".parse().unwrap()).authority(),
            "140.82.121.4:443"
        );
        assert_eq!(
            Target::Named {
                host: "github.com".to_owned(),
                port: 443
            }
            .authority(),
            "github.com:443"
        );
    }

    /// What the window actually asks: not "how many" but "anything since
    /// then". A bare count cannot answer it — the strongest true sentence a
    /// count supports is "nothing has ever arrived", which is wrong for
    /// anybody who used that way in last week and useless to somebody who
    /// has just flipped a switch.
    #[test]
    fn a_count_from_before_is_not_an_answer_about_since() {
        let arrivals = Arrivals::default();
        assert_eq!(arrivals.of(By::Socks5), Arrival::default());
        assert!(!arrivals.anything_since(By::Socks5, Instant::now()));

        arrivals.came(By::Socks5);
        // A moment strictly after that arrival, built from the arrival
        // rather than read from the clock. `Instant::now()` in the next
        // statement can come back equal on a coarse timer — a virtualized
        // one on a Windows runner does — and `anything_since` is `last >=
        // since`, which is the semantics wanted, so an equal pair makes the
        // old arrival look like a new one. This depends on no clock at all.
        let after = arrivals.of(By::Socks5).last.expect("it came") + Duration::from_nanos(1);
        assert_eq!(arrivals.of(By::Socks5).count, 1);
        assert!(arrivals.of(By::Socks5).last.is_some());

        // The switch went on after that arrival: the honest answer is no.
        assert!(!arrivals.anything_since(By::Socks5, after));
        arrivals.came(By::Socks5);
        assert!(arrivals.anything_since(By::Socks5, after), "now there is");
        assert_eq!(arrivals.of(By::Socks5).count, 2);
    }

    /// Each way in keeps its own count, which is the point: a browser on the
    /// proxy port must not make the WireGuard tab look busy.
    #[test]
    fn the_ways_in_are_counted_apart() {
        let arrivals = Arrivals::default();
        arrivals.came(By::ProxyPort);
        arrivals.came(By::ProxyPort);
        arrivals.came(By::WireGuard);
        assert_eq!(arrivals.of(By::ProxyPort).count, 2);
        assert_eq!(arrivals.of(By::WireGuard).count, 1);
        for by in By::ALL {
            if !matches!(by, By::ProxyPort | By::WireGuard) {
                assert_eq!(arrivals.of(by).count, 0, "{by:?}");
            }
        }
    }

    #[tokio::test]
    async fn the_proxy_port_hands_over_its_listener_once() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (found, mut took) = Found::channel();
        let (_tx, stop) = watch::channel(false);
        found
            .start(Box::new(ProxyPort::new(listener)), addr, stop)
            .unwrap();
        let Some(Took::Port(port, by)) = took.recv().await else {
            panic!("no listener");
        };
        assert_eq!(by, By::ProxyPort, "stamped by the channel, not by itself");
        assert_eq!(port.listener.local_addr().unwrap(), addr);
        assert!(port.essential, "the proxy is pointless without it");
        assert!(port.until.is_none(), "it lasts as long as the proxy");
    }

    /// Switched off, it opens nothing and says it listens nowhere.
    #[tokio::test]
    async fn other_devices_kept_out_listen_nowhere() {
        let devices = Arc::new(OtherDevices::new(false));
        let (found, mut took) = Found::channel();
        let (stop_tx, stop) = watch::channel(false);
        found
            .start(
                Box::new(Lan::new(Arc::clone(&devices))),
                "127.0.0.1:9090".parse().unwrap(),
                stop,
            )
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(devices.listening().is_empty());
        assert!(took.try_recv().is_err(), "nothing to serve");
        let _ = stop_tx.send(true);
    }
}
