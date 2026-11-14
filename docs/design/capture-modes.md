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
| Mechanism | NetworkExtension, App Proxy Provider | WinDivert (Windows Filtering Platform, kernel driver) | eBPF on `cgroup/sock`, into a TUN device |
| Hands over | **flows** | packets | packets |
| Needs our own TCP stack | no | **yes** | **yes** |
| Privilege | the user approves a system extension | Administrator | root, for `bpf()` |
| Shipping | Developer ID, notarized, packaged as a system extension | WinDivert's own signed `.sys` beside our executable; nothing of ours to sign | kernel 6.8 or newer |
| Knows the app by | audit token, so bundle id | PID | name, first 16 characters only (`task->comm`) |

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

A redirector process running as Administrator, using WinDivert to take packets
by PID and re-inject ours. Packets, so it needs the TCP stack.

Nothing here has to be signed by us. WinDivert asks only for Administrator —
`WinDivertOpen()` fails with error 5 without it — and says nothing about the
program that calls it. The driver does have to be signed, and the official
`WinDivert64.sys` already is, so it is shipped as it comes; only a driver we
built ourselves would need signing, and an unsigned one fails with error 577.
There is no installer to write either: `WinDivert.dll` and the `.sys` beside
our executable, and the driver installs itself silently on the first
`WinDivertOpen()`. Uninstalling is deleting the files, with `sc delete` or
`windivertctl uninstall` to take the loaded one away before a reboot does.

**Take the LGPLv3 branch.** WinDivert is "LGPLv3 or GPLv2, at your choice",
and only the first of those fits GPL-3.0-or-later: GPL-2.0-only would not.

Two things to weigh before committing to it, neither of them about signing:

- **Security software blocks the driver**, which WinDivert's own
  documentation gives an error number for (1275). Our Windows installer is
  not signed today, and an unsigned executable that elevates to
  Administrator and drops a kernel driver next to itself is the shape
  antivirus heuristics like most. Signing the installer is not something
  WinDivert requires; it is something this source would make necessary.
- **WinDivert looks dormant.** 2.2.2 is from September 2022 and the
  repository was last pushed to in March 2023; the driver's signing
  certificate has expired and holds only because the signature is
  timestamped. A kernel driver with nobody behind it to re-sign is a real
  dependency risk if Microsoft tightens policy or blocklists it.

The alternative, a WFP callout driver of our own, is where signing does
become ours: an EV certificate and attestation signing through Microsoft's
Partner Center, every time the driver changes. Much more expensive than
WinDivert, and a reason this stays at sixth rather than a reason to start it
sooner.

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

## Build once, use four times

The userspace TCP/IP stack is the keystone: Windows, Linux and WireGuard all
need it, macOS does not. `smoltcp` is the obvious foundation.

So the order is not the order of value, it is the order of dependency. Three
pieces are in already: the seam; plain HTTP on a taken connection, which
turned out not to need the rewriter this page once predicted; and **SOCKS5**,
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

What is left:

1. **Local mode on macOS.** Flow-level, so no TCP stack, and nothing to ask
   Apple for: the least work of the three operating systems.
2. The TCP/IP stack is **in**, in [`stack.rs`](../../crates/proxy/src/stack.rs):
   packets in, [`Took::Flow`](../../crates/proxy/src/source.rs) out, on
   `smoltcp`, answering each `SYN` as the server the client was going to. What
   is left of it is a device to feed it — nothing reads a real packet yet —
   and **WireGuard** is the first one to build: no privilege needed on this
   machine, and it is what finally reaches a phone whose apps ignore the
   Wi-Fi proxy. The stack itself is tested with packets written by hand, so a
   device is the only part that needs an operating system.

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

   One thing the stack does not do, which a device will want: it reads IPv4
   only. `smoltcp` is already built here with `proto-ipv6` on, so what is
   missing is this side of it — `peek` and the `Device` — and a phone on
   WireGuard will bring IPv6 along whether or not we are ready for it.
3. Local mode on Windows, which needs that stack, a kernel driver, and our
   installer signed first.
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
