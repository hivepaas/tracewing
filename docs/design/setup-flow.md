# Setting tracewing up: two surfaces

tracewing has seven ways in. It has one Setup page, and that page walks
through exactly one of them.

This is the design for two surfaces instead: a **configuration page** where
every way in has its own place, and a **wizard** that asks what somebody
wants to capture and leads them through the shortest path to it. Both are
needed, and for different people: the first for somebody who knows what they
want, the second for somebody opening tracewing for the first time.

## The seven ways in

| Way in | What it is |
|---|---|
| **System proxy** | The machine's own proxy setting. No privilege beyond the OS's own prompt; caught by whatever reads that setting |
| **A browser of its own** | A browser started in a profile of tracewing's, with the proxy and the authority already set |
| **A terminal** | A shell with the proxy and trust variables set — `tracewing env` |
| **Other devices** | A listener on this machine's LAN addresses, for a phone or another computer told the proxy by hand |
| **SOCKS5** | A port speaking RFC 1928, for what reads `ALL_PROXY` or has a SOCKS setting of its own |
| **WireGuard** | A tunnel a phone joins; everything it sends arrives, proxy setting or not |
| **Apps (macOS)** | The network extension, which takes the flows of the apps it is told of |

An eighth is being built: **local mode on Windows**, on a Wintun adapter the
operating system routes traffic into (`crates/proxy/src/tun.rs`,
`crates/proxy/src/wintun.rs`). It does not capture anything yet — an address
on the adapter, a route into it, and the exclusion that keeps tracewing's own
connections out are what is left
([capture-modes.md](capture-modes.md)) — but it matters to this design now,
because **it is a different shape from the macOS one**. macOS takes the flows
of apps it is *told of*: the user supplies a list. Windows takes what is
*routed* into an adapter: the user supplies nothing, and which programs are
caught follows from routes and exclusions. So these are two tabs with two
different middles, and the macOS tab's list of apps must not be built as if
it were the shape of both.

## What is wrong now, with the evidence

**One way in's configuration is in two places.** The system proxy's switch is
step 2 of Setup (`setup_page.rs:389`) and has no entry in Settings at all.
*Other devices* is the reverse: its switch is a Settings row
(`settings_page.rs:50`) while the address a phone needs is shown in Setup.
Somebody looking for "the thing I turned on" has to remember which page they
turned it on from.

**Three ways in share one undifferentiated list.** *Other devices*, *SOCKS5*
and *WireGuard* are three groups under one `SettingsSection::Network`, which
is also where **HTTP/3** lives (`settings_page.rs:167`). HTTP/3 is not a way
in — it is about what is done with traffic that arrives. Having it in the
same list is part of why the list does not read as a list of ways in.

**One way in has no configuration surface at all, and that is now what
blocks it.** The macOS extension reached the capture seam in #267: it hands
each flow to a port of tracewing's, signed with a key both ends hold, and the
flow is recorded as the app's. What is left of that mode is **the app's end**
— installing the extension, making the key, and saving and starting the
configuration that says which apps to take. A grep of `crates/app/src` for
`netext`, `EXTENSION_ID`, `APP_HEADER` or `handoff` finds **nothing**. So the
macOS tab is not a tidier home for settings that exist; it is the piece the
mode is waiting on.

**And the count is the real evidence.** Asked how many ways in tracewing has,
the person who built it said two. If the answer is not obvious from inside
the project, no user is going to assemble it from a page that walks through
one of them.

## Surface 1: a configuration page, one tab per way in

A tab for each row of the table above, and each tab holds **everything**
about that way in and nothing about any other:

- its switch, and whatever port or number it needs;
- the address, QR code or command somebody else has to be given;
- what it does **not** catch, in a line — the thing that sends people to a
  different tab rather than to the issue tracker;
- what it needs trusted, and a link into the trust steps;
- why it is not working, where tracewing knows: a port already held, an
  extension not installed, a LAN address that came and went.

Three rules fall out of that:

1. **A setting appears in exactly one tab.** If two ways in share a number —
   they share the proxy port — the tab that owns it says so and the other
   tab shows it as read-only with a note.
2. **What is not a way in does not get a tab.** HTTP/3, storage, decoding,
   AI: those stay where they are. This page answers "how does traffic reach
   tracewing", not "what does tracewing do with it".
3. **A tab says what tracewing cannot know.** Not every state is readable —
   see below — and a tab that guesses is worse than one that says it cannot
   tell.

## Surface 2: a wizard, by what somebody wants to capture

The wizard does not ask which way in to use. It asks **what** is being
captured, and picks the pair — a way in, and a trust step — that is shortest
for that answer. The knowledge is already written down in
[capturing.md](../capturing.md); this is it as a flow rather than a table.

| "I want to capture…" | Way in | Trust |
|---|---|---|
| A website in my browser | System proxy, or a browser of its own | The system store, or the profile tracewing makes |
| `curl`, `git`, a shell script | A terminal | `SSL_CERT_FILE`, which the terminal sets |
| A Node, Deno or Bun program | A terminal | `NODE_EXTRA_CA_CERTS`, which the terminal sets |
| A Python program | A terminal | `REQUESTS_CA_BUNDLE`, which the terminal sets |
| A Java program | System proxy **and** JVM flags | A truststore of its own — see below |
| My phone | WireGuard, or Other devices | The authority installed on the phone |
| A Mac app that ignores the proxy | Apps (macOS) | The system store |
| A Windows app that ignores the proxy | Local mode (Windows), once it captures; until then, a terminal or nothing | The system store |
| Something in Docker | A terminal for the container; the daemon is its own problem | Varies |

Each step ends in a **check**, following the one Setup already has for the
authority: tracewing says whether it worked rather than leaving somebody to
wonder. Where it cannot check — a program tracewing cannot start — the step
says what to run and what a working answer looks like.

### Java is the case worth designing properly

Java reads neither the system proxy nor the proxy environment variables, and
it keeps its own authority store. The instructions that exist today say
`keytool -importcert` or `-Djavax.net.ssl.trustStore`
([capturing.md](../capturing.md)), and the first of those is the wrong advice
to put in a wizard: it needs `sudo`, it changes a JDK that other things use,
and it is undone by the next JDK.

So the wizard should hand over a truststore **tracewing made**: a PKCS12 file
holding tracewing's authority, written into the data directory, and three
flags with the port and the path already in them:

```
-Dhttps.proxyHost=127.0.0.1 -Dhttps.proxyPort=<port>
-Djavax.net.ssl.trustStore=<data dir>/java-truststore.p12
-Djavax.net.ssl.trustStoreType=pkcs12
```

Nothing of the user's is touched, no `keytool` is needed — which matters,
because somebody running a Java program may have no JDK on their `PATH` —
and it works the same on all three operating systems.

## What tracewing cannot know, and must not pretend to

- **Whether the macOS extension is installed and running.** That is a macOS
  API, on a machine that may not be macOS at all. On Linux and Windows the
  tab says the way in is for macOS; on macOS it says what it can read and
  "cannot tell" where it cannot. Windows is the easier half of the same
  question — whether `wintun.dll` is beside the executable and whether the
  adapter opened are both things tracewing finds out by trying — so that tab
  can say more, and should not be written to the macOS tab's pattern.
- **Whether a program trusted the authority.** tracewing can check its own
  store, and Firefox's profiles, and it can offer a request to make. It
  cannot look inside a JVM or a Go binary.
- **Whether an app is using the proxy right now.** A flow arriving is the
  only proof, which is why every path ends in a check that produces one.

## Decisions this needs

1. **Where the configuration page lives.** A new top-level screen beside
   Setup and Settings, or Setup rebuilt as it with the wizard in front? The
   second is less to learn and more to break.
2. **Whether the wizard runs once or stays.** Shown on a first run only, or
   always reachable for the next unfamiliar target?
3. **Java's truststore**: written on demand from the wizard, or kept in step
   with the authority whenever that changes?
4. **The two local-mode tabs follow other people's work**, and should not
   lead it: the macOS one what #267 left to do at the app's end, the Windows
   one the Wintun capture being built now. Both are listed here so the tab
   strip has a place for them, not so their middles get designed here.
