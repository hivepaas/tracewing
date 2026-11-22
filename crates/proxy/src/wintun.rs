//! Windows: a Wintun adapter, which is a [`crate::tun::Device`].
//!
//! Wintun is WireGuard's layer-3 TUN driver for Windows. The operating
//! system routes packets into the adapter, Wintun hands them over through a
//! ring buffer, and what comes out is what [`crate::stack`] takes: IP
//! packets, which it turns into flows.
//!
//! ## Why this rather than WinDivert
//!
//! [capture-modes.md](../../../docs/design/capture-modes.md) planned
//! WinDivert, and the reasons against it are not about its code:
//!
//! - **There is no signed ARM64 driver, and nobody has agreed to make one.**
//!   On a Windows-on-ARM machine that capture mode would not exist at all.
//!   Wintun ships signed binaries for both x86-64 and ARM64.
//! - **Its signing has no owner.** The project froze v2.2.2 as its long-term
//!   version in 2022 and says it will change only for security; the attempt
//!   to get the driver signed again was abandoned in 2026 after a
//!   certificate expired mid-process. Wintun's signature is kept alive
//!   because WireGuard for Windows ships it, which is somebody else's reason
//!   and does not depend on a favour.
//! - **Antivirus does block it.** BitDefender quarantined `WinDivert64.sys`
//!   in 2025 with a clean VirusTotal. Wintun's driver is already installed on
//!   every machine that runs WireGuard.
//!
//! What is given up is real and belongs here rather than in a commit
//! message: WinDivert takes packets **by process**, leaving the rest of the
//! machine alone, and hands over the owning process with them. A TUN adapter
//! takes whatever is routed into it. Choosing one program is then a decision
//! made in tracewing after the packet is already ours, and the program
//! behind a flow has to be looked up by its address — which is what
//! [`crate::apps`] does for every other way in.
//!
//! ## What is here and what is not
//!
//! Here: the adapter, its session, and the two halves of
//! [`crate::tun::Device`] over Wintun's ring.
//!
//! Not here, and nothing captures anything until it is: **an address on the
//! adapter and a route into it**, and the matching exclusion that keeps
//! tracewing's own connections out of the adapter it is reading. Without the
//! second, every flow tracewing makes on a captured program's behalf is
//! routed back into tracewing. That is the dangerous half, and it is a
//! change in the outbound path rather than here, so it is its own piece of
//! work.
//!
//! The address setters the `wintun` crate offers are deliberately unused:
//! they shell out to `netsh`. [`WintunAdapter::address`] does it through
//! `windows-sys` instead, with the rest of this program's Windows calls, and
//! [`WintunAdapter::route`] the same for routing.

use crate::tun::Device;
use anyhow::{Result, anyhow};
use std::{net::IpAddr, path::Path, sync::Arc};

/// How much of the ring Wintun keeps for packets on their way in. It takes a
/// power of two, and this is the `0x400000` Wintun's own README passes to
/// `WintunStartSession` in its example: large enough to hold a burst while
/// the stack is busy, without taking memory for the whole life of a capture.
const RING_BYTES: u32 = 4 * 1024 * 1024;

/// What Windows calls the adapter, and the kind it says it is. Both are shown
/// in the network settings, so they say tracewing rather than anything about
/// tunnels.
const ADAPTER_NAME: &str = "tracewing";
const ADAPTER_TYPE: &str = "tracewing";

/// The adapter's identity, fixed rather than fresh each time.
///
/// Windows keys a network profile — and the firewall answers that go with it
/// — on the adapter's GUID. A new one on every run would mean a new
/// "unidentified network", and a new set of prompts, every time tracewing
/// started. Generated once; changing it is changing the adapter's identity on
/// every machine that has already answered for it.
const ADAPTER_GUID: u128 = 0x9f2b_4d61_8e03_47a5_b1c6_2f7d_0a93_5e48;

/// An open Wintun adapter, with a session running on it.
pub struct WintunAdapter {
    /// The field order is the drop order, and it is the point: a session must
    /// not outlive the adapter it came from, and neither may outlive the
    /// library whose code they are still inside. The `wintun` crate may well
    /// hold its own references for both — that could not be verified from its
    /// documentation, so these are held rather than assumed. The adapter is
    /// also read, by [`WintunAdapter::luid`].
    session: Arc<::wintun::Session>,
    adapter: Arc<::wintun::Adapter>,
    _library: ::wintun::Wintun,
}

impl WintunAdapter {
    /// Loads Wintun from `dll` and creates the adapter.
    ///
    /// Needs Administrator: Wintun installs its driver on the first call and
    /// fails without it.
    pub fn open(dll: &Path) -> Result<Self> {
        // SAFETY: this loads and runs the code in `dll`. The caller passes a
        // path — ours, beside our own executable — rather than a name for
        // Windows to go looking for, because a `wintun.dll` found on the
        // search path would be somebody else's choice of code to run with
        // Administrator behind it.
        let library = unsafe { ::wintun::load_from_path(dll) }
            .map_err(|e| anyhow!("load Wintun from {}: {e}", dll.display()))?;
        // The errors are carried by hand rather than with `?`: whether
        // `wintun::Error` is one `anyhow` will take could not be checked
        // without a Windows machine to build on, and a message loses nothing.
        let adapter =
            ::wintun::Adapter::create(&library, ADAPTER_NAME, ADAPTER_TYPE, Some(ADAPTER_GUID))
                .map_err(|e| {
                    // Three causes, said rather than one guessed at. The
                    // third was measured: in a session already elevated at
                    // High integrity, with the DLL where it should be, this
                    // failed because another process held an adapter of this
                    // name -- Wintun reports that the same way as a missing
                    // privilege, and a reader told only "needs
                    // Administrator" goes hunting for elevation they have.
                    //
                    // Not branched on the OS error code, because which code
                    // each cause leaves has not been measured, and a mapping
                    // guessed from the docs would put a confident wrong
                    // sentence in front of somebody.
                    anyhow!(
                        "create the {ADAPTER_NAME} adapter: {e}. Wintun needs Administrator \
                         and wintun.dll beside the program, and it also fails while another \
                         program is holding an adapter of this name"
                    )
                })?;
        let session = Arc::new(
            adapter
                .start_session(RING_BYTES)
                .map_err(|e| anyhow!("start a Wintun session: {e}"))?,
        );
        Ok(Self {
            session,
            adapter,
            _library: library,
        })
    }
}

impl WintunAdapter {
    /// What Windows **identifies** this adapter by, whatever the family.
    ///
    /// [`crate::roads::out_by`] leaves our own adapter out of the ways out
    /// by this and not by an interface index, because an index belongs to
    /// one family: `Ipv6IfIndex` is documented as zero where an interface
    /// has no IPv6, and neither index is promised to survive an adapter
    /// being disabled and enabled. On the machine this was tried on the two
    /// agreed, which is the common case and not a guarantee — and the cost
    /// of being wrong is that one family is not excluded, and that family's
    /// connections loop.
    ///
    /// Taken from the index the driver reports and converted, which is sound
    /// in the direction it is used: an index names one interface, whichever
    /// family numbered it, so the LUID that comes back is this adapter's.
    pub fn luid(&self) -> Result<u64> {
        use windows_sys::Win32::{
            Foundation::NO_ERROR, NetworkManagement::IpHelper::ConvertInterfaceIndexToLuid,
            NetworkManagement::Ndis::NET_LUID_LH,
        };
        let index = self
            .adapter
            .get_adapter_index()
            .map_err(|e| anyhow!("ask Wintun for the adapter's interface index: {e}"))?;
        let mut luid = NET_LUID_LH::default();
        // SAFETY: an index in, and eight bytes of ours to fill in.
        let got = unsafe { ConvertInterfaceIndexToLuid(index, &mut luid) };
        if got != NO_ERROR {
            anyhow::bail!("turn interface {index} into a LUID: error {got}");
        }
        // SAFETY: a union over a `u64` and a bitfield of those same bytes;
        // the whole value is what identifies the interface.
        Ok(unsafe { luid.Value })
    }

    /// Gives the adapter `addr`, with `bits` of it on-link, until the
    /// returned [`Address`] is dropped.
    ///
    /// Which address is the caller's to choose: nothing here has an opinion,
    /// because the stack behind the adapter answers for every destination
    /// and this address is only what makes the adapter a place Windows will
    /// route to at all.
    pub fn address(self: &Arc<Self>, addr: IpAddr, bits: u8) -> Result<Address> {
        use windows_sys::Win32::{
            Foundation::NO_ERROR,
            NetworkManagement::IpHelper::{
                CreateUnicastIpAddressEntry, InitializeUnicastIpAddressEntry,
                MIB_UNICASTIPADDRESS_ROW,
            },
            NetworkManagement::Ndis::NET_LUID_LH,
            Networking::WinSock::IpDadStatePreferred,
        };
        let mut row = MIB_UNICASTIPADDRESS_ROW::default();
        // Defaults first, and not for tidiness: this is what sets the
        // lifetimes to infinite. A zeroed row means a lifetime of zero, which
        // is an address that expires as soon as it is made.
        // SAFETY: a row of ours for the call to fill in.
        unsafe { InitializeUnicastIpAddressEntry(&mut row) };
        // **Preferred at once, which the defaults above do not make it**,
        // though this comment once said they did. Left to them, the address
        // starts `Tentative` while Windows checks nobody else has it, and
        // for as long as it is, nothing routes into the adapter: measured in
        // the VM, `Find-NetRoute` found no way to 203.0.113.9 for the two
        // seconds after the adapter came up, and curl's connect failed in
        // 0 ms (`local_live`'s S8 test): what the `/1` routes take went
        // nowhere for at least that long, chosen or not. There is nobody else
        // on this link to ask: the adapter is ours alone.
        row.DadState = IpDadStatePreferred;
        // By LUID rather than by index, for the reason `luid` gives: an
        // index belongs to one family and this address has its own.
        row.InterfaceLuid = NET_LUID_LH {
            Value: self.luid()?,
        };
        row.Address = sockaddr(addr);
        row.OnLinkPrefixLength = bits;
        // SAFETY: a row this call reads and does not keep.
        let got = unsafe { CreateUnicastIpAddressEntry(&row) };
        match got {
            NO_ERROR => Ok(Address {
                row,
                _held: Arc::clone(self),
            }),
            _ => Err(anyhow!("give the adapter {addr}/{bits}: error {got}")),
        }
    }

    /// Routes `to`/`bits` into the adapter, until the returned [`Route`] is
    /// dropped.
    ///
    /// **This is the call that makes the capture happen**, and the one that
    /// can take a machine off the network. What is routed in arrives as
    /// packets for [`crate::tun`] to pump and [`crate::stack`] to answer, so
    /// a route added with nothing reading the other end is traffic going
    /// nowhere — not an error, a black hole.
    ///
    /// Which prefixes is the caller's, and
    /// [capture-modes.md](../../../docs/design/capture-modes.md) says why the
    /// answer should be `0.0.0.0/1` and `128.0.0.0/1` rather than
    /// `0.0.0.0/0`: a `/0` gives the interface a metric of zero and turns
    /// Windows' automatic metric logic off, where the two halves reach just
    /// as far and leave the real default route in place.
    pub fn route(self: &Arc<Self>, to: IpAddr, bits: u8) -> Result<Route> {
        use windows_sys::Win32::{
            Foundation::NO_ERROR,
            NetworkManagement::IpHelper::{
                CreateIpForwardEntry2, IP_ADDRESS_PREFIX, InitializeIpForwardEntry,
                MIB_IPFORWARD_ROW2,
            },
            NetworkManagement::Ndis::NET_LUID_LH,
        };
        let mut row = MIB_IPFORWARD_ROW2::default();
        // As with an address, the defaults are not zeroes: this is what sets
        // the metric and the protocol to something Windows will accept.
        // SAFETY: a row of ours for the call to fill in.
        unsafe { InitializeIpForwardEntry(&mut row) };
        row.InterfaceLuid = NET_LUID_LH {
            Value: self.luid()?,
        };
        row.DestinationPrefix = IP_ADDRESS_PREFIX {
            Prefix: sockaddr(to),
            PrefixLength: bits,
        };
        // The unspecified address, which is what makes this on-link: there
        // is no gateway beyond a TUN device, the adapter *is* the far end.
        // A next hop of anything else would be a router to look for.
        row.NextHop = sockaddr(match to {
            IpAddr::V4(_) => IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            IpAddr::V6(_) => IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
        });
        // SAFETY: a row this call reads and does not keep.
        let got = unsafe { CreateIpForwardEntry2(&row) };
        match got {
            NO_ERROR => Ok(Route {
                row,
                _held: Arc::clone(self),
            }),
            _ => Err(anyhow!("route {to}/{bits} into the adapter: error {got}")),
        }
    }
}

/// An address on the adapter, taken away again when this is dropped.
///
/// **It holds the adapter rather than borrowing it**, which is a stronger
/// form of the same guarantee. Deleting an address after its adapter has gone
/// is meaningless -- the adapter going takes its addresses with it -- and a
/// borrow made that order a compile error at the call site. An `Arc` makes it
/// impossible instead: while this lives, the adapter cannot go.
///
/// The borrow was changed for a reason, not for taste: the capture mode's
/// source owns the adapter and must keep this for as long as it runs, and
/// nothing can hold a value that borrows from a value it also owns.
#[must_use = "dropping this takes the address off the adapter again"]
pub struct Address {
    row: ::windows_sys::Win32::NetworkManagement::IpHelper::MIB_UNICASTIPADDRESS_ROW,
    /// Held so the adapter cannot go first. Nothing reads it.
    _held: Arc<WintunAdapter>,
}

impl Drop for Address {
    fn drop(&mut self) {
        use windows_sys::Win32::{
            Foundation::NO_ERROR, NetworkManagement::IpHelper::DeleteUnicastIpAddressEntry,
        };
        // SAFETY: the row this address was made from, untouched since.
        let got = unsafe { DeleteUnicastIpAddressEntry(&self.row) };
        if got != NO_ERROR {
            tracing::warn!("take the adapter's address away again: error {got}");
        }
    }
}

/// An address as Windows wants one: the family in the structure rather than
/// beside it.
#[cfg(windows)]
fn sockaddr(addr: IpAddr) -> ::windows_sys::Win32::Networking::WinSock::SOCKADDR_INET {
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, IN_ADDR, IN_ADDR_0, IN6_ADDR, IN6_ADDR_0, SOCKADDR_IN, SOCKADDR_IN6,
        SOCKADDR_IN6_0, SOCKADDR_INET,
    };
    match addr {
        IpAddr::V4(v4) => SOCKADDR_INET {
            Ipv4: SOCKADDR_IN {
                sin_family: AF_INET,
                sin_port: 0,
                sin_addr: IN_ADDR {
                    // `S_addr` holds the address in network byte order, and
                    // `octets` already is that order, so the four bytes go in
                    // as the bytes they are. `to_be` here would reverse an
                    // address that was already the right way round — the
                    // same trap as `IP_UNICAST_IF`, in the other direction.
                    S_un: IN_ADDR_0 {
                        S_addr: u32::from_ne_bytes(v4.octets()),
                    },
                },
                sin_zero: [0; 8],
            },
        },
        IpAddr::V6(v6) => SOCKADDR_INET {
            Ipv6: SOCKADDR_IN6 {
                sin6_family: AF_INET6,
                sin6_port: 0,
                sin6_flowinfo: 0,
                sin6_addr: IN6_ADDR {
                    u: IN6_ADDR_0 { Byte: v6.octets() },
                },
                Anonymous: SOCKADDR_IN6_0 { sin6_scope_id: 0 },
            },
        },
    }
}

/// A route into the adapter, taken away again when this is dropped.
///
/// Holds the adapter, as [`Address`] does and for the reason given there: a
/// route outliving the interface it points at is nothing Windows will act on.
#[must_use = "dropping this takes the route away again"]
pub struct Route {
    row: ::windows_sys::Win32::NetworkManagement::IpHelper::MIB_IPFORWARD_ROW2,
    /// Held so the adapter cannot go first. Nothing reads it.
    _held: Arc<WintunAdapter>,
}

impl Drop for Route {
    fn drop(&mut self) {
        use windows_sys::Win32::{
            Foundation::NO_ERROR, NetworkManagement::IpHelper::DeleteIpForwardEntry2,
        };
        // SAFETY: the row this route was made from, untouched since.
        let got = unsafe { DeleteIpForwardEntry2(&self.row) };
        if got != NO_ERROR {
            tracing::warn!("take the adapter's route away again: error {got}");
        }
    }
}

/// The adapter **up**: its addresses, the routes that send this machine's
/// traffic into it, and the adapter itself, held together so that the pump
/// can own all of it and dropping it takes all of it down.
///
/// Which addresses these are barely matters, for the reason
/// [`WintunAdapter::address`] gives: the stack behind the adapter answers for
/// every destination, so an address is only what makes the adapter something
/// Windows will route to at all. What matters is that they are unlikely to be
/// in use, since an address this machine really reaches another way would be
/// shadowed by ours. Hence `198.18.0.1`, from the range RFC 2544 set aside
/// for benchmarking and which no LAN legitimately routes, and a unique local
/// address for IPv6.
///
/// **The routes are the two halves of the address space rather than a default
/// route.** A `/1` reaches further than any `/0`, so the machine's own
/// default route is left standing -- which is what tracewing's own
/// connections still use, by pinning to the interface
/// [`crate::roads::out_by`] names -- while everything that does not pin goes
/// into the adapter. A default route of our own would have had to replace
/// theirs, and putting a route back is a worse failure than never taking it.
pub struct Held {
    adapter: Arc<WintunAdapter>,
    /// Held to be dropped: the names say what they are for, since nothing
    /// reads them.
    _addresses: Vec<Address>,
    _routes: Vec<Route>,
}

impl Held {
    /// Opens the adapter and brings it up.
    ///
    /// The addresses go on before the routes, which is the order Windows
    /// wants: a route into an interface that holds no address of that family
    /// is one it has nothing to send from. IPv6's routes go on only where
    /// the machine has a way out for IPv6 of its own.
    ///
    /// Whether it has is asked once, here: a machine that gains IPv6 while
    /// the mode is on has its IPv6 captured from the next time it is turned
    /// on.
    pub fn up(dll: &Path) -> Result<Self> {
        use std::net::{Ipv4Addr, Ipv6Addr};
        const V4: IpAddr = IpAddr::V4(Ipv4Addr::new(198, 18, 0, 1));
        const V6: IpAddr = IpAddr::V6(Ipv6Addr::new(0xfd77, 0x7472, 0x6163, 0, 0, 0, 0, 1));
        let adapter = Arc::new(WintunAdapter::open(dll)?);
        let addresses = vec![adapter.address(V4, 32)?, adapter.address(V6, 128)?];
        let mut routes = vec![
            adapter.route(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 1)?,
            adapter.route(IpAddr::V4(Ipv4Addr::new(128, 0, 0, 0)), 1)?,
        ];
        // **IPv6 only where the machine has a way out for it.** Without one,
        // our halves were the only IPv6 routes there were: a program that
        // tries IPv6 first (curl, browsers) had its connection accepted by
        // the stack and then dropped, since a connection nobody chose has
        // nowhere to be passed on to, and it did not fall back to IPv4 as it
        // does when IPv6 fails at once. Measured on a GitHub runner, which
        // has no IPv6: HTTPS to example.com from a program nobody chose
        // worked with the mode off and failed with it on. Left alone, IPv6
        // fails at once as it did before.
        let ours = adapter.luid()?;
        let v6_out = crate::roads::roads_now()
            .ok()
            .and_then(|roads| crate::roads::out_by(&roads, ours, true))
            .is_some();
        if v6_out {
            routes.push(adapter.route(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 1)?);
            routes.push(adapter.route(IpAddr::V6(Ipv6Addr::new(0x8000, 0, 0, 0, 0, 0, 0, 0)), 1)?);
        } else {
            tracing::info!("no way out for IPv6 here, so IPv6 is not routed into the adapter");
        }
        Ok(Self {
            adapter,
            _addresses: addresses,
            _routes: routes,
        })
    }

    /// The adapter's LUID, which the sieve's way out has to leave out.
    pub fn luid(&self) -> Result<u64> {
        self.adapter.luid()
    }

    /// A handle to the adapter itself, for whoever has to end the capture.
    ///
    /// Ending it is [`crate::tun::Device::shutdown`] on the adapter, and that
    /// is the first link of the cascade `tun::run` documents -- so the window
    /// can stop the mode without the source being handed back to it.
    pub fn adapter(&self) -> Arc<WintunAdapter> {
        Arc::clone(&self.adapter)
    }
}

impl Device for Held {
    fn name(&self) -> String {
        self.adapter.name()
    }

    fn read(&self) -> Result<Vec<u8>> {
        self.adapter.read()
    }

    fn write(&self, packet: &[u8]) -> Result<()> {
        self.adapter.write(packet)
    }

    fn shutdown(&self) {
        self.adapter.shutdown();
    }
}

impl Device for WintunAdapter {
    fn name(&self) -> String {
        "wintun".to_owned()
    }

    fn read(&self) -> Result<Vec<u8>> {
        let packet = self
            .session
            .receive_blocking()
            .map_err(|e| anyhow!("read from the Wintun adapter: {e}"))?;
        // Copied out rather than held: the packet borrows the ring until it
        // is dropped, and the stack is handed bytes of its own.
        Ok(packet.bytes().to_vec())
    }

    /// A full ring is reported, not hidden, and [`crate::tun`] drops the
    /// packet. That is Wintun's own documented contract rather than a choice
    /// made here: its README's example ignores `ERROR_BUFFER_OVERFLOW` with
    /// the comment "silently drop packets if the ring is full".
    fn write(&self, packet: &[u8]) -> Result<()> {
        let size = u16::try_from(packet.len()).map_err(|_| {
            anyhow!(
                "a packet of {} bytes is longer than Wintun takes",
                packet.len()
            )
        })?;
        let mut out = self
            .session
            .allocate_send_packet(size)
            .map_err(|e| anyhow!("room in the Wintun ring for {size} bytes: {e}"))?;
        out.bytes_mut().copy_from_slice(packet);
        self.session.send_packet(out);
        Ok(())
    }

    fn shutdown(&self) {
        // Unlike the two above, this failing is worth saying: it is what ends
        // the reading thread, and a stop that did not take would be a
        // tracewing that will not quit.
        if let Err(e) = self.session.shutdown() {
            tracing::warn!("asking the Wintun session to stop: {e}");
        }
    }
}
