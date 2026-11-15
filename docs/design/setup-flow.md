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

**And the Windows tab must never say "one app".** A route names a
destination, and no destination can express "this program", so local mode on
Windows catches **the whole machine**; choosing one program would be a filter
applied to packets that are already tracewing's, with every other program's
traffic going through tracewing in order to be decided about.
[capture-modes.md](capture-modes.md) has why that trade was turned down. This
matters here and not only there, because the Capture tabs are where somebody
would read such a promise — and
[features.md](../features.md) does make one: "capture one app without proxy
settings", for Windows as well as macOS. Since #275 the macOS half is true.
The Windows half is not, and not for want of finishing it. That line carries
a plan tier, so it is left for whoever owns the tiers rather than reworded
from here; what this design commits to is that no tab will repeat it.

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

**One way in can only be reached from the command line.** The macOS
extension reached the capture seam in #267 and got its driver in #275:
`tracewing capture --app <signing id>` installs the extension, asks macOS to
allow it, starts a proxy configuration naming those apps, and stops it when
the capture ends; `tracewing extension status` and `extension uninstall` say
how it is and remove it. All of it is `tracewing_system::extension`, which the
window could call today. A grep of `crates/app/src` for `netext`, `handoff` or
`EXTENSION_ID` finds **two doc comments in `cli.rs`** and no code. So this tab
is not a design question any more — it is a window over a mechanism whose
semantics the command line has already settled.

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

### What is built

Settings › **Capture**, a tab for each way in
([`Way`](../../crates/app/src/layout.rs)), holding what belongs to it:

- `other_devices`, `socks`/`socks_port` and `wireguard`/`wireguard_port` moved
  out of `Network` into the tabs that own them, and the WireGuard
  configuration and its QR code moved with the switch that serves it. What is
  left of `Network` is HTTP/3, which is not a way in — and a test says so, so
  that the next setting about arriving traffic does not land back there.
- the **system proxy's switch**, which was only ever on Set up, with the same
  three states: on, being turned on, and a desktop with no setting tracewing
  can change — where a sentence takes the button's place rather than a button
  that cannot work.
- what each way in **catches**, and under it what it **does not**. The second
  line is the one worth the room: somebody whose Java program shows nothing
  has a way in that cannot catch it, not a tracewing that is broken.
- for *Apps*, what is true today rather than a switch wired to nothing: on
  macOS the command line that drives the extension, and on Linux and Windows
  a line saying it is macOS that hands an app's connections over.

A tab is a place in the window's history, so Back goes from the terminal tab
to the WireGuard tab rather than out of Capture altogether, and the tab open
is kept in `layout.toml`. Everything that asks "what is on this page" —
groups, the restart notice, **Back to defaults** — goes through one function,
which is what stops a reset on the SOCKS5 tab from reaching WireGuard's
settings.

What is not built yet: Java's truststore, and the trust links out of each tab.

## What the wizard is, now that it is built

Set up **is** the wizard. It asks one question — what do you want to capture —
and then shows the steps and cards for the one way in that reaches that
answer, which is what the pile of seven cards was failing to do.

The routing lives in [`wizard.rs`](../../crates/app/src/wizard.rs) as plain
functions with tests, not in the view, because "does a Java program need the
system proxy" should be answerable without a window and because the answers
are the part that will be argued about. Eight answers, and what they come out
at:

| Answer | Way in | Trust | Can tracewing do the trust? |
|---|---|---|---|
| A website in my browser | System proxy | This computer's store | Yes |
| `curl`, `git`, a script | A terminal | The terminal sets it | Yes |
| Node, Deno, Bun | A terminal | The terminal sets it | Yes |
| Python | A terminal | The terminal sets it | Yes |
| A Java program | System proxy | A store of Java's own | **No** — the flags are given, the store is not written |
| My phone | Other devices | On the phone | No — the guides say how |
| A Mac app that ignores the proxy | Apps (macOS) | This computer's store | Yes |
| Something in Docker | A terminal | Whatever the image brings | No — tracewing cannot tell from here |

Three things that fell out of writing it down:

- **A terminal answers four of the eight.** Three of those four would
  otherwise have somebody turning the system proxy on and finding that it
  changed nothing, which is the single most likely way to conclude that
  tracewing does not work.
- **tracewing can do the trust for two of the five kinds.** The wizard says
  which, rather than showing a step as done: a step claimed as done sends
  somebody to look for a bug in their own program.
- **Blocking UDP port 443 belongs to the browser answer and to no other.**
  It is not a way in and not a setting — it is the companion of the system
  proxy, and Set up is the only place in the window that offers it, so a
  route that dropped it would have lost a feature quietly. A test now says
  which answer shows it.

An answer is kept in `layout.toml`, so the page opens where it was left, and
choosing the open answer again puts the question back — a wizard that cannot
be unanswered is a page somebody is stuck on.

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
| A Windows app that ignores the proxy | Local mode (Windows), once it captures — the machine, not one app (below); until then, a terminal or nothing | The system store |
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

## Decisions taken

1. **The configuration page is a section of Settings, not a screen of its
   own.** Settings is already a page with a menu of sections
   ([`SettingsSection`](../../crates/app/src/layout.rs)), and
   [`navigation.rs`](../../crates/app/src/navigation.rs) already keeps which
   section is open in the window's history — so a new section gets back,
   forward and a place in the history for nothing. The tab strip lives inside
   it. The two surfaces this document is about then land as: **Settings ›
   Capture** holds the tabs, **Set up** becomes the wizard, and the three
   ways in now under `Network`, together with the system proxy's switch, move
   into the tabs that own them.

   The alternative of a section per way in was turned down for the reason the
   grouping is wrong today: a menu that lists *SOCKS5* beside *Decoding* and
   *Storage* mixes how traffic arrives with what is done with it, which is the
   problem, not the fix. The cost of this choice is two levels of chrome — a
   menu on the left, tabs above — and that is the thing to watch when it is
   built.

2. **The wizard stays.** Shown on a first run, and reachable afterwards from
   the rail and from a "walk me through this" in each tab, because the second
   unfamiliar target — a Java program, a phone — is exactly when somebody
   needs leading again, and a wizard that has been used up leaves them back
   in the maze this is meant to clear. What it costs is that the wizard has to
   **read the current state and be safe to run twice**: it says the proxy is
   already on rather than turning it on again, and it never undoes a setting
   somebody chose on purpose.

3. **Java's truststore is written lazily, and checked.** Not once and never
   again, because a new authority leaves the old PKCS12 silently wrong and it
   fails as a TLS error inside somebody's JVM, which is the worst place to
   debug it. Not rewritten on every change to the authority either, because
   that writes a file for everybody who never touches Java. Instead the step
   builds it when it is opened, compares the authority's fingerprint with the
   one in the file, and writes again only when they differ.

## Still open

4. **The Windows local-mode tab follows the Wintun capture being built**,
   and should not lead it: until an adapter has an address, a route and the
   exclusion, there is nothing for a switch to switch. The tab strip has a
   place for it; its middle gets designed where the capture is.

   The macOS tab is no longer in this list. `tracewing capture --app` settled
   its shape — a list of signing ids, the install and the allow, and a
   status — and the window should wear that rather than invent a second
   model of the same thing.
