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
   destination, not a new mechanism.
4. **`Apps::sent` names the program behind a datagram** (#358). Choosing which
   apps to capture over QUIC needs exactly that, and until #358 the sieve could
   not answer it: it had only the TCP table.
5. **The upstream connector is ours and is reusable.** hudsucker builds its
   client from `serving.connector`; so can we. An HTTP/3 front door can send
   its requests through the same `upstream.rs` — timings, certificates, traffic
   rules and all — on the day its own QUIC client is not yet written.

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

**Two.** `quic_server.rs`: a `quinn` endpoint, certificates minted through
`Over::Quic`, an `h3` accept loop, a body adapter in each direction between
`h3`'s streams and the `Body` the core already takes. Tested with an `h3`
client of its own: a GET, a POST, a large body, an upstream that fails, and a
client whose trust does not include the CA. Not yet wired to any way in.

**Three.** The packets. `datagrams.rs` sends a chosen app's UDP 443 to the
endpoint instead of refusing it, with a table beside it saying where each
datagram was really going — the UDP twin of `handover.rs`. Unchosen apps keep
today's behaviour exactly, and refusal stays what happens when HTTP/3 capture
is off. One setting, off by default, which has to be made coherent with
hiding the offers and blocking UDP 443: capturing means doing neither.
The same code serves a phone on the WireGuard tunnel.

**Four.** Upstream over QUIC, so both halves of a captured flow are HTTP/3
rather than one half being HTTP/2 on the client's behalf. The probe already
does this much; what it does not do is live inside `upstream.rs` with its
timings, its client certificates and its traffic rules.

**Five.** The rest, each its own decision: 0-RTT (proposed: **off**, and said
out loud, because early data a proxy replays is a request sent twice),
connection migration against a NAT keyed by both ends, QUIC that is not HTTP —
DNS-over-QUIC, WebTransport — which keeps the `quic` row it has now,
`NEAppProxyUDPFlow` so macOS's local mode can do this too, and the Pro gating
`features.md` already promises.

## What this will not do, and why it is said now

- **A client using real Encrypted ClientHello cannot be captured.** Without the
  name there is no certificate to mint. The attempt stays a `quic` row, and
  `quic.rs` says why no row claims whether ECH was real or the GREASE every
  Chrome sends.
- **Proxy settings and SOCKS5 cannot carry QUIC.** A browser told to use an
  HTTP proxy stops offering QUIC, and `CONNECT` does not carry datagrams. QUIC
  capture exists only in the modes that take packets by route: TUN, Wintun,
  WireGuard, and NetworkExtension once it handles UDP flows.
- **Nothing past the Initial of an attempt that is refused** is readable, now
  or later. That is not a gap to close; those keys belong to the endpoints.
- **Firefox is unmeasured.** Chrome was asked; Firefox keeps its own trust
  store and needs its own run before anything here is claimed of it.

## Numbers to keep in mind

Chrome's transport parameters say `max_udp_payload_size 1472`, and quinn's say
the same. `datagrams.rs` caps a datagram at `BODY_MAX` 1472 and drops what is
larger: that is exactly the edge, so the path must carry a full-size QUIC
datagram without clamping it, and QUIC needs at least 1200 bytes to work at all.
