//! Where traffic comes from.
//!
//! A source brings connections to tracewing. Today both of them are of the
//! same kind: something was told to use the proxy — the system proxy setting,
//! `HTTP_PROXY` in a terminal, a phone's Wi-Fi settings — and it connects and
//! says where it wants to go. The sources that are to come take connections
//! the OS hands over, from apps that were never told anything and may not know
//! a proxy exists. See [capture-modes.md](../../../docs/design/capture-modes.md).
//!
//! What any source has to deliver is the same two things: where the connection
//! was going, and who sent it. A client that asked says the first itself; a
//! connection taken on its way did not, so its source has to know. The second
//! is looked up by address for the ones that asked ([`crate::apps`]), and known
//! in advance by a source that took the connection from the OS.

use crate::lan::OtherDevices;
use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};
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

/// Something that brings traffic to tracewing.
pub trait Source: Send + 'static {
    /// What it is, for the logs.
    fn name(&self) -> String;

    /// Starts it, and returns once it is running rather than once it is done.
    /// What it finds goes to `found`; it stops when `stop` is set.
    ///
    /// `proxy` is where clients reach tracewing on this machine: the port a
    /// taken connection is handed to, and the one a source has to leave out
    /// of what it captures, or it will capture tracewing itself.
    fn start(self: Box<Self>, proxy: SocketAddr, found: Found, stop: Stop) -> Result<()>;
}

/// What a source found: either a way for clients to reach the proxy, or one
/// connection that is already on its way somewhere.
pub enum Took {
    Port(Port),
    Flow(Flow),
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

/// Where a source hands over what it found.
#[derive(Clone)]
pub struct Found(mpsc::Sender<Took>);

impl Found {
    /// The channel a source sends to, and the end the proxy serves from.
    pub fn channel() -> (Self, mpsc::Receiver<Took>) {
        let (tx, rx) = mpsc::channel(WAITING_MAX);
        (Self(tx), rx)
    }

    /// Hands over a listener. Returns once it has been taken, or at once if
    /// the proxy has stopped, which is not a source's problem to report.
    pub async fn port(&self, port: Port) {
        let _ = self.0.send(Took::Port(port)).await;
    }

    /// Hands over one connection.
    pub async fn flow(&self, flow: Flow) {
        let _ = self.0.send(Took::Flow(flow)).await;
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

    #[tokio::test]
    async fn the_proxy_port_hands_over_its_listener_once() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (found, mut took) = Found::channel();
        let (_tx, stop) = watch::channel(false);
        Box::new(ProxyPort::new(listener))
            .start(addr, found, stop)
            .unwrap();
        let Some(Took::Port(port)) = took.recv().await else {
            panic!("no listener");
        };
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
        Box::new(Lan::new(Arc::clone(&devices)))
            .start("127.0.0.1:9090".parse().unwrap(), found, stop)
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(devices.listening().is_empty());
        assert!(took.try_recv().is_err(), "nothing to serve");
        let _ = stop_tx.send(true);
    }
}
