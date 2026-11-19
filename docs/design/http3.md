# Capturing HTTP/3

What tracewing does with QUIC today is **see** it. `quic.rs` reads the
ClientHello out of an Initial packet — its keys come from the clear-text
Destination Connection ID under a published salt, so anything on the path can
open one — the stack hands the attempt over as `Took::Quic`, and a row of kind
`quic` says which server was asked for and which program asked, then says the
client was sent back to TCP (#357, #358).

Reading it is a different thing. QUIC is encrypted from its second packet with
keys only the endpoints have, so capturing HTTP/3 means **terminating QUIC on
both sides**: a QUIC server facing the app, a QUIC client facing the real
server, and HTTP/3 between them. This is what that takes.

Nothing here was written from reading alone. The questions that decide whether
it is worth building are about what a browser's policies and a server's
deployment actually do, so they were asked, with
[`quicprobe`](../../crates/dev/src/bin/quicprobe.rs), on Linux with Chrome
157. Anyone can ask them again.

## What was measured

### A browser will speak HTTP/3 to a certificate from tracewing's own CA

The probe's server holds a leaf minted the way the proxy would mint it —
`LocalCa::config_for(host, Over::Quic)`, the CA's own key out of the keychain —
and offers `h3` alone over TLS 1.3 alone. Chrome, told to use QUIC for that
origin, does:

```
** QUIC from [::1]:41388 in 24.6ms, alpn "h3"
>> HTTP/3 GET https://localhost:4433/ from Mozilla/5.0 (X11; Linux x86_64) ... HeadlessChrome/157.0.0.0
>> HTTP/3 GET https://localhost:4433/favicon.ico from ...
```

It renders the body, and fetches the favicon over the same connection. The same
holds for a **public** name rather than `localhost` (`localtest.me`, which
resolves to loopback): there is no extra rule for QUIC about publicly trusted
roots or Certificate Transparency, any more than there is for TLS over TCP
today.

**A trap, because it cost an hour and looked like a verdict.** The first answer
was `ERR_QUIC_HANDSHAKE_FAILED` with nothing at all in the server's log, which
reads exactly like a browser refusing a locally-trusted certificate over QUIC.
It was the probe: the endpoint was bound to `0.0.0.0` and a browser asked for
`localhost` goes to `::1` first. Bind both families.

### The other direction works too

HTTP/3 to real servers, with the steps timed the way `upstream.rs` times a name
lookup, TCP and TLS:

| | | name | handshake | first answer |
|---|---|---|---|---|
| cloudflare.com | 301, 167 B | 6.8 ms | 72 ms | 122 ms |
| www.google.com | 200, 93 KB | 0.5 ms | 207 ms | 218 ms |
| quic.nginx.org | 200, 2.5 KB | 607 ms | 650 ms | 217 ms |

Three steps, the same shape `Upstream` already records, so an HTTP/3 connection
to a server fits the existing model of one rather than needing a new one. (One
first run showed a 5.2 s handshake to Cloudflare that never repeated; written
down rather than explained.)

### Clients choose QUIC two ways, and only one of them is `Alt-Svc`

In Chrome's own net log, one load of `google.com` picks QUIC for twelve hosts
from a **DNS HTTPS record** rather than from any `Alt-Svc` header:

```
("https://www.google.com", using_quic: True)          type: dns_alpn_h3
("https://accounts.google.com", using_quic: True)     type: dns_alpn_h3
("https://play.google.com", using_quic: True)         type: dns_alpn_h3
... twelve in all
```

Two things follow, one a correction and one a design constraint.

- **Hiding the offers cannot keep an app off QUIC by itself.** Taking
  `Alt-Svc` out of responses does nothing for a host with an HTTPS record,
  which is most of Google's and a growing share of everyone's. Only blocking
  UDP 443 — or refusing the datagrams, which the local modes do — keeps such
  a client on TCP. `features.md` was narrowed with this document, and the
  setting itself followed: it is called `Hide servers' offers of HTTP/3`
  rather than `Keep apps off HTTP/3`, and says which apps it reaches and
  which it does not.
- **A captured HTTP/3 flow may have no TCP flow before it.** A client that
  learns `h3` from DNS goes to QUIC on first contact. So the capture path must
  not assume it has already seen a TLS handshake, a `CONNECT`, or anything
  else, for that host.

A browser offered both races QUIC against TCP and takes whichever connects
first. Over loopback TCP wins every time, the QUIC attempt is cancelled
(`net_error -3`), and after a few of those the browser marks the offer broken —
which is the test bench, not the world. Real QUIC to real servers is what the
`quic` rows and the live sockets on a developer's own machine already show.

## The library: quinn, with h3 above it

`quinn` 0.11 with `h3` 0.0.8 and `h3-quinn` 0.0.10, which is the combination
`reqwest` uses for its own HTTP/3, and which the probe now exercises in this
workspace.

The reason is TLS, not QUIC. `quinn` does its TLS through **the same rustls**
this workspace already has, so one certificate path serves both front doors:
`LocalCa` mints a leaf, `Over::Quic` says what to offer over it, and the key log
Wireshark reads is the same object. The alternatives each add a second TLS
stack — `quiche` brings BoringSSL, `neqo` brings NSS — and `s2n-quic`, while
rustls-capable, is not what the client half of this workspace is already built
around. All three of `quinn`, `h3` and `h3-quinn` are MIT or Apache-2.0, which
`about.toml` already accepts.

## What the current code makes easy, and what it does not

Five things found by reading, each one shaping the plan below.

1. **The proxy core owns its accept loop and takes a `TcpListener`.**
   `hudsucker` is the core (`lib.rs:serve_port`), so HTTP/3 cannot arrive
   through it. QUIC needs a front door of its own.
2. **`hudsucker::HttpContext` is `#[non_exhaustive]`**, so nothing outside that
   crate can build one — and `FlowHandler::handle_request` takes one. The
   recording core therefore has to stop being hudsucker-shaped before a second
   front door can use it. That is worth doing on its own merits: `FlowHandler`
   is built only in `lib.rs` and so has **no unit tests at all** today.
3. **`datagrams.rs` already is a UDP NAT.** It sends a datagram out through a
   socket of its own and puts the answer back as a packet from the address it
   was sent to, with a table, an idle time and a cap. Pointing a chosen app's
   443 at an endpoint of ours instead of at the server is a change of
   destination, not a new mechanism — true as far as it goes, and step three
   says what it left out: the asking that decides, and the way back for a
   datagram the answer refuses.
4. **`Apps::sent` names the program behind a datagram** (#358). Choosing which
   apps to capture over QUIC needs exactly that, and until #358 the sieve could
   not answer it: it had only the TCP table.
5. **The upstream connector is ours**, and what it knows is reusable even
   though the connector itself turned out not to be — see step four, which is
   the second thing this list had wrong. hudsucker builds its client from
   `serving.connector`, and `upstream.rs` is where which certificates to
   trust, which to show, and what a rule says about a host all live. A second
   way out needs those facts, not that code.

## The shape: two front doors, one set of rules

```
  TCP 443  ──►  hudsucker  ──┐                        ┌──►  upstream.rs: TCP + TLS, H2/H1
                             ├──►  the recording core ─┤
  UDP 443  ──►  quinn + h3 ──┘      rules, scripts,    └──►  upstream over QUIC + H3
  (chosen apps only)                pause, flows
```

A QUIC connection carries many requests, as an HTTP/2 connection does, and the
flow model already handles that. What a captured HTTP/3 flow is, then, is an
ordinary flow whose client version says HTTP/3.

## The work, in the order it can land

**One.** The recording core stops being hudsucker-shaped: the bodies of
`handle_request` and `handle_response` move behind methods that take a client
address, and the `HttpHandler` impl becomes a shim. Ships with the first unit
tests `FlowHandler` has ever had. Nothing else in this list can start before
it, and it is worth landing even if HTTP/3 is parked again.

**Two, landed.** [`quic_server.rs`](../../crates/proxy/src/quic_server.rs):
a `quinn` endpoint, an `h3` accept loop, a body carried each way between
`h3`'s streams and the `Body` the core takes, and the request handed to
`FlowHandler::request` exactly as hudsucker's door hands one over.

The certificate is **minted per handshake**, not per host in advance, and that
was the one thing the plan above had wrong. TCP names the host in a `CONNECT`
before TLS starts, so `config_for(host, Over::Quic)` can hand rustls a config
that already holds the leaf. QUIC names it inside the ClientHello of the
Initial, by which time the endpoint is listening and its config is made — so
the endpoint holds one config whose resolver is `LocalCa::certified_for`, and
leaves are cached as themselves rather than as whole configs.

Its tests connect a real `h3` client to it: the check page over HTTP/3, two
names over one endpoint — which is what says the minting is per handshake — a
request sent on to a server, a request that reaches none, and a client that
trusts nothing, which gets nowhere.

**Three, landed.** The packets. [`datagrams.rs`](../../crates/proxy/src/datagrams.rs)
sends a chosen program's UDP 443 to the door instead of refusing it, which is
what makes everything above reachable by a browser rather than only by tests.
UDP 443 now has three answers, and which one a datagram gets depends on the
program behind it: passed on, carried to the door, or refused. The setting is
`capture_http3`, off, and it applies while the session runs.

Four things about it, three of which the plan above had wrong or did not see.

- **There is no table beside it; it is the same table.** The plan called for
  "the UDP twin of `handover.rs`" and what it needed was `handover.rs`
  itself. A captured client reaches the door over the loopback from a socket
  of tracewing's own, so `conn.remote_address()` says nothing about whose
  connection it is — which is exactly the shape of a connection a way in
  hands to the proxy port, and `Handed` is the table that answers it, read by
  `capture.rs` under exactly that address. So the catcher writes a `Known`
  there and nothing in the recording core changed at all: the way in, the
  client's real address, and the program are on the flow because that path
  already existed. An entry lives exactly as long as its pair of ends, held
  by a guard rather than remembered and forgotten in turn.
- **The refusal could not move.** Refusing is the stack's own answer and not
  something `datagrams.rs` could do: `smoltcp` holds no socket for port 443,
  and the ICMP port unreachable it sends for one is what makes a client give
  up on QUIC. So a datagram whose program nobody chose goes **back** to the
  stack, down a channel of its own, and is refused there exactly as it was
  before any of this — the row saying the attempt happened included. Two
  refusals in two places would have been two answers to drift apart.
- **The cost is a wait, and every program's QUIC pays it.** Which program
  holds a datagram socket is a question for the operating system, so the
  first datagram of each new pair of ends waits for an answer — one look at a
  table of the kernel's, a millisecond or two in the ordinary case and half a
  second at worst. A program nobody chose pays it too, and is then refused:
  that is the one way an unchosen program's behaviour is not quite what it
  was, and it is paid only while capture is on. The datagrams that arrive
  during the wait are **held**, not dropped, because a QUIC client that loses
  its first flight waits out a timeout of its own before sending it again.
- **"Capturing means doing neither" came out as: capture wins, for
  everybody.** While capture is on, servers' offers of HTTP/3 are not hidden
  from anyone — `Live::hiding_http3` is the setting and `!capture_http3`
  together — because a program never offered HTTP/3 never speaks it and the
  door would have nothing to read. Hiding them from unchosen programs alone
  would need the chosen list in the response path, which is a different
  change; what this costs is that an unchosen program may now try QUIC and be
  refused, which is what a program that learns HTTP/3 from DNS has always
  done anyway. Blocking UDP 443 still defeats capture outright, since those
  datagrams never reach tracewing, and the setting's own text says so.

Where the two halves meet is `quic_server::Opened` and `ProxyOptions::handed`:
the door is opened inside the proxy and the way in that feeds it is built
outside and before it — a local mode's adapter is made when somebody presses
the switch — so the port is told rather than agreed in advance.

### It really does capture, and that was watched

[`local_live_http3`](../../crates/app/tests/local_live_http3.rs) is the join
no unit test reaches: a real QUIC client whose datagrams leave by a route
into a real TUN device, the real stack turning them into a flow, the real
sieve asking the operating system which program sent them, the real catcher,
the real door, the real core, and the real way out asking a real HTTP/3
server. It answers 200 with the server's own body, and a second test says a
program nobody chose gets nowhere and has its attempt said once.

**It needs no password and touches nothing**, which is the part worth
reusing. The nine lines of `adapter::script` want an administrator on a real
machine, and while the device holds the default route the machine's traffic
goes into it — so this test re-executes itself under
`unshare --user --net --mount --map-root-user`. The kernel grants the
capabilities inside a user namespace of ours, so nothing asks for rights,
and the device, the addresses and the routes live in a network namespace
that goes when the process does. Two things cost a run each to find out:
`/sys` stays as the old namespace mounted it, so `adapter::look` reads the
real machine's interfaces and reports the device missing while `ip` has just
made it — hence `--mount` and a fresh `sysfs`; `/proc` needs nothing,
because the net sysctls and `/proc/net` are resolved against the reading
task's own namespace. **The same trick made `local_live_linux` runnable
without polkit** (#368), and it does **not** make either of them runnable in
CI — but the reason is not CI, which both of us said first and neither had
measured. **It is AppArmor.** On Ubuntu 24.04 and newer,
`kernel.apparmor_restrict_unprivileged_userns` is 1 and the profile an
unconfined process transitions to when it makes a user namespace carries
`allow userns` with `audit deny capability`, so the namespace is made and
the capabilities inside it are not:
`unshare: write failed /proc/self/uid_map: Operation not permitted`. TraceW 2
measured it on the runner by flipping the sysctl both ways and back (#372),
and the same refusal is one terminal away on this machine —
`aa-exec -p unconfined -- unshare --user --map-root-user true` fails here
while the plain command succeeds, because the editor these sessions run in
has a profile that grants `userns`. So both tests stay a thing somebody runs
by hand, and the one command in each file's header now says what to do about
the profile; un-ignoring either would have broken `cargo test` for a
developer in a terminal, not only in CI.

Two things are deliberately not in it. **The way out is not pinned**: a
namespace has one interface — ours — so `roads::out_by` answers nowhere and
the way out cannot be made at all, which is the loop prevention doing its
job and which the first run proved by answering 502 to everything, having
gone through the adapter, the catcher, the door and the core to get there.
**The pinning is covered by neither live test**, which this document said
until TraceW 2 corrected it: `local_live_linux` builds the source and reads
the routing table, then lets go, without ever calling `start`. What covers
which part of it, and what is joined nowhere, is written down on
`local_source::Pinned`. And the server is on the loopback rather than across
a network, which the way out's own tests cover.

What is still **unverified is a real browser** through a real adapter: its
own policies, its HTTPS-record path to QUIC, and its own trust store are
what `quicprobe` asked about in isolation and what nothing has yet asked
about through the capture path. That wants a machine with a browser and an
adapter, which is the Windows VM. The same code would also serve a phone on
the WireGuard tunnel, but nothing points it at one yet.

**Four, landed.** [`quic_upstream.rs`](../../crates/proxy/src/quic_upstream.rs):
the way out over QUIC, so both halves of a captured flow are HTTP/3 rather
than one half being HTTP/2 on the client's behalf. A name looked up, one
handshake, `h3` over it, and the connection kept for the next request to that
server — HTTP/3 multiplexes, so a page's requests belong on one connection.
The facts go in the same `Upstream` the TCP way out fills, so a flow that went
this way reads in the inspector like any other.

**It is a module of its own, not a branch of `upstream.rs`**, which is where
the plan above was wrong a second time. There is no `TcpStream` in it to hand
to hyper and nothing above it wanting one: hyper's client pools connections
and sends requests over them, and that is exactly what `h3` does instead. What
the two ways out share is passed in rather than written twice — the verifier
that carries which roots to trust, the client certificates, the traffic rules,
and `upstream::Ids`, the counter connections are numbered from. **That counter
has to be one counter**: a flow row points at its connection by number, so a
second way out counting from 1 would put two connections under one number in
the inspector.

Three things about it are worth knowing before reading the code:

- **One handshake, so one timing.** QUIC agrees the connection and its TLS
  together, so the whole handshake is the flow's connect step and its TLS step
  is empty. What the TLS was is still on the connection: TLS 1.3 because QUIC
  has no other, the ALPN, and the certificates the server showed. The cipher
  suite is the one fact missing — `quinn` hands over the protocol and the name
  and says nothing about the suite.
- **Two things are refused in words rather than done differently.** A company
  proxy cannot carry QUIC (`CONNECT-UDP`, RFC 9298, is not spoken by tracewing
  or by most such proxies), and a rule that says *ask this host with HTTP/2*
  cannot be obeyed by a way out that speaks HTTP/3. Quietly ignoring that rule
  would be worse than refusing the request: the rule is there to make the
  server answer a certain way.
- **A socket per connection, not an endpoint held.** While tracewing holds a
  TUN adapter, which interface a socket must leave by is a live answer that
  changes as the machine's routing does, and a bound socket cannot be rebound.
  So each connection asks `upstream::datagrams_out` for a socket and gets one
  pinned as the rules then say — the same pinning `datagrams.rs` uses, now
  spelled in one place.

What it does not do is race the families the way hyper's connector races them
for TCP: addresses are tried in the order the resolver gave them.

**Five.** The rest, each its own decision.

**0-RTT: off at both ends, decided and measured.** Early data a proxy replays
is a request the server may be sent twice, and nothing here is ready to answer
for that — so the door sets `max_early_data_size = 0` and the way out leaves
rustls's `enable_early_data` off, each with the reason beside it.

Asking rather than reading changed what the decision is *about*. Both settings
were already right, and neither was ever the thing that was going to go wrong:
**quinn's convenience constructors set the opposite at both ends** —
`QuicServerConfig::inner` behind `ServerConfig::with_single_cert`, and
`QuicClientConfig::inner` behind `ClientConfig::with_root_certificates` — and
it is only the `TryFrom` paths this code happens to use that let our two
values stand. A later simplification to the shorter call turns early data on,
at either end, with nothing failing.

So the test asks from the outside, where a reading could not: a client that
enables early data, keeps whatever tickets it is given, and asks for 0-RTT on
a second connection. It is refused — and refused **holding two tickets**,
which is the part worth knowing. The door does issue them, so an ordinary
resumption works and only the early data is withheld, which is the narrow
thing that was wanted rather than "no resumption". Taking the door's zero out
turns the same client's early data **accepted**, measured, so the test is
about the setting and not about the client.

What is left, each still its own decision: connection migration against a NAT
keyed by both ends, QUIC that is not HTTP — DNS-over-QUIC, WebTransport —
which keeps the `quic` row it has now, `NEAppProxyUDPFlow` so macOS's local
mode can do this too, and the Pro gating `features.md` already promises.

## What this will not do, and why it is said now

- **A client using real Encrypted ClientHello cannot be captured.** Without the
  name there is no certificate to mint. The attempt stays a `quic` row, and
  `quic.rs` says why no row claims whether ECH was real or the GREASE every
  Chrome sends.
- **Proxy settings and SOCKS5 cannot carry QUIC.** A browser told to use an
  HTTP proxy stops offering QUIC, and `CONNECT` does not carry datagrams. QUIC
  capture exists only in the modes that take packets by route: TUN, Wintun,
  WireGuard, and NetworkExtension once it handles UDP flows. The same holds
  the other way round: with a company proxy set in tracewing's own settings,
  the way out over QUIC refuses the request rather than going around the
  proxy, which is what a machine behind one would notice.
- **Nothing past the Initial of an attempt that is refused** is readable, now
  or later. That is not a gap to close; those keys belong to the endpoints.
- **Firefox is unmeasured.** Chrome was asked; Firefox keeps its own trust
  store and needs its own run before anything here is claimed of it.

## Numbers to keep in mind

One more since step three: the read for an answer coming back is **one byte
longer** than a datagram may carry. A read into exactly `BODY_MAX` truncates
anything longer and the path would then put 1472 bytes of a longer datagram
on the wire as though that were all of it, which for QUIC means a packet
whose authentication fails for a reason nothing anywhere could name. One byte
makes it a drop with a line in the log instead.

Chrome's transport parameters say `max_udp_payload_size 1472`, and quinn's say
the same. `datagrams.rs` caps a datagram at `BODY_MAX` 1472 and drops what is
larger: that is exactly the edge, so the path must carry a full-size QUIC
datagram without clamping it, and QUIC needs at least 1200 bytes to work at all.
