# Capture modes

How traffic gets to tracewing, and how to add a way that does not exist yet.

Today there is one way: something is told to use the proxy — the system proxy
setting, `HTTP_PROXY` in a terminal, a device's Wi-Fi settings — and it connects
and says where it wants to go. Everything that will not be told is out of reach,
which is the whole of [capturing.md](../capturing.md): Node before it is
asked, Go CLIs outside that terminal, Flutter, a Docker daemon, a Windows
service.

The way out is for the OS to hand tracewing the connection whether the app
agreed or not. Every product does this differently, and each is a separate
piece of work. This page is about the seam they all plug into, so that the
first one does not have to invent it and the fourth does not have to move it.

## What a source has to deliver

Two things, and the proxy core needs nothing else:

1. **Where the connection was going.** A client that asked for a proxy says so
   itself, with `CONNECT` or an absolute URI. A connection taken on its way did
   not say, so the source has to know: an address from the packet, or a name
   out of the TLS ClientHello.
2. **Who sent it.** Today this is looked up after the fact by the connection's
   address ([`apps.rs`](../../crates/proxy/src/apps.rs), a thread walking
   `/proc`). A source that took the connection from the OS usually knows
   already, and knowing for certain beats looking it up.

## Flows or packets: the one thing that decides the cost

Every OS mechanism falls on one side of this, and the side decides how much
work the source is:

- **Flow-level.** The OS has already done TCP. It hands over "this app, to
  this address, here is the byte stream". Plug it straight in.
- **Packet-level.** The OS hands over IP packets. Something has to do the
  handshakes, sequence numbers, windows, retransmits and reassembly in user
  space before there is a stream to plug in. That something is a **TCP/IP stack
  of our own**, and it is the largest single piece of work on this page.

| | macOS | Windows | Linux |
|---|---|---|---|
| Mechanism | NetworkExtension, App Proxy Provider | Wintun (WireGuard's layer-3 TUN driver) | eBPF on `cgroup/sock`, into a TUN device |
| Hands over | **flows** | packets | packets |
| Needs our own TCP stack | no | **yes** | **yes** |
| Privilege | the user approves a system extension | Administrator | root, for `bpf()` |
| Shipping | Developer ID, notarized, packaged as a system extension | Wintun's signed `wintun.dll` beside our executable, driver inside it; nothing of ours to sign, but its licence is not ours either | kernel 6.8 or newer |
| Knows the app by | audit token, so bundle id | the TCP table, by address | name, first 16 characters only (`task->comm`) |

mitmproxy has all three and is worth reading before starting any of them; the
design here is deliberately close to theirs, and `mitmproxy_rs` is MIT, so its
code may be read and used in this GPL-3 project with attribution.

## What exists today

Two sources, both of the first kind — clients that ask:

- **the proxy port**, a loopback listener, bound by
  [`engine.rs`](../../crates/app/src/engine.rs) before the runtime starts;
- **other devices**, a listener on each of this machine's LAN addresses, which
  come and go as networks do.

Before this change both were written into `run_proxy` by hand, the second as a
loop of its own. They are now the first two implementations of one interface.

## The seam

[`crates/proxy/src/source.rs`](../../crates/proxy/src/source.rs).

```rust
/// Something that brings traffic to tracewing.
pub trait Source: Send + 'static {
    fn name(&self) -> String;
    /// Starts it, and returns once it is running. What it finds goes to
    /// `found`; it stops when `stop` is set. `proxy` is where clients reach
    /// tracewing on this machine: the port a taken connection is handed to,
    /// and the one a source has to leave out of what it captures, or it will
    /// capture tracewing itself.
    fn start(self: Box<Self>, proxy: SocketAddr, found: Found, stop: Stop) -> Result<()>;
}

/// What a source found.
pub enum Took {
    /// A listener whose clients say where they are going.
    Port(Port),
    /// One connection already on its way, taken from the OS.
    Flow(Flow),
}

pub struct Flow {
    pub stream: Box<dyn Duplex>,
    /// The other end, as the OS sees it.
    pub client: SocketAddr,
    /// Where it was going before tracewing took it.
    pub to: Target,
    /// What the source already knows about the program behind it. `None`
    /// leaves it to be looked up by `client`.
    pub who: Option<Arc<AppAnswer>>,
}

pub enum Target {
    /// All a packet carries.
    Addr(SocketAddr),
    /// A name, where the source saw one: an SNI, or a `Host` header.
    Named { host: String, port: u16 },
}
```

Three sources implement it today. `ProxyPort` and `Lan` are the two that were
there before, lifted out of `run_proxy` as they were. `Given` is the third,
and the one the next operating system will arrive through: it hands over flows
from a channel, so whatever reads them out of a bridge — a network extension's
unix socket, a redirector's named pipe, a TCP stack of our own above
WireGuard — feeds that channel and has to know nothing else. It is also how
the taken path is tested with no OS to do the taking, in
`a_taken_connection_is_recorded_as_the_app_that_made_it`.

A source may hand over as many `Took`s as it likes, whenever it likes. That is
not generality for its own sake: the LAN source produces a listener per address
and takes them away again, and a local-mode source will produce flows for as
long as it runs.

`stream` is `Box<dyn Duplex>` — anything `AsyncRead + AsyncWrite + Send +
Unpin` — because the next sources will not have a `TcpStream` to give: an
NE flow arrives over a unix socket, and a WireGuard or TUN flow comes out of
our own TCP stack.

### Why `Port` is a variant and not just a flow

tracewing's MITM core is [hudsucker](https://docs.rs/hudsucker), and
`Proxy::builder().with_listener()` takes a concrete `tokio::net::TcpListener`.
hudsucker owns the accept loop, and the type that serves one connection,
`InternalProxy`, is `pub(crate)`. So a listener cannot be reduced to a stream of
connections without forking hudsucker: the honest interface says "here is a
listener" for the sources that have one.

### How a taken flow reaches the core

[`handover.rs`](../../crates/proxy/src/handover.rs) makes a taken flow look
like a client that asked:

1. connect to tracewing's own proxy port;
2. write `CONNECT host:port` for whatever `Target` says, and read the `200`;
3. splice the two streams together;
4. register what the source knew — the app, and the real client address —
   under the port that handover socket got, so the handler attributes the flow
   to the app that made it and not to `127.0.0.1`. Same idea as `apps.rs`, with
   the answer given instead of looked up.

The cost is one loopback hop per taken connection. **The proxy port's own
traffic does not go through any of this** and is exactly as fast as before,
which matters: it is the path that has to carry a million flows.

There are two ways in, and the handover tells them apart by the first two
bytes the client sends. **TLS** (`\x16\x03`) gets the `CONNECT` above, since a
tunnel is the only place the proxy will terminate TLS. **Everything else** is
passed on as it stands, to be read at the proxy port as any client's request
is, and must not be wrapped: inside a tunnel hudsucker reads TLS and a plain
`GET` and tunnels the rest blind, so a transparent `POST` would pass through
unseen. A client that says nothing within two seconds is neither, and is
given a tunnel.

An earlier version of this page predicted a small HTTP/1 rewriter for that
second way, to turn `POST /orders` into `POST http://host/orders`. It was not
needed. The request only has to reach the proxy port, where hyper parses it
and frames the connection already; what was missing was the server's name in
the URI, which `capture.rs`'s `as_absolute` fills in from what the source
knew. No parser of our own, and keep-alive comes free.

One thing is left to know before building on it. **A source should name its
target if it can.** The URI is built from what the source handed over, and it
decides both what the flow is called and where the proxy connects, so a
`Target::Addr` means a flow named `140.82.121.4` and a `Host:` header with an
address in it, which a server with several names on one address will answer
wrongly. A name is not taken from the client's own `Host` header instead,
because that would be looked up again here and can answer with a different
address than the one the client was going to. So: read the ClientHello's
`server_name`, or the host out of a SOCKS request, and hand over
`Target::Named`. Having both — the name to show and ask with, the address to
connect to — needs the connector to take a destination of its own, and that
is the next piece of work for a source that deals in packets.

## The sources to come

### Local mode, macOS

A `NETransparentProxyProvider` in a system extension. `handleNewFlow` gives an
`NEAppProxyTCPFlow` — a byte stream with the originating app's audit token —
and the extension passes it to tracewing over a unix socket. Flow-level, so no
TCP stack needed, which makes it the cheapest of the three in code.

**There is nothing to ask Apple for.** Since November 2016 the Network
Extension provider capability — packet tunnel, app proxy, content filter — is
self-serve: tick it in Xcode's Signing & Capabilities, or on the developer
website, and the profile comes back with the entitlement in it. What still
needs Apple to authorise a managed capability is the app push provider (Local
Push Connectivity) and `NEHotspotHelper`, neither of which this needs.
An earlier draft of this page said the opposite, from reports of long waits
that were about those managed ones.

What it does take, as a Developer ID app rather than an App Store one:

- macOS 10.15 or later, and the provider packaged as a **system extension**,
  not an app extension;
- the `-systemextension` value of the entitlement in **both** the app and the
  extension — `app-proxy-provider-systemextension`, which is the one a
  transparent proxy uses, since `NETransparentProxyProvider` is an app proxy
  as far as the entitlement is concerned;
- `com.apple.developer.system-extension.install` on the app, so it may install
  the extension;
- in the extension's `Info.plist`, `NEProviderClasses` mapping
  `com.apple.networkextension.app-proxy` to the provider class, and an
  `NSSystemExtensionUsageDescription` for the prompt the user sees.

mitmproxy's redirector ships exactly that pair of entitlement files, and is
worth reading rather than guessing at.

**The step that will eat a day** is the export. Xcode's own Direct
Distribution could not produce a Developer ID system extension correctly until
very recently, so the archive has to be re-signed by hand: dump both
entitlement sets out of the archive with `codesign -d --entitlements`, add the
`-systemextension` suffix, swap in the Developer ID provisioning profiles for
the app and for the extension, then re-sign from the inside out — extension
first, app last, `--timestamp -o runtime` on each — and notarise and staple it
yourself. Keep the plain values in the Xcode project: development builds want
those, and only the Developer ID build wants the suffixed ones.

The rest, in the order it will bite:

- the extension's own traffic must be excluded or it will capture itself;
- the extension runs under a tight memory budget, so bytes must leave it
  immediately rather than be buffered;
- it only sees TCP and UDP.

### Local mode, Windows

**A Wintun adapter**, which the operating system routes traffic into, feeding
the TCP stack that is already here. Packets, so it needs that stack; no
process of its own, and no driver of ours.

This page planned WinDivert, and the switch is not about WinDivert's code. It
is a working driver with a maintainer who answers. What it has no owner for
is its future:

- **There is no signed ARM64 driver, and nobody has agreed to make one.** The
  ARM build compiles and runs; somebody offered money for a signed one in
  November 2025 and asked again at the end of that year. On a Windows-on-ARM
  machine this capture mode would simply not exist.
- **Its signing has nobody behind it.** v2.2.2 (September 2022) was declared
  the long-term version in September 2025, to change only for security. The
  attempt to have the driver signed again was abandoned in August 2026, after
  the volunteer's EV certificate expired part-way through the process. An
  earlier draft of this page called the project dormant, which was wrong in a
  way worth keeping: the code is frozen **on purpose** and will not rot from
  neglect. It is the signature that has no owner.
- **The antivirus risk this page predicted has happened.** BitDefender
  quarantined `WinDivert64.sys` from the last release in December 2025, with
  a clean VirusTotal.

Wintun answers each of those, and for a reason that is not a favour to us:
**WireGuard for Windows ships it**, so its binaries are kept alive by
somebody whose own product depends on them. That was checked on a Windows
machine rather than taken from a web page: the official 0.14.1 archive
matches its published SHA-256, and all four DLLs in it — amd64, arm64, arm,
x86 — carry a valid Authenticode signature from `CN=WireGuard LLC` with EV
attributes. The driver is not a file of its own; it lives inside
`wintun.dll`, which is the only thing to ship.

**The licence is not what the first draft of this section said**, and the
difference matters. It is *not* "more permissive than the GPLv2 of the
source". The prebuilt `wintun.dll` is proprietary, under a *Prebuilt
Binaries License* that licenses rather than sells it, forbids modifying,
decompiling or extracting from it, and reserves the right to change its own
terms at any time. What it does grant is exactly what tracewing needs, in
clause 3(d): redistribution **alongside other software that uses it only
through the published `wintun.h` API**, which is all tracewing does. So
shipping it is permitted — but there is licence arithmetic here. It is a
different sum, not an absent one:

- it is a proprietary binary beside a GPL-3 program, loaded dynamically and
  used only through its documented interface. That is the separate-work case
  rather than a combined one, and the same shape as any GPL program that
  `dlopen`s a vendor's driver library. Worth being deliberate about now
  rather than at a release.
- **`cargo about` will not see it.** It reads crates, and `wintun.dll` is
  not one, so its licence has to go into the Windows package's third-party
  notices by hand. Clause 3(c) forbids removing proprietary notices, so
  shipping it with none is the one mistake here that is plainly a breach.
- WinDivert's risk was that nobody would re-sign the driver. This one's is
  that the owner may change the terms (clause 8), and that the grant ends on
  non-compliance (clause 6). Smaller, and of a different kind — but not the
  absence of risk.

**What is given up is real.** WinDivert takes packets **by process**, leaves
the rest of the machine alone, and hands the owning PID over with them. An
adapter takes whatever is routed into it. So:

- choosing one program becomes a decision made inside tracewing, after the
  packet is already ours. Possible, but every program's traffic then goes
  through tracewing in order to be decided about: a wider blast radius, and a
  cost paid on traffic nobody asked us to look at.
- the program behind a flow is looked up by its address, as it is for every
  other way in, instead of arriving with the packet. One change is needed
  there and it is an easy one to miss:
  [`apps/windows.rs`](../../crates/proxy/src/apps/windows.rs) keeps only the
  rows of the TCP table whose **far end is a port of tracewing's**, and a
  flow taken from an adapter has the real server at its far end. It needs a
  look by local address and port alone.
- [`features.md`](../features.md) promises "capture one app without proxy
  settings". That sentence has to be made to match what an adapter can do
  before this ships.

#### What is in

[`tun.rs`](../../crates/proxy/src/tun.rs) and
[`wintun.rs`](../../crates/proxy/src/wintun.rs): the adapter, its session,
and the pump between a blocking device and the stack's channels.

The split between them is not generality for its own sake. None of the
platform half can be built or run by the CI that sees every change, which is
Linux — so the pump, which is where the bugs with teeth are, is a trait with
a fake device behind it and five tests that run everywhere. The three things
they guard: a **stop that hangs**, because only the device can end a blocked
read and nothing else will notice until somebody tries to quit; **one refused
packet ending the whole capture**, when a full ring is ordinary under load
and the packet is the operating system's to send again; and a **read and a
write starving each other**, which is why each gets a thread and why the
device is `&self` throughout.

#### What a Windows machine said, and CI cannot

[`tests/wintun_live.rs`](../../crates/proxy/tests/wintun_live.rs) is
`#[ignore]`d and wants Administrator and a real DLL:
`$env:WINTUN_DLL = "...\\wintun\\bin\\amd64\\wintun.dll"`, then
`cargo test -p tracewing-proxy --test wintun_live -- --ignored`. CI compiles
it and never runs it, which is enough to keep it from rotting. Run in a
Windows VM it settled three things no amount of compiling could:

- the adapter opens at all — the DLL loads from a path, `Adapter::create`
  takes our fixed GUID, and `start_session` accepts the 4 MiB ring;
- a packet goes out through `allocate_send_packet` and `send_packet`;
- **a shutdown ends the reading loop**, saying `Session shutting down`. That
  is the one contract the whole stop in `tun.rs` rests on, and it was until
  then an assumption.

It also found what the next piece of work would have tripped over, and found
it by **failing**. A first version of that third test asserted that nothing
arrives on an adapter with no address and no route. **A fresh adapter is not
quiet:** three IPv6 packets — 64, 48 and 76 bytes, which is router and
neighbour solicitation and a multicast listener report — arrive within half a
second of the adapter existing. So the stack is handed Windows's own IPv6
housekeeping from the moment the device opens, before anything has been
routed into it on purpose. Nothing is wrong with that; the stack drops what
opens no connection. But anything that counts packets, logs them, or expects
a quiet start will be surprised, and it is cheaper to know now than to debug
later.

#### What is not in, and nothing captures until it is

**An address on the adapter, a route into it, and the exclusion that keeps
tracewing's own connections out of it.** Without that last one, every
connection tracewing makes on a captured program's behalf is routed back into
the adapter, where the stack answers it as the server it was going to, which
makes another connection, which is routed back in: a loop with nothing
bounding it. That is this mode's one genuinely dangerous piece, and it lives
in the outbound path rather than in the device.

**The mechanism is `IP_UNICAST_IF`, and there is no way to reach it from
where tracewing connects.** Checked in hyper-util 0.1.21's own source rather
than assumed: `HttpConnector::set_interface` is compiled only for Android,
Fuchsia, Linux and the Apple and Solaris families, and none of its other
setters takes a socket, a handle or a closure. `set_local_address` is no
substitute — a source address does not decide which route a destination
takes, so with a default route into the adapter a socket bound to the
physical address still leaves down the adapter, now with the wrong source.

So the exclusion costs more than one call, and the next piece of work is a
choice rather than an obvious step:

- **Connect ourselves**, in
  [`upstream.rs`](../../crates/proxy/src/upstream.rs)'s `connect_straight`,
  building the socket and setting the option before connecting. The direct
  answer, and the price is that it replaces hyper's connector on **every**
  platform's outbound path — including the happy eyeballs the code there
  relies on today, "falling back from IPv6 to IPv4 when the first is slow",
  which is not fifty lines to write again correctly.
- **Ask upstream for the Windows arm**, since `set_interface` exists already
  and wants only a mapping onto `IP_UNICAST_IF`. The cheapest answer if it
  lands, and not ours to schedule.

**And pinning is not free, which is the part worth knowing before building
it.** WireGuard for Windows solves exactly this problem — its own UDP must
not go down its own tunnel — and
[its notes on Windows networking](https://git.zx2c4.com/wireguard-windows/tree/docs/netquirk.md)
say four things that apply to us directly. They are its words about its own
service, not measurements of ours, and worth repeating rather than
rediscovering:

- **The option forces traffic, it does not merely permit it.** "The use of
  `IP_UNICAST_IF` forces the packets to, in fact, go over the default route
  interface" — including packets that would otherwise have left by a more
  specific route. So while tracewing holds the adapter, *all* of its own
  outbound connections are pushed out one interface, which can be wrong for
  a machine with a second network or a VPN of its own. WireGuard calls this
  problematic and says Microsoft offers no viable workaround. It is a cost
  this mode has, not a detail of how to call it.
- **The interface has to be watched, not read once.** Their service "watches
  for a default-route interface other than WireGuard's own", because which
  one that is changes as a machine moves between Wi-Fi, Ethernet and
  tethering. Pinning at startup would be right until the user unplugged
  something. And with no default-route interface at all, "those outgoing
  packets are blackholed".
- **Route the adapter with two halves, not `/0`.** They advise `0.0.0.0/1`
  and `128.0.0.0/1` over `0.0.0.0/0`, and `::/1` and `8000::/1` over `::/0`.
  Their stated reason is their own kill-switch firewall rules, which are
  theirs and not something Windows does to any adapter — but a `/0` also
  gives the interface a metric of `0` and turns off Windows' automatic metric
  logic, and the two halves leave the real default route in place, which is
  what we want anyway.
- **A fixed adapter GUID was the right call for a documented reason.**
  Windows ties firewall policy and the network category to the adapter's
  GUID, so a new one each run means re-answering for a new network. That is
  why [`wintun.rs`](../../crates/proxy/src/wintun.rs) pins one, and now there
  is a source for it rather than a hunch.

One thing this settles, and it is worth saying plainly: a TUN captures by
**route**, which is to say by destination, and no route can express "this
program". So this mode captures the whole machine, and choosing one program
is a filter applied after the packets are already ours rather than an
exemption the operating system makes on our behalf. Which is the other half
of the `features.md` problem noted above.

The `wintun` crate's own address setters are not used for any of it: they
shell out to `netsh`. Addressing and routing go through `windows-sys`, with
the rest of this program's Windows calls.

**Signing the installer is no longer a prerequisite**, and that is what moves
this up the list. The driver is signed by WireGuard LLC and widely installed
already, so what happens is no longer an elevated unsigned installer dropping
an unknown kernel driver beside itself — the shape antivirus heuristics like
most. Signing the installer is still worth doing; it is no longer what blocks
the mode.


### Local mode, Linux

A root helper that loads a `BPF_PROG_TYPE_CGROUP_SOCK` program, keeps the list
of process names to catch in a BPF map, and sends sockets that match to a TUN
device; packets from there go through our TCP stack. Lowest priority of the
three: it needs kernel 6.8, which is past the glibc 2.35 floor the `.deb` aims
at, it only catches outbound connections, process names match on 16 characters,
containers need `--network host`, and WSL cannot do it at all — while Linux
developers are the ones most at home with `tracewing env` anyway.

### WireGuard

tracewing runs a userspace WireGuard server; a phone connects with the stock
WireGuard app and needs nothing installed. No privilege on this machine, and it
needs the same TCP stack. This is the best answer to the gap that actually
costs tracewing users today — a phone has to be told the proxy by hand, and an
app that ignores the Wi-Fi proxy cannot be caught at all.

**The endpoint is in**, in
[`wireguard.rs`](../../crates/proxy/src/wireguard.rs): a UDP port, one peer,
and the packets it takes go to [`stack.rs`](../../crates/proxy/src/stack.rs)
through the channels `Wire::channels` hands out. What a phone needs is an
address and a key, and what comes out the other end is flows.

**The protocol is `boringtun`'s**, pinned at the published 0.7.1 and built
with no default features, so none of its own device or FFI layers come with
it. The reason to take it rather than write it is not the handshake, which is
a documented Noise pattern — it is the cookie replies that answer a flood,
the rekeying timers, the replay window and the session bookkeeping, every one
of which fails **silently**. A handshake that is wrong in a way tests cannot
see still completes, and there is no ablation that catches it. BSD-3-Clause,
which `about.toml` already accepts.

What the library deliberately leaves out is what tracewing wanted anyway: it
implements the protocol "without the network or tunnel stacks", so the socket,
the peer's address and the bridge are ours, and what `decapsulate` hands back
is an IP packet — which is exactly what `Wire` takes.

Two things are not done. **One peer**: a second needs the receiver index read
out of each datagram to tell whose it is, which `Tunn::parse_incoming_packet`
is there for, and is no use before peers can be configured. And **nothing
turns it on yet**: the key, the port and the QR code the phone scans are
Settings and window work, and `engine.rs` does not build one. The endpoint
and its tests stand on their own until then.

The peer's address is learnt from whatever datagram last authenticated, which
is how WireGuard roams: a phone moving from Wi-Fi to mobile keeps its session
and arrives from somewhere else, and answering to where it used to be would
lose it.

## Build once, use four times

The userspace TCP/IP stack is the keystone: Windows, Linux and WireGuard all
need it, macOS does not. `smoltcp` is the obvious foundation.

So the order is not the order of value, it is the order of dependency. What
is in is below, under *What is left*; the three that went first were the
seam; plain HTTP on a taken connection, which turned out not to need the
rewriter this page once predicted; and **SOCKS5**,
in [`socks.rs`](../../crates/proxy/src/socks.rs) — not an operating system at
all, just a listener and the RFC 1928 handshake, after which what is left on
the socket is a stream to a destination the client named, which is a
`Took::Flow` and nothing more. It went first because it needs no privilege on
any of the three platforms, and because the taken path otherwise had no caller
but its own tests: the first real user of a seam is what finds the seam's
bugs. It also gets `Target::Named` free from the protocol, where a source
reading packets has to find the name in a ClientHello.

Nothing is left of SOCKS5. The port is in Settings, off by default, on this
computer only, and read when tracewing starts.

What the protocol did cost, beyond itself, was teaching
[`apps`](../../crates/proxy/src/apps.rs) that tracewing has more than one way
in. It finds the program behind a connection by looking for a socket whose far
end is tracewing's port, and a SOCKS5 client's far end is the SOCKS5 port, so
those flows arrived with no program against them. A source now says which
ports of ours clients reach it on — `Source::own_ports` — and `apps` is given
the lot, the proxy port first because that is where most connections come
from. On Linux that order is what keeps the cost off the common case: one
netlink ask names one port, so a client on the proxy port still costs one ask
and only the others pay for a second. The look itself is started by the
client's address either way, which is why a taken flow whose source named no
program is now looked up like any client that asked.

What is left, and what no longer is:

1. **Local mode on macOS is in**, in
   [`crates/netext`](../../crates/netext): a `NETransparentProxyProvider` in
   a system extension, which takes the TCP flows of the apps its
   configuration names. Flow-level, so it needed none of the TCP stack below.
   `scripts/package_macos.sh` puts it in the app when the entitlement is
   there; `docs/building.md` has how to build and run it.

   It reaches the seam as a source like the others:
   [`extension.rs`](../../crates/proxy/src/extension.rs), a port of its own
   on the loopback. For each flow the extension connects there and says, in
   a few lines ([`crates/handoff`](../../crates/handoff)), where the flow
   was going — the name the app asked for, which macOS gives even to a
   program that connected by address — and the app's signing id and process.
   What follows is the app's conversation, handed over as a `Took::Flow`, so
   the TLS and plain-HTTP split, the name in the hello and `as_absolute` are
   the same code every taken flow goes through: a plain `POST` is read, not
   tunnelled blind.

   **The program is named, not looked for.** It used to be the open end of
   this: an earlier version sent a `CONNECT` with the app in an
   `X-Tracewing-App` header that nothing read, so the program was looked up
   by the socket's owner, which is the extension. Now the port reads the
   app's process from the lines and the program from that process, as the
   `who` of the flow; `curl` on a Mac is recorded as `curl`, with its pid.

   **Both ends prove they hold a key**, which tracewing makes and saves in
   the extension's configuration, and neither ever sends. The extension's
   lines are signed with it, so no other program on the computer can hand a
   flow over under another app's name; tracewing's answer is signed too, and
   that one matters more. While tracewing is not running, anything may
   listen on its port, and an extension that trusted whatever answered would
   hand it the traffic of every app it was told to take. Instead a flow that
   cannot be handed to something proving it is tracewing goes out as it
   would have: the extension does the hand-off before it answers macOS, so
   it can still say no.

   What is left is the app's end: installing the extension, the key, and
   saving and starting the configuration with the capture.
2. The TCP/IP stack is **in**, in [`stack.rs`](../../crates/proxy/src/stack.rs):
   packets in, [`Took::Flow`](../../crates/proxy/src/source.rs) out, on
   `smoltcp`, answering each `SYN` as the server the client was going to. What
   is left of it is a device to feed it, and the first of those is **in**:
   the WireGuard endpoint in
   [`wireguard.rs`](../../crates/proxy/src/wireguard.rs), which needs no
   privilege on this machine and is what reaches a phone whose apps ignore
   the Wi-Fi proxy.

   **And it is switched on.** A setting and a port, both keys kept where
   the CA's key is kept, the configuration shown as a QR code the WireGuard
   app scans, and the names the phone asks for answered — a phone sends its
   DNS down the tunnel too, and dropping that looks to the phone like the
   network has gone rather than like a name that will not resolve. What is
   left of the tunnel itself is a second peer, which needs the receiver
   index read out of each datagram to tell whose it is.

   The stack itself is tested with packets written by hand; the endpoint is
   tested with `boringtun` playing the phone, so neither needs an operating
   system to prove.

   **The name is in**, in [`name.rs`](../../crates/proxy/src/name.rs). A
   packet carries no name, so the stack hands a flow over as an address; the
   first TLS record the client writes carries one, and
   [`handover`](../../crates/proxy/src/handover.rs) reads the `server_name`
   out of it as the flow goes past. So a flow the stack took is called
   `github.com:443` rather than `140.82.121.4:443`, asks the server by the
   name it holds a certificate for, and every source that deals in addresses
   gets that for nothing — this one, WinDivert, the BPF helper. It is
   parsed, not searched for: every length in a `ClientHello` comes from the
   client, so a length that disagrees with the record ends the walk, and the
   name is refused unless it is a plain DNS name. That last part is not
   cosmetic — the name is formatted into a `CONNECT` line, and one holding a
   carriage return would smuggle a second request into tracewing's own
   proxy port.

   This is not only about reading well. `upstream.rs` takes the host to ask
   for from the URI, so an address there becomes a rustls
   `ServerName::IpAddress`: no SNI goes out, and the server's certificate is
   checked against the address, which no public certificate carries. A TLS
   flow taken as an address **failed on the way out**. The client's own side
   was never broken — hudsucker issues its leaf from the SNI the client
   sent, so the client saw the name it expected all along.

   What this does *not* do is keep the address beside the name: the name
   replaces it, so the proxy resolves it again on the way out. That can
   reach a different address than the client was going to — a CDN's nearer
   edge, a split-horizon answer — or fail to resolve at all, for a name only
   the client's own resolver knows. A connection that used to be made by
   address can now fail, and that is the real price of this rather than a
   theoretical one; it is the trade `socks5h://` has always made, for a
   handshake that works instead of one that cannot. Holding both needs the
   connector to take a destination of its own, and that is the piece still
   to come.

   **IPv6 is in** as well. `peek` reads either family — the version nibble
   picks the reader, and a v6 header is a fixed forty bytes with the protocol
   in `next_header` instead of `protocol` — and the rest of the stack never
   needed telling, because it works in `IpAddress` throughout. A phone on
   WireGuard brings IPv6 along whether or not we are ready for it, so this
   was worth doing before the device rather than after.

   **UDP is refused, with one exception that had to be made.** The stack
   turns TCP into flows and nothing else, so a phone's QUIC gets an ICMP
   port unreachable: `smoltcp` answers a datagram no socket accepts that
   way, in `iface/interface/udp.rs`, and that module is compiled in because
   of the `socket-udp` feature the names needed. It is worth more than a
   drop — a datagram dropped in silence is retried until it times out, and
   an app waiting on a timeout looks stuck, where a refusal sends it to TCP
   at once. `quic_is_refused_rather_than_dropped` asserts it, because
   nothing of ours does it and so nothing of ours would notice it stopping.
   But a phone
   sends its **DNS** down the tunnel too, and a dropped query does not look
   like a DNS fault from the phone: it looks like the network has gone,
   because everything it does starts with a name. So the stack answers
   `A` and `AAAA` itself, on a `smoltcp` UDP socket bound to port 53 at any
   address, and [`dns.rs`](../../crates/proxy/src/dns.rs) resolves each name
   through the operating system rather than forwarding to a resolver chosen
   on the user's behalf. The answer is sent back with the metadata the query
   arrived with, which is what makes it come **from the address the phone
   asked**: `smoltcp` keeps the destination in `local_address` and sends the
   reply from it, and a reply from anywhere else is one a phone ignores.

   Two packets are deliberately left alone, and they are the same case
   twice: an IPv4 fragment that is not the first, and an IPv6 packet
   carrying an extension header. In both the TCP header is not where `peek`
   looks, so no connection is made and the stack drops the packet. A client
   on a path that needs either is a client this does not catch — worth
   knowing rather than discovering, and cheaper to fix when something real
   needs it than to guess at now.
3. Local mode on Windows. The device is **in** — see its section — and
   what is left is an address, a route, and keeping tracewing's own
   connections out of the adapter. It no longer waits on a signed installer.
4. Local mode on Linux, last for the reasons in its section.

## What none of this changes

- **The CA still has to be trusted.** These modes decide what reaches
  tracewing, not what an app is willing to trust. If anything they make trust
  more visible, because they catch exactly the apps that carry their own store
  — Node, Python, Java, Flutter. See [capturing.md](../capturing.md).
- **The system proxy stays the default**, because it needs no privilege and no
  driver. So the safety net that puts it back when tracewing is killed — the
  journal, `trw-watchdog`, the minute timer — stays exactly as needed as it is
  now. These modes only make it rarer for a user to be exposed to it.
- **Pinning still wins.** The one answer that works against it is not a capture
  mode at all: take the raw TLS bytes these modes give us, read the client's
  own `SSLKEYLOGFILE`, and decrypt without touching the certificate. That
  deserves its own design.
