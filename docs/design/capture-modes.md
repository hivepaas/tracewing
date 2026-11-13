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
| Shipping | Developer ID, notarized, **and an entitlement Apple grants by review** | a signed `.sys` driver alongside the app | kernel 6.8 or newer |
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

Two things to know before building on it:

- **TLS is fully covered. Plain HTTP is not, yet.** Inside a `CONNECT` tunnel
  hudsucker peeks four bytes: `\x16\x03` it treats as TLS, `GET ` it serves as
  HTTP, and anything else it tunnels blind with a warning. So a transparent
  plain-HTTP `POST` would pass through unseen. The fix is to rewrite such a
  connection's request lines into absolute form (`POST http://host/path`)
  rather than wrap it in `CONNECT`, which is a small HTTP/1 rewriter and the
  natural next piece of work here.
- **A source should name its target if it can.** hudsucker takes the
  certificate's name from the ClientHello's SNI, so certificates are right
  either way, but the flow's URI comes from the `CONNECT` authority. Hand over
  `Target::Addr` and the flow list shows `140.82.121.4`; peek the SNI and hand
  over `Target::Named` and it shows `github.com`.

## The sources to come

### Local mode, macOS

A `NETransparentProxyProvider` in a system extension. `handleNewFlow` gives an
`NEAppProxyTCPFlow` — a byte stream with the originating app's audit token —
and the extension passes it to tracewing over a unix socket. Flow-level, so no
TCP stack needed; this is the cheapest of the three in code and the most
expensive in paperwork.

Open questions, in the order they will bite:

- the entitlement, `com.apple.developer.networking.networkextension` with the
  system-extension variant. Apple reviews these; developers report weeks, and
  refusals. **Nothing else here can start until this is answered, so ask
  first** — it costs no code.
- the extension's own traffic must be excluded or it will capture itself;
- the extension runs under a tight memory budget, so bytes must leave it
  immediately rather than be buffered;
- it only sees TCP and UDP.

### Local mode, Windows

A redirector process running as Administrator, using WinDivert to take packets
by PID and re-inject ours. Packets, so it needs the TCP stack. WinDivert is
dual-licensed LGPLv3/GPLv2, which fits this project, and its prebuilt driver is
signed — but it is a kernel driver we would be shipping, with everything that
means for the installer and for antivirus noise.

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

So the order is not the order of value, it is the order of dependency:

1. **Ask Apple for the entitlement.** Today. It is pure waiting.
2. The seam — this change.
3. The absolute-form rewriter, so plain HTTP is covered before the first real
   transparent source exists.
4. The TCP/IP stack, with **WireGuard** as its first user: it is the one with
   no privilege problem and the clearest payoff.
5. Local mode on macOS, if the entitlement came through.
6. Local mode on Windows.
7. Local mode on Linux.

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
