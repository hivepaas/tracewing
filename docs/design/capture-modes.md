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

### Which way in a flow came by, and what that can answer

Every source says which way in it is — `Source::by`, with no default, so a new
source cannot silently be counted as something else — and
[`Found::start`](../../crates/proxy/src/source.rs) stamps that onto everything
the source hands over. The stamp is on `Took`, not on `Port` or `Flow`,
because a field each source filled in itself is a field the next source
forgets, and the forgetting would be silent: that source would simply never
be counted, and nobody would find out until somebody asked why a tab said
nothing had arrived.

`Arrivals` then counts what comes in by each, and the handler takes the way in
from one of two places: a handed-over flow carries its own in `Known`, and
everything else came in on the listener the handler is serving. Neither is a
guess. The alternative — reading it off the client's address, loopback or not
— would have called a program on *this* machine that used the LAN address
another device.

**What this can and cannot answer.** Three ways in share `By::ProxyPort`: this
machine's proxy setting, a browser tracewing started with a profile of its
own, and a terminal it set the variables in. All three are told the same port
and the same listener accepts them, so nothing here tells them apart — and a
window that claimed to would be wrong. Everything else has a listener or a
bridge of its own and is exact, *other devices* included, since
[`Lan`](../../crates/proxy/src/source.rs) binds listeners of its own per
address rather than sharing the loopback's.

Separating tracewing's own browser from the other two by its process id was
looked at and is not available: `launch::spawn` drops the `Child`, so no pid
is kept. That is a bigger change than the sentence it would buy.

**Why a count is not enough.** The question worth answering is not "has
anything ever arrived" but "has anything arrived since this was switched on".
A count that was not zero last week is not zero now, so a bare count would
tell somebody who has just turned a switch on that their way in works, on the
strength of traffic from before they touched it. `Arrivals` keeps the time of
the last arrival and answers `anything_since`, against a moment the window
remembers. `Instant` and not a wall clock, because a clock that jumps —
a machine waking from sleep — would answer that comparison wrongly.

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
- [`features.md`](../features.md) promised "capture one app without proxy
  settings" for all three platforms. **Decided:** that row is now two — one
  app on macOS, where the extension really is handed one app's flows, and
  *this machine* on Windows and Linux, where the route decides and the
  choosing is ours. What that costs is [below](#what-capturing-the-whole-machine-costs).

#### Who holds the adapter, and the answer

Creating a Wintun adapter needs an administrator, and so do the address and
the routes. tracewing is not elevated, and **it already has an answer to
exactly this shape of problem**: blocking HTTP/3 needs a Windows Filtering
Platform filter, which needs an administrator, so
[`quic_windows.rs`](../../crates/system/src/quic_windows.rs) launches an
elevated copy of this program through the UAC dialog (`runas` via
`ShellExecuteExW`), and that copy holds the filter in a dynamic session and
ends when told to or when the app it serves has gone. So the honest line for
the window is not *run tracewing as an administrator* and not *you are or
are not one*: it is that **Windows asks when the switch is turned on**, as
it already does for HTTP/3.

What is **not** settled is whether the same arrangement carries an adapter,
and the difference between the two cases is the whole question. A WFP filter
is a kernel-side object held by a handle: whoever owns the handle is done. An
adapter has to be **read from** — a session's ring buffers live in the
address space of the process that started the session — so if the pump
cannot be in the unelevated process, it has to be in the helper.

The crate cannot answer it. `Adapter::open` exists (`adapter.rs:109`,
`WintunOpenAdapter`), and the only privilege note anywhere in `wintun` 0.5.1
is "Must be run as Administrator **because we create network adapters**",
which is about creating. `bindgen` carried no doc comment for
`WintunOpenAdapter`, so the signature is all there is here; `wintun.h` itself
is on the Windows VM, not in the crate.

**The experiment was run on the VM on 2026-11-16, and it is the expensive
answer: opening needs an administrator too.** What was measured, in one
transcript, with the adapter visible as `tracewing-probe / tracewing Tunnel`
between the steps:

| The token | `WintunOpenAdapter` |
|---|---|
| a **second elevated** process, Administrators enabled | **OK**, and `WintunStartSession` on it OK too |
| Administrators **deny-only** (`runas /trustlevel:0x20000`) | **`ERROR_ACCESS_DENIED`** |
| a scheduled task of the same account | OK — but see below, it is not the case it looks like |

The middle row is the one that decides it: **an unelevated process of an
administrator holds a token with Administrators present *for deny only*,**
and a deny-only SID does not satisfy an access check against a DACL that
grants Administrators. That is exactly the token tracewing itself would have.
The probe's token was high integrity with Administrators deny-only rather
than a full UAC-filtered token, which is medium; since the denial comes from
the group and not from the integrity level, a medium-integrity token can only
be denied as well.

The third row is a trap worth writing down rather than repeating: a scheduled
task of an administrator account **opened the adapter successfully**, because
**UAC token filtering is a property of interactive logon**. A batch logon
gets the full administrator token. So a scheduled task is no way to measure
what an unelevated app can do, and reading that row alone would have given
the opposite answer.

Two things that are good news inside the bad:

- **The pump does not have to be in the creating process.** A different
  process opened the adapter and started a session on it, so the two halves
  can be split — as long as both are elevated.
- **The adapter's lifetime is the creator's handle.** Closing it removed the
  adapter, twice, and `Get-NetAdapter` showed it gone. So "only for as long
  as it is on" is true of the adapter itself, whatever happens to the routes.

So this mode is the larger shape, and the cost is named rather than
discovered later: **the packet pump has to be elevated.** WireGuard for
Windows never had to choose — its tunnel service runs as `SYSTEM`.

**The DACL is not the lever, and that was measured rather than assumed.**
The obvious third possibility was an elevated creator loosening the DACL on
the adapter's device object so an unelevated proxy could open it. Both of the
adapter's device interfaces — `GUID_NDIS_LAN_CLASS` and
`GUID_DEVINTERFACE_NET` — carry the same one:

```
O:BA G:SY D:(A;;0x1201bf;;;WD)(A;;FA;;;SY)(A;;FA;;;BA)(A;;0x1200a9;;;RC)
```

`WD` is **Everyone**, and `0x1201bf` includes `FILE_READ_DATA` and
`FILE_WRITE_DATA`. So there is nothing to loosen: the device object already
grants everyone read and write, and an unelevated `WintunOpenAdapter` is
still `ERROR_ACCESS_DENIED`. Whatever refuses it is somewhere else — the
`SetupAPI` enumeration the DLL uses to find the device, a registry key, the
access on the `SWD\Wintun` device node, or a ring device of Wintun's own that
is not registered under `DeviceClasses`. Answering that means reading
Wintun's source, which is its own piece of work, so **this option is not
actionable** rather than cheap-but-risky, which is how it was first written
up here.

Two notes on how that was read, because both nearly produced the opposite
answer. `GetNamedSecurityInfo` with `SE_FILE_OBJECT` answers
`ERROR_NOT_SUPPORTED` on a device interface path; what works is `CreateFileW`
for `READ_CONTROL` and then `GetSecurityInfo` with `SE_KERNEL_OBJECT`. And a
first search of `HKLM\SYSTEM\CurrentControlSet\Control\DeviceClasses`
returned **nothing at all** while `Get-PnpDevice` showed the device plainly —
the search was broken, not the object absent, and an empty result with
nothing beside it would have read as "no such object, therefore not
feasible". The search now prints what it walked (70 classes, 184 instance
keys) so that empty and broken stop looking alike.

#### How it is elevated: a relaunch, and only while the mode is on

**Decided by the user.** tracewing **relaunches itself elevated** when the
switch goes on, through the UAC dialog — `ShellExecuteExW` with `runas`, the
call [`quic_windows.rs`](../../crates/system/src/quic_windows.rs) already
makes — and **starts a new session**. Nothing is handed across: the session
that was open is kept on disk and can be reopened from the list
(`store::session::list`), so the only thing lost is what was in front of
somebody, and the window can say so truthfully before it happens.

The alternatives were a small elevated helper carrying packets over a pipe,
or the whole stack moving into one. Both are more work than this: a helper
binary, a pipe protocol with batching, the route guards living on the far
side of it, and — for the version that moves the stack —
[`handover::Handed`](../../crates/proxy/src/handover.rs), which is an
in-process map that `capture.rs` reads, crossing a process boundary. None of
what a relaunch builds would have to be undone to do that later: the sieve,
the datagrams, the echoes, the pinning and the roads are shared by every
version of this.

What it costs, plainly: while the mode is on, **everything** runs elevated,
including the parsers that read bytes a server sent — `crates/convert`, TLS,
HTTP/2, brotli. That is the exposure a helper would have bought away, and it
is the reason to keep a helper on the list rather than close the question.

**Turning the mode off does not relaunch.** A process cannot drop to a
filtered token — that is the hard direction on Windows — so going back would
mean a second restart, and the decision is that this is not worth the
inconvenience when the user has already agreed to be elevated. Two things
make it honest instead:

- the window **says** tracewing is running as an administrator, for as long
  as it is. The thing worth avoiding was never elevation, which was agreed
  to; it was elevation **nobody can see**. A visible state is one somebody
  ends by restarting when it suits them.
- nothing else about the mode lingers: the address and the route go with
  `Drop`, and the adapter goes with the handle, which is measured above.

#### What an elevated tracewing must not hand to anybody

A child process inherits elevation, and tracewing starts two kinds of child.
The ones it runs **for itself** — `schtasks`, `keytool`, the watchdog, and
the `node --version` probe in
[`launch.rs`](../../crates/system/src/launch.rs) — can inherit it. The ones
it starts **for the user to use** are the question:

- a **browser**, `launch.rs`, for the browser way in;
- a **terminal**, [`windows::open_terminal`](../../crates/system/src/windows.rs),
  reached from Set up, which starts `cmd /c start tracewing cmd /k …` with
  the proxy variables already set. This one matters more than the browser,
  because somebody types commands into it.

So they are launched at the user's own level **by default**: the shell's
token, borrowed with `GetShellWindow` → `GetWindowThreadProcessId` →
`OpenProcess` → `OpenProcessToken` → `DuplicateTokenEx` →
`CreateProcessWithTokenW`, which needs `SE_IMPERSONATE_NAME` and an elevated
administrator has it. That gives the **real** interactive token, medium
integrity and filtered, which is what a browser started by double-clicking
has — not the restricted token `runas /trustlevel` makes, which a browser
may not survive.

One failure to get right: `GetShellWindow` answers `NULL` where no shell is
running — a kiosk, a server session, an explorer that died. There is then no
token to borrow, and the fallback **must not** be to launch elevated anyway,
because that is the exact thing being avoided and it would be invisible. It
has to be said instead.

##### But "as administrator" has to be offered, because the case is real

**Decided by the user.** Defaulting to the user's own level is not the same
as forbidding the other, and for a terminal the other is a first-class use
rather than an edge: `open_terminal` exists to put the proxy variables in
front of a program, and **a program that needs an administrator will not
start in a terminal that is not one** — a service, an installer, anything
writing `HKLM`, anything binding a port below 1024. Refusing would mean the
environment-variable way in cannot reach exactly the programs it is best at
reaching. There is a workaround today (open an elevated terminal by hand and
run `tracewing env` in it), so this is about convenience rather than
capability — but it is the convenience of the way in's main case.

For a **browser** the case is much weaker, and the warning should say so:
Chromium-based browsers refuse or complain when run as an administrator, so
the option exists for symmetry and for the rare extension or kiosk being
debugged under elevation.

So each of those two ways in gets a choice of its own in Settings, **at your
own level** by default, remembered rather than asked each time — the same
reasoning as the administrator indicator above: a visible setting beats a
repeated prompt, and the tab shows which one is in force.

**The choice is about what the child gets, not about what tracewing has**,
which is what makes it worth building on its own rather than as part of this
mode's wiring — it is useful in the ordinary, unelevated case, which is most
of the time. All four combinations are reachable:

| tracewing | the child wanted | how |
|---|---|---|
| not elevated | own level | an ordinary spawn, as today |
| not elevated | administrator | `ShellExecuteExW` with `runas` — **a UAC prompt per launch**, which the option's text has to say |
| elevated | own level | the borrowed shell token above |
| elevated | administrator | an ordinary spawn, inheriting |

The warning on "as administrator" wants to be concrete rather than stern:
what is started has full control of this computer and tracewing cannot limit
it, and — for the terminal — that this is why the option is there at all.

One method note, because it nearly cost a wrong answer. The first run of the
probe reported `CREATE OK` and then failed to open the adapter **from the
elevated session that had just created it**, with `ERROR_NOT_FOUND`, and no
adapter appeared under the name asked for. That was not a finding about
Wintun: a PowerShell delegate marshals `string` as **ANSI** unless told
otherwise, Wintun takes `LPCWSTR`, and the adapter had been created under a
garbled name. What exposed it was the control step failing and listing the
adapters **unfiltered** — a list filtered by the name we meant to use is
exactly how a garbled name hides.

The window's line still has to be true of this, and "only for as long as it
is on" has a second half that is still unverified: `Drop` takes the address and the route down and that is tested,
but `Drop` does not run for a killed process — see the last item under
[what capturing the whole machine costs](#what-capturing-the-whole-machine-costs).

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

**Whether Windows has the dead-route problem too is not known here.** On
Linux it did: a device up with its carrier down keeps its routes, `/proc`
lists them as up, and `out_by` chose one of them over the real interface
until the carrier was read — the measurement is under "Which way out is not
ours" in the Linux section. Windows reads `GetIpForwardTable2`, and whether a
disconnected interface's rows survive there, and whether `Metric` or the
`MIB_IPFORWARD_ROW2` flags say so, has not been tested. If they do survive,
the same silent hang is reachable, and the two `/1` halves above make it
worse rather than better: a dead interface with a `/1` pair beats a live
default route on reach. The case to arrange is a **second** interface going
down while tracewing holds its own — not the adapter itself, which is
already measured — with the table dumped before and after, and
`GetIfEntry2`'s `MediaConnectState` and `OperStatus` beside it, since that is
the Windows analogue of a carrier and the likelier place the state shows.

**Whichever way those tables answer, the rule that decides it is the same on
both platforms, and it is not symmetric.** Excluding too much leaves a family
with no way out, which is `Out::Nowhere` in `upstream.rs`: the connection is
refused and says why. Excluding too little pins to an interface the kernel is
ignoring, and that **hangs**. So anything the tables cannot vouch for is not
a way out — a loud failure beats a silent wait, and neither of them is the
loop. Worth having here rather than in each platform's own section, because
it is the kind of choice that looks like caution and is the opposite: the
cautious-looking default is the quiet one.

One thing this settles, and it is worth saying plainly: a TUN captures by
**route**, which is to say by destination, and no route can express "this
program". So this mode captures the whole machine, and choosing one program
is a filter applied after the packets are already ours rather than an
exemption the operating system makes on our behalf. `features.md` now says so
in two rows rather than one, and what the filter costs is next.

#### What capturing the whole machine costs

What follows is what changes when this stack is fed a machine's traffic
instead of one app's. Almost none of it is new: WireGuard mode already feeds
the same stack a whole phone. What is new is that the machine is the user's
own, so each item breaks the computer they are working on rather than a test
device.

- **TLS breaks for every program that pins, not only the ones already thought
  of.** [`rules.rs`](../../crates/proxy/src/rules.rs)'s `DEFAULT_NO_DECRYPT`
  holds five patterns, Apple's and Windows Update's. Defender, the Store,
  OneDrive, Teams, Dropbox, Steam and Chrome's own updater are not among them
  and all pin. This is the loudest day-one problem and the cheapest to soften:
  the Windows defaults want to be as long as the macOS ones before the mode
  can be switched on.
- **UDP is refused, and that is wider than QUIC.** The stack turns TCP into
  flows and answers DNS itself; every other datagram gets an ICMP port
  unreachable (see the WireGuard section). For QUIC that is deliberate — it
  sends an app back to HTTP/2. For a user's own VPN, NTP, a game or a voice
  call it is the connection not working while tracewing is on. ICMP goes the
  same way, so `ping` and `tracert` stop answering.
- **Every connection whose server greets first pays two seconds.**
  `FIRST_WORD_WITHIN` in [`handover.rs`](../../crates/proxy/src/handover.rs):
  SSH, RDP, SMTP, MySQL, Postgres, IMAP. One app's connections are a handful;
  a machine's are all of them. The fix is already written down there — go
  straight to a tunnel when the destination port is not HTTP's.
- **The stack's caps were sized for one tunnel, not for a machine.**
  `FLOWS_MAX` is 512 and `WINDOW` is 64 KiB, in
  [`stack.rs`](../../crates/proxy/src/stack.rs). A browser, the Store,
  OneDrive and Defender together can hold more than 512 connections at once;
  past the cap a `SYN` gets nothing and the program waits. And a download that
  would saturate the link now goes through a single-threaded userspace TCP
  stack, with a loopback hop per connection on top.
- **The session grows at the machine's rate.** Every flow is a row, and a body
  past the limit is a file. Telemetry, update checks and file sync produce
  thousands of rows nobody asked for.
- **Credentials of programs nobody meant to inspect are decrypted and written
  to disk.** Not a bug, a consequence — and the reason a filter that only
  hides rows is not a filter.
- **What the local network is spared.** An on-link prefix is longer than the
  two `/1`s, so file shares, printers and anything else beside this machine
  keep using the real interface and are never captured. The blast radius is
  what leaves the machine, not what is next to it.
- **A hard kill leaves nothing behind, and that is now measured.** The
  address and the route are deleted on `Drop`, which
  [`wintun_live.rs`](../../crates/proxy/tests/wintun_live.rs) tests — but
  `Drop` does not run for a killed process, and this was the one failure here
  that would cost somebody their network rather than their session. On
  2026-11-16 the holder was killed with `Stop-Process -Force`, so none of
  tracewing's cleanup ran, and afterwards the adapter was **gone** and the
  route on it was gone with it. Twice, consistently, with `Get-NetAdapter`
  and `Get-NetRoute` both asked.

  So a tracewing that crashes does not leave a Windows machine routing into a
  device nobody reads. That is better than it has to be on Linux, where the
  device survives its holder and what saves the machine is
  `ignore_routes_with_linkdown` — and where the order it is set in decides
  whether a dead default route is *still chosen*, with `sendto` succeeding and
  the packets vanishing. Windows removes the device, so there is no such state
  to ignore and no such ordering to get wrong.

  **Three things that run did not establish**, stated rather than rounded up.
  The last two were spotted by the session doing the Linux half, from its own
  hard-kill measurements, and both are cheap to close in the next window:

  - whether **traffic was flowing** into the adapter at the moment of the
    kill. The claim rests on the mechanism — the kernel closes handles on
    process death whatever the handle was doing — not on having watched
    packets in flight.
  - the routes were looked for **by their prefix**, so a route on some other
    prefix still naming the dead interface would not have shown. A
    `Get-NetRoute` with no filter at all, before and after, is what catches a
    route that outlived its interface — which is the same class of fault as
    Linux's plain-`linkdown` case: a destination that resolves to nothing.
  - the state was read **once**, about three seconds after. A graceful close
    removes the adapter and its routes in an order the code chose; a kill
    leaves that order to the kernel, so there may be a brief window where the
    adapter is gone while a route still names it. Small, silent, and only
    visible if the state is sampled several times rather than once.

- **A session is what brings the adapter up**, which took three runs to see
  and is worth knowing before anybody wonders why nothing is routed. An
  adapter that has been created and has no session is `Disconnected`, and
  Windows does not route through a disconnected interface; with
  `WintunStartSession` held it is `Up` at a nominal 100 Gbps. So **holding the
  session is the switch**, which is the same shape as Linux, where holding the
  device is.

  The first attempt at the kill experiment missed that and was therefore
  vacuous: the route was `Alive` on the adapter, nothing was using it because
  the adapter was `Disconnected`, and "after the kill, traffic went back to
  the real interface" was true of traffic that had never left it. The second
  attempt asserts the precondition and says so when it fails, which is what
  caught it.

- **`Find-NetRoute` is not a usable instrument for "is this route in use".**
  Asked about an address inside a live `/24` on an interface that was `Up`, it
  answered **"no route"** — where with the adapter `Disconnected` it had
  named the real interface. Whatever it is doing, it cannot be read as route
  selection, and the only instrument that answers that question is a holder
  that reads its session and says what arrived.

##### Keeping one program, three ways

1. **Hide the rest in the window.** Cheapest, and worth almost nothing: the
   traffic is still captured, still decrypted, still stored. It solves a
   crowded list and not one other item above.
2. **Decide at the `SYN`, and splice the rest through.** At the `SYN` the stack
   holds the client's address and port, which is what
   [`apps`](../../crates/proxy/src/apps.rs) needs to name the program. A
   program nobody chose has its bytes relayed straight to the destination over
   a socket pinned with `IP_UNICAST_IF` — no CA in the middle, no certificate
   error, nothing stored. This is the filter worth having, and it needs three
   things: the `apps/windows.rs` look-up change above, a wait for the answer
   that look-up thread gives asynchronously, and the splice itself, which is
   platform-independent and so testable on Linux. One thing to check on
   Windows rather than assume: that the TCP table already holds the row when
   the `SYN` arrives — the socket is created before the `SYN` is sent, so it
   should be there in `SYN_SENT`. What it does **not** do is lower any cap
   above: the packets are ours either way.
3. **Narrow the routes.** The only filter the operating system enforces on our
   behalf. Instead of `0.0.0.0/1` and `128.0.0.0/1`, add routes for the hosts
   being debugged alone, and the rest of the machine never enters the adapter:
   no broken pinning, no refused datagrams, no cost. It cannot say "this
   program", only "these addresses" — and for "I am debugging one API" that is
   the better answer anyway.

The window already has a list of apps, from the macOS extension
([`mac_apps.rs`](../../crates/app/src/mac_apps.rs), `Way::Apps` in
`settings_page.rs`). Reusing it here is right, and the trap is that the same
list would mean two different things: on macOS the operating system hands over
only those apps, while on Windows everything is captured and only those are
kept. Whatever the tab says has to make that difference impossible to miss, or
somebody will read "only Chrome" and believe the rest of the machine was never
touched.

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

**It starts somewhere else now.** The plan written here was a root helper
loading a `BPF_PROG_TYPE_CGROUP_SOCK` program, keeping process names in a BPF
map and sending the sockets that match to a TUN device. That was written
before the Windows work, and the Windows work built everything above the
device: the pump ([`tun.rs`](../../crates/proxy/src/tun.rs)), the stack, and
the sieve that decides at the `SYN` which connections are kept and splices the
rest. All of it is platform-independent. So the Linux mode is an
`impl Device` over `/dev/net/tun` and the setting up around it, not an eBPF
program — and it shares the deciding with Windows rather than doing it a
second way in the kernel.

The eBPF plan's objections stand against the eBPF plan: kernel 6.8, past the
glibc 2.35 floor the `.deb` aims at, outbound connections only, process names
matching on 16 characters, containers needing `--network host`, WSL unable to
do it at all. None of them is an objection to a TUN device.

**What is in:** three of the pieces, each with its own section below.
[`tun_linux.rs`](../../crates/proxy/src/tun_linux.rs) — the device, its four
methods, and an `eventfd` beside the descriptor so that `shutdown` ends a read
that is waiting on a quiet machine; three tests that need no privilege and one
that needs a device, which is `#[ignore]`d and documented like the Wintun one.
Then the two halves of staying out of our own adapter: the rows of
[`roads.rs`](../../crates/proxy/src/roads.rs), which choose the way out, and
the socket option in [`upstream.rs`](../../crates/proxy/src/upstream.rs),
which applies that choice. And
[`adapter_linux.rs`](../../crates/system/src/adapter_linux.rs): the one
privileged step, and the unprivileged reading that decides whether it has
anything to do.

#### What a Linux machine said, and the Windows answer it is not

Measured on 2026-11-16 with a control in the same transcript, because a step
that succeeds is the one nobody re-reads:

| What | Privilege | What happened |
|---|---|---|
| create a new TUN | none | `EPERM` |
| `ip tuntap add dev … mode tun user <uid>` | root | `tun persist user 1000` |
| `TUNSETIFF` on **that** device | **none** | **opened** |
| `TUNSETIFF` on a *different* name | none | `EPERM` again |
| `ip addr add`, `ip link set up` | none | not permitted, exit 2 |

The fourth row is what makes the first three mean anything: the open succeeds
because the device was created persistent and owned, not because the process
had gained something.

So **the pump stays in tracewing, unelevated**, and only the setting up needs
privilege — create the device owned by this user, give it an address, bring it
up, route into it. Windows could not have that: there, opening an adapter
needs an administrator too, so the pump itself has to be elevated.

**The carrier follows whoever holds the device open.** With the address and
route in place, an unprivileged process holding the descriptor takes the link
`UP` and the route live. When that process goes the link reads
`<NO-CARRIER,…,UP>` — the administrative flag `ip link set up` wrote stays,
and the **carrier** drops — and the route reads `linkdown`, so nothing is
routed into a device nobody is reading.

**Measured for a hard kill as well, not only a clean exit.** `SIGKILL` to the
holder leaves exactly that state: the kernel closes the descriptor whatever
killed the process. So the capture ends when tracewing ends, however it ends,
with nothing to undo — the piece that is dangerous to get wrong on Windows,
where the equivalent is so far measured only for a graceful close.

**A device takes one holder.** A second `TUNSETIFF` on a device somebody has
answers `EBUSY`; the live tests found it by running in parallel on one device.
The error for that case says so rather than telling somebody to create a
device that already exists, and a tracewing that crashed leaves nothing to
clean up, since the kernel closes the descriptor while the device, being
persistent, waits for the next run.

#### Which way out is not ours

The choosing is [`roads::out_by`](../../crates/proxy/src/roads.rs), shared
with Windows: Linux supplies rows and reuses the judgement rather than
writing a second one. `rows_v4` and `rows_v6` read `/proc/net/route` and
`/proc/net/ipv6_route` as **text**, so their tests run on every platform
rather than only where `/proc` is.

Two things the real tables of an ordinary machine taught the parser:

- **A route in the table is not a route in use.** An unreachable `::/0` over
  `lo` sits there with `RTF_UP` clear. Taken as a way out it is a default
  route — the widest there is — and tracewing's own connections would be
  pinned to the loopback. The rows are filtered on that flag, and the test
  carries the real line.
- **`luid` and `interface` hold the same number here.** Linux numbers an
  interface once for both families, where Windows numbers it per family and
  identifies it by a LUID spanning them. The distinction exists because
  Windows needs it.
- **A route that is up is not a route that works**, which took a third
  reading to see and is the one the flag cannot answer. A device that is up
  with its **carrier** down keeps its routes, and `/proc` lists them with
  `RTF_UP` set: that is the `dead linkdown` state, and the tables say nothing
  about it. So the carrier is read too, from
  `/sys/class/net/<iface>/carrier`, and anything but a plain `1` — including
  unreadable — is not a way out. That direction is deliberate: a family left
  with no way out at all is `Out::Nowhere` in `upstream.rs`, which refuses
  the connection and says so, where a dead interface kept is pinned to and
  waited on. A loud failure beats a silent wait.

  Measured both ways round, with a second device up, carrier down, and a
  default route of metric 40 beside the real one's 600. Without the carrier
  read, `out_by` chose **that** interface while the kernel routed through the
  Wi-Fi: `way out v4: Some(48)` against the kernel's index 6. With it,
  `Some(6)`. An unplugged cable with a static route is enough to arrange it,
  and a connection pinned to an interface the kernel is ignoring **hangs
  rather than fails** — 0.84s against a 3s timeout — so this was the quiet
  failure the whole seam exists to prevent, reachable by accident.

What this cannot see: both files are the **main** table, so a machine routing
by `ip rule` has routes that are not here — the same class of gap as an
interface's own metric being absent from a Windows row.

#### Keeping our own connections out

Applying the exclusion the choosing above answers, in
[`upstream.rs`](../../crates/proxy/src/upstream.rs). Without it every
connection tracewing makes on a captured program's behalf is routed back into
the adapter it is reading, which is the dangerous one on both platforms.

Almost all of it is the Windows code widened rather than a second
implementation: the flag, the two indices, the decision `out_from` takes on
them, and the connector branch that builds its own socket once an adapter is
held. Only the one call differs, and on Linux it is `SO_BINDTOIFINDEX` — one
option at the socket level, the same for both families, **no byte order to get
wrong**, where Windows wants the index in network order for `IP_UNICAST_IF`
and as it is for `IPV6_UNICAST_IF`.

Three things measured rather than assumed, each on an ordinary socket with
`CapEff: 0000000000000000`:

- **It needs no privilege.** Setting it **worked**, `SO_MARK` in the same
  transcript answered `EPERM`, and `SO_REUSEADDR` worked as a control. `man 7
  socket` on that machine names a capability for `SO_MARK` — "CAP_NET_ADMIN or
  CAP_NET_RAW (since Linux 5.17)" — and names none for binding a socket to an
  interface. Measured on 7.0.0; older kernels did require `CAP_NET_RAW` for
  `SO_BINDTODEVICE`, and which kernel stopped is a changelog nobody here has
  read, so this is written as what was measured and what the page says rather
  than as a guarantee. It matters beyond the one option: had it needed a
  capability, the pump could not have stayed unprivileged, and Linux would
  have been pushed into the elevated shape Windows is in.
- **The option does not check the index.** Setting 9999 succeeds and reads
  back 9999; `ENODEV` arrives later, at `connect`. So `Ok` here means the
  option was set, not that the interface exists — which is why the index comes
  from the tables above rather than from a guess.
- **The failure that matters is quiet.** An index that exists but is the wrong
  one does not fail at all: pinned to this machine's Wi-Fi interface a
  connection to its gateway took 0.84s, pinned to `lo` it timed out after 3s
  with nothing said. The loop this seam exists to prevent arrives as a hang,
  not as an error — worth knowing before somebody debugs one.

The tests are the round trip the Windows side uses, on both families and on
both kinds of socket, one of them bound before the option rather than after;
they need no privilege, so CI runs them. The ablation is the Windows trap
transplanted: byte-swapping the index makes the read-back say 16777216 where 1
was set, and both tests fail.

#### The privileged work is one step, and holding the device is the switch

The address and the routes need `CAP_NET_ADMIN`: as an ordinary user both `ip
addr add` and `ip link set up` answered `RTNETLINK answers: Operation not
permitted`, and no knob changes that. The question was what that costs
tracewing — netlink in-process, a dependency this workspace does not have; a
shell script through polkit, which this workspace already does, with `nft` for
the HTTP/3 block in
[`quic_linux.rs`](../../crates/system/src/quic_linux.rs) and with `trust`,
`update-ca-certificates` and the rest behind `run_as_admin` in
[`linux.rs`](../../crates/system/src/linux.rs); or an elevated process that
stays running, which is the shape Windows is in.

It costs none of them. All of it is **one step, run once**, and afterwards
tracewing's unprivileged hold on the device is the switch:

```sh
ip tuntap add dev tracewing mode tun user $UID
sysctl -w net.ipv4.conf.tracewing.ignore_routes_with_linkdown=1
sysctl -w net.ipv6.conf.tracewing.ignore_routes_with_linkdown=1
ip addr add 10.77.0.1/24 dev tracewing
ip addr add fd00:77::1/64 dev tracewing
ip link set tracewing up
ip route add default dev tracewing metric 50
ip -6 route add default dev tracewing metric 50
```

What this machine said, with `CapEff: 0000000000000000` in the process doing
the holding every time:

- **The addresses and the routes outlive the holder.** Opened, closed and
  opened again, the device keeps both addresses and both routes. Only the
  carrier follows the holder, and it drops on a clean `shutdown` and on a
  `SIGKILL` alike.
- **A route down a device nobody holds is used, and the packets vanish.** At
  the knob's default of 0, `sendto` to an address routed down the unheld
  device **succeeds**, says nothing, and goes nowhere. Routes left in place
  would mean a machine that blackholes whenever tracewing is not running.
  That, and not tidiness, is what the knob is for.
- **At 1 the kernel skips the route instead.** The table marks it `dead
  linkdown` rather than `linkdown`, and a lookup falls through to the real
  interface: with a **default route of metric 50 down the unheld adapter above
  the real one**, `8.8.8.8` still resolved to `via 192.168.1.1 dev wlp8s0`,
  `2001:4860:4860::8888` to `via fe80::1 dev wlp8s0`, and a DNS lookup worked
  while they did. Held, the same lookups choose the adapter.
- **The order matters, and that is the trap.** Set *after* the carrier had
  already dropped, the knob left the IPv4 route plain `linkdown` and still
  chosen, while IPv6 fell through — which read as "IPv4 does not support
  this" until the order was the thing that changed. Set before the routes are
  installed, both families behave alike. So the two `sysctl` lines belong
  above the addresses rather than beside them, and the word `dead` in `ip
  route show` is how to tell which of the two happened.
- **The device is not quiet once it is up.** The moment it is held the
  machine's own multicast arrives: IGMP to `224.0.0.22`, SSDP to
  `239.255.255.250` and `ff02::c`, MLDv2 to `ff02::16`. A packet off the
  device is no evidence that anything was captured, so the measurement that
  proves capture carries bytes to recognise — 51 bytes, `10.77.0.1 ->
  192.0.2.1`, our own payload inside.

So the mode turns itself on and off: holding the device raises the carrier,
letting go lowers it, and a tracewing that is not running leaves a machine
that routes normally. A `SIGKILL` on the holder does the same as a clean
`shutdown` — same `NO-CARRIER`, same `dead linkdown`, same immediate
fall-through — so the recovery does not depend on anything of ours running,
which is the only reason to trust it.

**Both platforms end up there, by opposite mechanisms.** Windows removes the
adapter when the process dies and the routes go with it, measured by a
`Stop-Process -Force` in the section above; Linux keeps the device, the
addresses and the routes, and stops using them. Removing and ignoring are not
the same guarantee — Windows has nothing left to be wrong about, while Linux
depends on a knob being right — but both leave a crashed tracewing with a
machine that routes as it did. The nftables block gets there a third way:
`flags owner` has the kernel drop the table when the `nft` holding it exits.
That property is the one to insist on for anything privileged this mode
adds.

What stays open is only **who runs the step**. The `.deb`'s `postinst` makes
the device part of installing tracewing; `run_as_admin` asked the first time
somebody switches local mode on keeps a tracewing nobody switched on from
owning an adapter, and reuses a dialog this codebase already shows. The second
is the one to write unless packaging says otherwise.

#### Looking before asking

`make_adapter` reads the adapter first and asks for a password only when
something is missing. A dialog for a machine that is already set up is how
people learn to click dialogs away, and this mode will show one often enough
without that.

Every part of the reading is unprivileged, measured with `CapEff:
0000000000000000` in the process doing it: `tun_flags` and `owner` under
`/sys/class/net/<dev>/`, both `ignore_routes_with_linkdown` under
`/proc/sys`, the routes from `/proc/net/route` and `/proc/net/ipv6_route`,
the IPv6 address from `/proc/net/if_inet6`, and the IPv4 address by
`SIOCGIFADDR` and `SIOCGIFNETMASK`. With the adapter made, all six parts read
true; with nothing there, `Look::missing` names the five that are gone rather
than saying "something".

Four things the real tables taught it, three of which would otherwise have
passed:

- **`/proc` cannot see `dead linkdown`.** With nobody holding the device, the
  default route is in `/proc/net/route` with `RTF_UP` set and nothing to say
  the kernel is ignoring it. That is what makes the reading possible before
  the mode is switched on — and it is why this answers **installed**, never
  *in use*. It is also a **bug in
  [`roads.rs`](../../crates/proxy/src/roads.rs)**, measured rather than
  suspected: with a second device up, its carrier down, and a default route
  of metric 40 beside the real one's 600, `roads_now` and `out_by` chose
  **that** interface — index 44 — while the kernel itself routed through the
  Wi-Fi, index 6. An unplugged cable with a static route is enough to arrange
  it. Pinned to an interface the kernel is ignoring, a connection **hangs
  rather than fails** (0.84s against a 3s timeout, measured for the
  exclusion), so this is the quiet failure that whole seam exists to avoid.
  That is **fixed**: `roads_now` reads
  `/sys/class/net/<iface>/carrier` too, the same file this reading already
  uses, and the measurement is in "Which way out is not ours" above.
- **A destination of all zeros is not a default route.** `0.0.0.0/1` and
  `::/1` start at the same address and cover half of it — and are not a
  hypothetical: the two `/1` halves are exactly how the Windows side covers
  everything while leaving the real default alone. The mask and the prefix
  length are what tell them apart, which the first version of this checked
  for neither; the ablation found it, because both checks could be deleted
  with every test still passing.
- **The IPv4 address needs the `ioctl`.** `/proc/net/route` carries the
  connected route an address implies — `10.77.0.0/24` on the device — which
  would read `10.77.0.5/24` as the adapter being ready, and the stack needs
  the address itself.
- **Held, the kernel adds its own addresses.** A link-local `fe80::` appears
  on the device the moment something holds it, along with multicast routes.
  So the check is for the address asked for rather than for any address, and
  `Look` deliberately says nothing about the carrier: that follows the
  holder, which is the whole shape.

#### The source, and what it refuses to be built without

[`tun.rs`](../../crates/proxy/src/tun.rs) was already the pump both
platforms share — two real threads, because a blocked read cannot poll a
stop, and a cascade of closing channels to end them. What it handed the
stack was a bare `Stack::new(wire)`, and that is the gap this closed:
**`Tun::new` now takes the sieve and the datagram bind as arguments rather
than builders**, because a mode that captures by *route* has no safe default
for either and both mistakes are quiet:

- with no sieve the stack keeps every connection on the machine and offers
  tracewing's certificate to all of them, which breaks programs that pin
  their own, for traffic nobody asked to see;
- with no bind every datagram the sieve does not keep is **refused** rather
  than passed on, so a captured machine loses the UDP it did not choose to
  show.

Neither fails loudly, and neither shows up in a test that only asks whether a
flow arrived. A tunnel a phone joined is the opposite case, and is why
[`Stack`](../../crates/proxy/src/stack.rs) keeps both optional: everything
sent down it was sent on purpose, so there is nothing to sift and nothing to
pass. The echo stays a builder, since `ping` left silent is a feature missing
rather than traffic going where it should not.

That gives `sieve::by_programs`, `upstream::straight_to` and
`upstream::bound_datagrams` their first caller, and the tests go through
`Found::start` rather than `Source::start` on purpose. **A source that starts
another source inside it hands down its own stamp**: `Tun` starts a `Stack`,
and the flows say `By::Tun` rather than the stack's own answer — which is
also why a phone's flows say `WireGuard` though a stack does the work. That
is now a test rather than a thing to notice: with `Tun::by` changed to
anything else, the flow arrives stamped wrongly and the test says so.

#### What is not in, and nothing captures until it is

**The engine never builds one.** `make_adapter`, `roads::watching` and
`leave_by` have no caller, so no adapter is made, no way out is pinned, and
no `Tun` is pushed into the sources. Until that lands this mode still
captures nothing — the difference is that the piece it is missing is now
wiring rather than design, and a `Tun` that would capture the machine can no
longer be written by accident.

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

   **And tracewing starts it.** `tracewing capture --app` opens the port
   on the loopback wherever there is room, makes a key for that run alone,
   installs the extension if it has to, saves the configuration with both,
   and stops the proxy when it ends
   ([`crates/system/src/extension`](../../crates/system/src/extension.rs)).
   No key is kept between runs, so a configuration a killed run left behind
   names a port and a key nobody answers for, and the extension sends every
   flow out as it would have. The window does the same from Settings ›
   Capture › Apps, with a port opened as it starts so that the list can
   change while it runs.

   **A program with no signing id of its own goes by its path.** Go's are
   all `a.out`, and a linker's are named after the file and a hash that
   changes each build, so the configuration carries `programs` as well as
   `apps`: files, and folders ending in `/`. The extension asks the system
   for the file the process runs (`proc_pidpath`, which its sandbox allows)
   only when there are any, and tracewing turns down a flow from its own
   process, which a folder holding a build of its own could hand over.
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
