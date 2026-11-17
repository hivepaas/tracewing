//! A TUN device feeding the stack: packets in, flows out.
//!
//! [`crate::stack`] turns packets into flows and needs somebody to hand it
//! packets. [`crate::wireguard`] is the first such **device**, and what it
//! reaches is a phone. This is the shape of the devices that reach *this*
//! machine: an adapter the operating system routes traffic into — Wintun on
//! Windows (`crate::wintun`), `/dev/net/tun` on Linux — so that a program
//! is captured without ever being told to use a proxy.
//!
//! Only the pump is here. Opening an adapter, addressing it and routing to
//! it are platform code and live with their platform; what crosses between a
//! blocking device and the stack's channels is the same everywhere, and this
//! is it.
//!
//! ## Why there is a trait here, and not just the Windows code
//!
//! Because none of the platform code can be built or run by the continuous
//! integration that sees every change, which is Linux. The pump can be, and
//! the pump is where the bugs with teeth are. Three of them, each with a test
//! below:
//!
//! - **A stop that hangs.** Reading a TUN device blocks until the operating
//!   system has a packet, and on a quiet machine that is for ever. Only the
//!   device can end that wait ([`Device::shutdown`]), and forgetting to ask
//!   gives a tracewing that will not quit — worse than one that fails, and
//!   invisible until somebody tries to close the window.
//! - **One packet ending the capture.** A full ring is an ordinary event
//!   under load, and the packet is the operating system's to send again. A
//!   write that fails has to drop it and go on.
//! - **A read and a write starving each other.** Both block, so each gets a
//!   thread of its own, which is why [`Device`] takes `&self` throughout and
//!   the pump holds it in an [`Arc`].
//!
//! The two are real threads rather than `spawn_blocking` tasks: they block
//! for as long as the capture runs, and the blocking pool is sized for work
//! that ends.
//!
//! See [capture-modes.md](../../../docs/design/capture-modes.md).

use crate::{
    datagrams::Bind,
    icmp::Echo,
    sieve::Sieve,
    source::{Found, Source, Stop, stopped},
    stack::{Stack, Wire},
};
use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, thread};
use tokio::sync::mpsc;

/// An adapter the operating system hands packets to, and takes packets from.
///
/// Every method takes `&self` because reading and writing both block and so
/// run on threads of their own. Implementations are platform code:
/// `crate::wintun` on Windows and `crate::tun_linux` on Linux.
pub trait Device: Send + Sync + 'static {
    /// What this way in is called, where a flow says where it came from.
    fn name(&self) -> String;

    /// Blocks until the operating system has a packet, and hands it over.
    ///
    /// An error ends the capture: a device that cannot be read is gone, and
    /// there is nothing to retry. [`Device::shutdown`] is how a blocked call
    /// is made to return one.
    fn read(&self) -> Result<Vec<u8>>;

    /// Hands a packet back to the operating system.
    ///
    /// An error is about this packet alone — a full ring under load is the
    /// ordinary one — so the pump drops it and goes on. Anything that is
    /// really the end of the device will be said by [`Device::read`] too.
    fn write(&self, packet: &[u8]) -> Result<()>;

    /// Makes a blocked [`Device::read`] return, so its thread can end.
    /// Called once, when the proxy stops.
    fn shutdown(&self);
}

/// A way in: a device, and the stack that turns what it takes into flows.
/// One type, because the two are no use apart — a device with nothing to
/// read its packets, or a stack with nothing to feed it. The same reason
/// [`crate::wireguard::Endpoint`] holds both.
pub struct Tun<D> {
    device: D,
    /// Which connections the proxy is to see, and the way out for the rest.
    sieve: Sieve,
    /// How a socket to the far side is made for a datagram that is passed on.
    datagrams: Bind,
    /// Where an echo leaves from, for `ping` and `tracert`.
    echo: Option<Echo>,
}

impl<D: Device> Tun<D> {
    /// **The sieve and the bind are arguments rather than builders**, because
    /// a mode that captures by *route* has no safe default for either and
    /// both mistakes are quiet ones.
    ///
    /// Without a sieve the stack keeps every connection on the machine and
    /// offers tracewing's certificate to all of them — which breaks programs
    /// that pin their own, for traffic nobody asked to see. Without a bind,
    /// every datagram the sieve does not keep is refused instead of passed
    /// on, so a captured machine loses the UDP it did not choose to show.
    /// Neither fails loudly, and neither is visible in a test that only asks
    /// whether a flow arrived.
    ///
    /// A tunnel a phone joined is the other case and is why
    /// [`crate::stack::Stack`] keeps both optional: everything down it was
    /// sent on purpose, so there is nothing to sift and nothing to pass.
    pub fn new(device: D, sieve: Sieve, datagrams: Bind) -> Self {
        Self {
            device,
            sieve,
            datagrams,
            echo: None,
        }
    }

    /// Answers `ping` and `tracert` by really sending the echo, rather than
    /// leaving them silent. Optional because silence is a feature missing
    /// rather than traffic going somewhere it should not.
    pub fn answering(self, echo: Echo) -> Self {
        Self {
            echo: Some(echo),
            ..self
        }
    }
}

impl<D: Device> Source for Tun<D> {
    fn name(&self) -> String {
        self.device.name()
    }

    fn by(&self) -> crate::source::By {
        crate::source::By::Tun
    }

    // `own_ports` stays empty: a program caught this way never connected to a
    // port of tracewing's, it was talking to the server it was going to, so
    // there is no socket of ours for `crate::apps` to find it by. The stack
    // and the WireGuard endpoint say the same, for the same reason.

    fn start(self: Box<Self>, proxy: SocketAddr, found: Found, stop: Stop) -> Result<()> {
        let Self {
            device,
            sieve,
            datagrams,
            echo,
        } = *self;
        let (wire, into_stack, from_stack) = Wire::channels();
        let mut stack = Stack::new(wire).sifting(sieve).passing(datagrams);
        if let Some(echo) = echo {
            stack = stack.answering(echo);
        }
        // The stack first: it is what `found` hears from, and a packet that
        // arrived before it was listening would have nowhere to go.
        Box::new(stack).start(proxy, found, stop.clone())?;
        run(Arc::new(device), into_stack, from_stack, stop);
        Ok(())
    }
}

/// Pumps `device` both ways until the proxy stops or the device ends, and
/// returns once the threads are running.
///
/// The two ends are the channels [`Wire::channels`] hands out, so this knows
/// nothing about either side: what the stack makes of the packets is flows,
/// handed to the proxy as though a client had asked.
///
/// **The stop is a cascade, and each link is load-bearing.** Asking the
/// device to shut down is what ends the reading thread; that thread dropping
/// its end of `into_stack` is what ends the stack, which breaks its loop when
/// the channel closes; the stack dropping its end of `from_stack` is what
/// ends the writing thread. Nothing here polls the stop but the one task that
/// asks the device, because a thread blocked in the operating system cannot
/// poll anything.
///
/// [`Wire::channels`]: crate::stack::Wire::channels
pub fn run<D: Device>(
    device: Arc<D>,
    into_stack: mpsc::Sender<Vec<u8>>,
    mut from_stack: mpsc::Receiver<Vec<u8>>,
    stop: Stop,
) {
    let reading = Arc::clone(&device);
    thread::spawn(move || {
        loop {
            match reading.read() {
                // `blocking_send` and not `try_send`: a stack slower than the
                // wire has to be waited for here. The alternative is dropping
                // the packet, and the device's own ring already does that
                // when this thread stops taking them — which is the drop the
                // operating system knows how to tell the sender about.
                Ok(packet) => {
                    if into_stack.blocking_send(packet).is_err() {
                        return; // The stack has gone.
                    }
                }
                // Expected at every stop, which is why this is not a warning:
                // `shutdown` ends the blocked read by making it fail.
                Err(e) => {
                    tracing::debug!("the device has ended: {e:#}");
                    return;
                }
            }
        }
    });
    let writing = Arc::clone(&device);
    thread::spawn(move || {
        while let Some(packet) = from_stack.blocking_recv() {
            // Also not a warning: a full ring is what this says under load,
            // and it is the ordinary case rather than a fault.
            if let Err(e) = writing.write(&packet) {
                tracing::debug!("a packet the device would not take: {e:#}");
            }
        }
    });
    tokio::spawn(async move {
        stopped(stop).await;
        device.shutdown();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::VecDeque,
        sync::{Condvar, Mutex},
        time::Duration,
    };
    use tokio::{sync::watch, time::timeout};

    /// Long enough that a loaded machine does not fail a test, short enough
    /// that a pump which will never answer is a failure and not a hang.
    const SOON: Duration = Duration::from_secs(5);

    #[derive(Default)]
    struct State {
        /// What `read` hands over, in order; `None` is an error instead.
        reads: VecDeque<Option<Vec<u8>>>,
        written: Vec<Vec<u8>>,
        /// How many of the next writes refuse their packet.
        refuse: usize,
        shut: bool,
    }

    /// A device with no operating system under it: the packets it will read
    /// are given to it, and what is written to it is kept.
    #[derive(Default)]
    struct Fake {
        state: Mutex<State>,
        woken: Condvar,
    }

    impl Fake {
        fn will_read(&self, packet: Vec<u8>) {
            self.state.lock().unwrap().reads.push_back(Some(packet));
            self.woken.notify_all();
        }

        fn will_fail_to_read(&self) {
            self.state.lock().unwrap().reads.push_back(None);
            self.woken.notify_all();
        }

        fn will_refuse_writes(&self, how_many: usize) {
            self.state.lock().unwrap().refuse = how_many;
        }

        fn written(&self) -> Vec<Vec<u8>> {
            self.state.lock().unwrap().written.clone()
        }

        fn was_shut_down(&self) -> bool {
            self.state.lock().unwrap().shut
        }
    }

    impl Device for Fake {
        fn name(&self) -> String {
            "fake".to_owned()
        }

        fn read(&self) -> Result<Vec<u8>> {
            let mut state = self.state.lock().unwrap();
            // Blocks where a real device blocks: until there is a packet, or
            // until the shutdown that is the only other way out of here.
            while state.reads.is_empty() && !state.shut {
                state = self.woken.wait(state).unwrap();
            }
            match state.reads.pop_front() {
                Some(Some(packet)) => Ok(packet),
                Some(None) => anyhow::bail!("this device was told to fail"),
                None => anyhow::bail!("this device was shut down"),
            }
        }

        fn write(&self, packet: &[u8]) -> Result<()> {
            let mut state = self.state.lock().unwrap();
            if state.refuse > 0 {
                state.refuse -= 1;
                anyhow::bail!("the ring is full");
            }
            state.written.push(packet.to_vec());
            Ok(())
        }

        fn shutdown(&self) {
            self.state.lock().unwrap().shut = true;
            self.woken.notify_all();
        }
    }

    /// A pump running over a [`Fake`], with the ends the stack would hold.
    struct Running {
        fake: Arc<Fake>,
        into_stack: mpsc::Receiver<Vec<u8>>,
        from_stack: mpsc::Sender<Vec<u8>>,
        /// Kept alive: dropping it would look like a stop to the pump.
        stop: watch::Sender<bool>,
    }

    fn pump() -> Running {
        let fake = Arc::new(Fake::default());
        let (into_tx, into_rx) = mpsc::channel(8);
        let (out_tx, out_rx) = mpsc::channel(8);
        let (stop_tx, stop) = watch::channel(false);
        run(Arc::clone(&fake), into_tx, out_rx, stop);
        Running {
            fake,
            into_stack: into_rx,
            from_stack: out_tx,
            stop: stop_tx,
        }
    }

    /// Waits for the writes to be `want`. A thread does the writing, so
    /// there is nothing here to await.
    async fn written_becomes(fake: &Fake, want: &[Vec<u8>]) {
        let until = tokio::time::Instant::now() + SOON;
        loop {
            let written = fake.written();
            if written == want {
                return;
            }
            assert!(
                tokio::time::Instant::now() < until,
                "the device was written {written:?}, wanted {want:?}"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn a_packet_the_device_reads_reaches_the_stack() {
        let mut running = pump();
        running.fake.will_read(vec![4, 5, 6]);
        let got = timeout(SOON, running.into_stack.recv())
            .await
            .expect("the packet reached the stack");
        assert_eq!(got, Some(vec![4, 5, 6]));
    }

    #[tokio::test]
    async fn a_packet_from_the_stack_is_written_to_the_device() {
        let running = pump();
        running
            .from_stack
            .send(vec![7, 8])
            .await
            .expect("the writing thread is there");
        written_becomes(&running.fake, &[vec![7, 8]]).await;
    }

    /// The hang this module exists to prevent. Nothing is queued, so the
    /// reader is where a real one spends its life: blocked on an operating
    /// system with nothing to say.
    #[tokio::test]
    async fn a_stop_ends_the_reader_rather_than_waiting_for_a_packet() {
        let mut running = pump();
        let _ = running.stop.send(true);
        // The reading thread dropping its end is what closes this.
        let ended = timeout(SOON, running.into_stack.recv())
            .await
            .expect("the reading thread ended");
        assert_eq!(ended, None, "the end, and no packet with it");
        assert!(
            running.fake.was_shut_down(),
            "the device was asked to end the read"
        );
    }

    #[tokio::test]
    async fn a_write_the_device_refuses_drops_that_packet_alone() {
        let running = pump();
        running.fake.will_refuse_writes(1);
        for packet in [vec![1], vec![2]] {
            running
                .from_stack
                .send(packet)
                .await
                .expect("the writing thread is there");
        }
        written_becomes(&running.fake, &[vec![2]]).await;
    }

    #[tokio::test]
    async fn a_device_that_cannot_be_read_ends_the_pump() {
        let mut running = pump();
        running.fake.will_fail_to_read();
        let ended = timeout(SOON, running.into_stack.recv())
            .await
            .expect("the reading thread ended");
        assert_eq!(ended, None);
    }
    /// What a `Tun` is given has to reach the stack, and these are the three
    /// that are quiet when it does not. Built through `Tun::start` rather
    /// than by handing a `Stack` the same arguments, because the wiring is
    /// what is being tested.
    ///
    /// The proxy address is one nothing listens on, as `stack.rs`'s own
    /// tests use: a flow is handed over when the connection is taken, not
    /// when the proxy answers.
    const NOWHERE: &str = "127.0.0.1:9090";

    /// Starts a `Tun` over a fake device, and hands back what the test
    /// watches: the flows, what the sieve was asked, the far end of anything
    /// it sent straight out, and whether a datagram asked for a socket.
    fn start_tun(keeps: bool) -> (Arc<Fake>, Watching) {
        let (found, took) = Found::channel();
        let (stop_tx, stop) = watch::channel(false);
        let (asked_tx, asked) = mpsc::channel(8);
        let (out_tx, out) = mpsc::channel(8);
        let (bound_tx, bound) = mpsc::channel(8);
        let sieve = Sieve::new(
            Box::new(move |client, to| {
                let asked = asked_tx.clone();
                Box::pin(async move {
                    let _ = asked.send((client, to)).await;
                    keeps
                })
            }),
            Box::new(move |to| {
                let out = out_tx.clone();
                Box::pin(async move {
                    let (ours, theirs) = tokio::io::duplex(1024);
                    let _ = out.send((to, theirs)).await;
                    Ok(Box::new(ours) as Box<dyn crate::source::Duplex>)
                })
            }),
        );
        let datagrams: Bind = Arc::new(move |v6| {
            let bound = bound_tx.clone();
            Box::pin(async move {
                let _ = bound.send(v6).await;
                tokio::net::UdpSocket::bind("127.0.0.1:0").await
            })
        });
        let device = Arc::new(Fake::default());
        // Through `Found::start` and not `Source::start`, because that is
        // where a way in is stamped on what a source hands over -- so this
        // sees what the proxy sees, including whether `Tun`'s own `by` is
        // what reaches a flow or whether the stack inside overwrites it.
        found
            .start(
                Box::new(Tun::new(Held(Arc::clone(&device)), sieve, datagrams)),
                NOWHERE.parse().unwrap(),
                stop,
            )
            .expect("the source starts");
        (
            device,
            Watching {
                took,
                asked,
                out,
                bound,
                _stop: stop_tx,
            },
        )
    }

    struct Watching {
        took: mpsc::Receiver<crate::source::Took>,
        asked: mpsc::Receiver<(SocketAddr, SocketAddr)>,
        out: mpsc::Receiver<(SocketAddr, tokio::io::DuplexStream)>,
        bound: mpsc::Receiver<bool>,
        _stop: watch::Sender<bool>,
    }

    /// The `Device` the pump owns is the fake the test still holds, which is
    /// what this stands in for: `Tun` takes a device by value and the test
    /// needs it afterwards.
    struct Held(Arc<Fake>);

    impl Device for Held {
        fn name(&self) -> String {
            self.0.name()
        }
        fn read(&self) -> Result<Vec<u8>> {
            self.0.read()
        }
        fn write(&self, packet: &[u8]) -> Result<()> {
            self.0.write(packet)
        }
        fn shutdown(&self) {
            self.0.shutdown();
        }
    }

    /// The three-way handshake, through the device rather than through a
    /// channel: the stack writes its SYN-ACK to the device like any other
    /// packet, so the test reads it back out of the fake. A connection is
    /// not a connection until this is done, which is why both tests below
    /// need it -- the sieve is asked once the handshake is complete, not
    /// when the SYN arrives.
    async fn shake_hands(device: &Fake, client: &mut crate::stack::Client) {
        device.will_read(client.send(smoltcp::wire::TcpControl::Syn, &[]));
        let answer = next_written(device, 0).await;
        let (control, _) = client.heard(&answer);
        assert_eq!(
            control,
            smoltcp::wire::TcpControl::Syn,
            "a SYN-ACK answers the SYN"
        );
        device.will_read(client.send(smoltcp::wire::TcpControl::None, &[]));
    }

    /// The packet after `already`, waited for rather than looked for once: a
    /// stack that answers nothing must fail this test, not hang the suite.
    async fn next_written(device: &Fake, already: usize) -> Vec<u8> {
        let deadline = tokio::time::Instant::now() + SOON;
        loop {
            let written = device.written();
            if written.len() > already {
                return written[already].clone();
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the stack wrote nothing back"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    /// The sieve reaches the stack, and a connection it keeps is a flow that
    /// says it came in this way.
    #[tokio::test]
    async fn what_the_sieve_keeps_becomes_a_flow_from_this_way_in() {
        let (device, mut watching) = start_tun(true);
        let mut client = crate::stack::Client::new("10.77.0.9:40000", "93.184.216.34:443");
        shake_hands(&device, &mut client).await;
        let took = timeout(SOON, watching.took.recv())
            .await
            .expect("a flow before the timeout")
            .expect("the stack is running");
        match took {
            crate::source::Took::Flow(_, by) => assert_eq!(by, crate::source::By::Tun),
            crate::source::Took::Port(..) => panic!("a TUN takes flows, not ports"),
        }
        let (_, to) = timeout(SOON, watching.asked.recv())
            .await
            .expect("the sieve is asked")
            .expect("the channel is open");
        assert_eq!(to.to_string(), "93.184.216.34:443");
    }

    /// And the way out reaches it: a connection the sieve does not keep is
    /// connected straight to where it was going, and is never a flow.
    #[tokio::test]
    async fn what_the_sieve_refuses_goes_straight_out_and_is_never_a_flow() {
        let (device, mut watching) = start_tun(false);
        let mut client = crate::stack::Client::new("10.77.0.9:40001", "93.184.216.34:443");
        shake_hands(&device, &mut client).await;
        let (to, _far) = timeout(SOON, watching.out.recv())
            .await
            .expect("the way out is asked")
            .expect("the channel is open");
        assert_eq!(to.to_string(), "93.184.216.34:443");
        // And nothing was handed to the proxy. `try_recv` rather than a
        // second timeout: the flow would already be there if it were coming,
        // since the connection above has been made.
        assert!(
            watching.took.try_recv().is_err(),
            "a connection nobody chose is not tracewing's to see"
        );
    }

    /// The bind reaches it too, which is the one of the three with no
    /// symptom at all when it does not: a datagram would simply be refused,
    /// and a captured machine would lose the UDP nobody chose to show.
    #[tokio::test]
    async fn a_datagram_nobody_chose_asks_for_a_socket_rather_than_being_refused() {
        let (device, mut watching) = start_tun(false);
        let packet = crate::datagrams::packet_for(
            "10.77.0.9:40002".parse().unwrap(),
            "93.184.216.34:7777".parse().unwrap(),
            b"a datagram",
        )
        .expect("a packet");
        device.will_read(packet);
        let v6 = timeout(SOON, watching.bound.recv())
            .await
            .expect("a socket is asked for")
            .expect("the channel is open");
        assert!(!v6, "an IPv4 datagram asks for an IPv4 socket");
    }
}
