# Capturing an app

A program's traffic comes through tracewing only when that program is told to
send it there, and programs listen in different places: a Mac or Windows app to
the system's proxy setting, a command-line tool to its environment, a browser
to either or to its own flags. Each place below takes a different set of
programs, and none of them takes all.

| The way in | What comes through | What does not |
|---|---|---|
| **The system proxy** — Set up, step 2 | Safari, Chrome and Edge as they are, Mac and Windows apps, desktop apps built on Electron (Claude, ChatGPT, Cursor, Slack, Discord) | Command-line tools; on macOS, `localhost` and `127.0.0.1` |
| **A browser on its own** — Set up › Browsers › Open on its own | Chrome, Edge, Brave, Opera, Chromium and Firefox, in a profile of tracewing's, servers on this computer included | Safari, which has no such profile |
| **A terminal** — Set up › Open a terminal, or `tracewing env` | What is started in it: curl, git, Python, Go, Deno, Bun, and AI agents such as Claude Code, Codex and Gemini CLI | Node.js before 22.21, and 23 to 24.4; Java |
| **A SOCKS5 port** — Settings › SOCKS5 | What speaks SOCKS: anything reading `ALL_PROXY`, `git` with a `socks5h://` proxy, Firefox's own SOCKS setting, proxychains | Programs that read only `HTTP_PROXY` and `HTTPS_PROXY`, and anything with no SOCKS setting of its own |
| **One app, as it is** — Settings › Capture › Apps, or `tracewing capture --app`, macOS | Whatever the app chosen sends over TCP, whether or not it reads any proxy setting: a browser, an Electron app, Node of any version, Java, Python from Apple | Connections to this computer itself (`localhost`), which macOS does not hand over; programs that carry no signing id of their own (below); UDP, so HTTP/3 |
| **Chosen programs, as they are** — Settings › Capture › Programs on Windows, Apps on Linux | The connections of the programs chosen, whether or not they read any proxy setting; their HTTP/3 too, with Settings › HTTP/3 on | Every program not chosen, which goes straight through unrecorded with its TLS never opened. Windows needs an administrator, and tracewing starts again as one; Linux asks once, to make tracewing's adapter |
| **A phone over WireGuard** — Settings › WireGuard | Everything the phone sends once it joins the tunnel, apps that ignore its proxy setting included | Apps that pin their own certificates; anything on the phone before tracewing's authority is installed there |

The last three leave the system proxy and every other app alone, which is what
to use when only one program is being looked at.

**Set up asks what you want to capture**, and then shows the steps for the one
way in that reaches it and leaves the others off the page — so a path above
that names Set up is where that thing is once the matching answer is chosen:
*a website in my browser* for the system proxy and the browsers, *curl, git, a
script* or a runtime for the terminal, *my phone* for the device guides. Every
way in, with no question asked first, is in Settings › Capture.

## The system proxy is not one setting

It is a different thing on each system, and what has to obey it differs with
it:

| | What tracewing sets | What follows it |
|---|---|---|
| **macOS** | SystemConfiguration, per network service | CFNetwork and URLSession, so Safari and Mac apps. `localhost` and `127.0.0.1` go straight out whatever it says, and so does HTTPS to any of this Mac's own addresses |
| **Windows** | WinInet, **per user** | WinInet as it comes: Chromium, Office, and .NET's `HttpClient` where it reads that configuration. **WinHTTP does not follow it** — it has a setting of its own (`netsh winhttp set proxy`) unless the program asks for the user's, which is also why a service running as SYSTEM comes through nothing. Local addresses are left out by default |
| **Linux** | GSettings for GNOME, the config file for KDE | only what reads them: Chromium, Electron, GNOME's own apps. **Linux has no proxy setting at the level of the system**, and a terminal knows nothing of this one |

So on Linux the system proxy reaches the least, and a terminal or a browser on
its own is usually the better way in.

## Browsers

**Open on its own** starts the browser in a profile kept for tracewing, pointed
at the proxy by its own flags, beside the profile the user works in. Nothing
else on the computer changes, and a server on this computer comes through too.

Safari has no profile to start that way: it comes through only by the system
proxy. Like every program on macOS's own networking, it then still sends
`localhost` and `127.0.0.1` straight to the server, whatever the proxy settings
say. A name under `.localhost` goes to this computer as well and does go
through the proxy: `http://playground.localhost:10000` instead of
`http://localhost:10000`.

HTTPS to any address of this Mac's own goes straight to it too: not only
`127.0.0.1`, but its Wi-Fi or Ethernet address (`https://192.168.1.12:8443`)
and a bridge's, such as the iOS Simulator's. Plain HTTP to those addresses
does go through, so a page can show up while its HTTPS calls do not: they
reach the server directly, working or failing on its own certificate, and
tracewing never sees them. Ask for a server on this Mac by a name: one under
`.localhost`, or a line in `/etc/hosts`.

## Command-line tools

**Open a terminal**, on Set up, opens one whose programs go through tracewing
and trust its certificate authority. For a terminal tracewing did not open — an
editor's, iTerm's, a script's — `tracewing env` says the same, for the shell to
run:

```sh
eval "$(tracewing env)"                    # sh, bash, zsh
tracewing env --shell fish | source        # fish
tracewing env | Invoke-Expression          # PowerShell
```

Set up shows that line with the full path of the program, for a terminal that
does not have `tracewing` on its `PATH`. It reads the port of the window that is
open, or of a `tracewing capture` running on the same data folder; `--port`
names another.

What it sets, and who reads it:

| Variable | Read by |
|---|---|
| `HTTP_PROXY`, `HTTPS_PROXY` (and in lower case) | curl, wget, git, Python, Go, Rust's reqwest, Deno, Bun, Node from 22.21 and 24.5 |
| `NO_PROXY` empty | the same: a server on this computer goes through as well |
| `SSL_CERT_FILE`, `CURL_CA_BUNDLE`, `REQUESTS_CA_BUNDLE`, `GIT_SSL_CAINFO`, `AWS_CA_BUNDLE` | OpenSSL and what is built on it, curl, Python's `requests`, git, the AWS CLI: a file of the system's roots with tracewing's authority added, so that hosts tracewing does not decrypt still check out |
| `NODE_EXTRA_CA_CERTS`, `DENO_CERT` | Node and Bun, Deno: tracewing's authority, added to the roots they have |
| `NODE_USE_ENV_PROXY=1` | Node, which reads `HTTPS_PROXY` only when asked to |

What does not come through, although started in such a terminal:

- **Node.js before 22.21, and 23 to 24.4.** These send every request straight
  out whatever `HTTPS_PROXY` says (24.0 to 24.4 send `fetch` through, but not
  `http` and `https`, which most libraries use). Set up says so when the Node a
  terminal runs is one of them, and so does `tracewing env`. `nvm install 22`
  or a newer Node puts it right.
- **Python from Apple, `/usr/bin/python3`.** It comes through, but its LibreSSL
  ignores `SSL_CERT_FILE`, so `urllib` turns down tracewing's certificates.
  `requests` and `httpx` load the file themselves and work; so does Python
  from python.org, Homebrew or uv.
- **Java**, which reads neither the proxy variables nor the trust ones. Set
  up, with *a Java program* as the answer, writes a truststore of Java's own
  — `java-truststore.p12` in tracewing's data folder — and gives the flags
  that point a JVM at it, port and path filled in. Nothing of yours is
  touched: not the JDK's `cacerts`, which takes an administrator, is shared
  with everything else on the machine, and is replaced by the next JDK. No
  `keytool` is needed either, which matters because somebody running a Java
  program may have no JDK on their `PATH`.

  One of those flags is `-Dhttp.nonProxyHosts=`, and it is not decoration. A
  JVM's default is `localhost|127.*|[::1]|0.0.0.0|[::0]`, so without it a
  program talking to a server **on this computer** — which is most of what
  anybody debugs — goes straight past tracewing, and the only sign is HTTPS
  failing against that server's own certificate instead of tracewing's.
- **Docker**, where the daemon and a container are two different problems.
  The daemon's own traffic — pulling an image — does not inherit the
  terminal's environment: on Linux it takes a systemd drop-in, and on Docker
  Desktop its own proxy settings. A container's traffic takes the variables
  set inside the container, tracewing's authority in the image, and a route to
  this machine: `--network host` on Linux, `host.docker.internal` on Docker
  Desktop, which has no host networking of that kind.
- **A Windows service running as SYSTEM**, which reads neither this terminal
  nor the user's own proxy setting: `netsh winhttp set proxy`.
- **WSL**, which has a network stack of its own: set the variables in the WSL
  shell, pointed at the Windows host's address rather than `127.0.0.1` —
  unless it runs with `networkingMode=mirrored` (Windows 11 22H2 and after),
  where `127.0.0.1` does reach the host.

## A SOCKS5 port

Some programs speak SOCKS and not HTTP proxying, and some speak both but are
more easily told about SOCKS. **Settings › SOCKS5** opens a port for them: off
by default, 1080 unless another number is given, and on this computer only
whatever **Other devices** says — a SOCKS5 port asks for no password and will
connect wherever a client names. It is read when tracewing starts, so the
switch takes effect at the next start.

```sh
ALL_PROXY=socks5h://127.0.0.1:1080 curl https://example.com
git -c http.proxy=socks5h://127.0.0.1:1080 clone https://example.com/repo.git
```

The `h` is worth the keystroke: `socks5h://` leaves the name for tracewing to
look up, so the flow is recorded as `example.com`, while plain `socks5://`
resolves the name in the client and sends an address, which is what the flow
is then named by. Firefox does the same thing with **Proxy DNS when using
SOCKS v5** in its own Network Settings, and proxychains with `proxy_dns` in
its configuration file.

This is another place to tell a program about a proxy, not a way around a
program that cannot be told at all. Those are in
[design/capture-modes.md](design/capture-modes.md), and how all of these are
meant to be presented is in [design/setup-flow.md](design/setup-flow.md).

## One app, as it is (macOS)

Every way in above tells a program where the proxy is. On a Mac, tracewing can
instead have macOS hand it an app's connections, with nothing in the app told
and nothing else on the computer changed: its network extension, a
transparent proxy that takes the flows of the apps it is given and leaves
every other flow alone.

Windows and Linux reach the same place by a different road: an adapter of
tracewing's own, and routes that send this machine's traffic into it. One
tab and one answer in Set up cover both, and which machinery is underneath
is what nobody should have to know; this section is the macOS half, and
[local mode](design/capture-modes.md) is the other.

**Settings › Capture › Apps** — or Set up, answering *an app that ignores
the proxy* — has the list. The tab is named for the word the system uses, so
it reads **Programs** on Windows and **Apps** on macOS; it is one way in
either way, as one *System proxy* tab covers three operating systems'
different machinery. **Add an app…** picks one from Applications, or a
program such as `/usr/bin/curl`, and **Turn on** has macOS hand their
connections over until it is turned off or tracewing closes. The list is
kept; the switch starts off each time, as the system proxy's does.

- **An app is taken whole.** macOS names a flow by the signing id of the
  process that opened it, and a browser or an Electron app connects from a
  helper (`com.google.Chrome.helper`, `….helper`), not from the app. So
  adding an app reads the signing ids of everything inside it that runs —
  apps, XPC services, extensions, helper programs — and the list says how
  many came with it. Each flow is recorded as the program that made it, with
  its process id.
- **Not `localhost`.** macOS gives a transparent proxy no connection to this
  computer itself, so a server on this Mac is reached by the system proxy, a
  browser of tracewing's own or a terminal instead.
- **Only tracewing.app in `/Applications` has the extension**, signed and
  notarized; a build of your own says so. The first time, macOS asks for the
  extension to be allowed (System Settings › General › Login Items &
  Extensions › Network Extensions) and for tracewing to add a proxy
  configuration (Allow). `tracewing extension status` says where it is.
- **The app still has to trust tracewing's authority** to have its HTTPS
  read: the CA in the keychain for a Mac app, `NODE_EXTRA_CA_CERTS` or its own
  setting for the rest, as below.
- **A program with no name of its own is taken by where it is.** Every Go
  program `go build` makes is `a.out`, and what a linker signs — `cargo
  build`'s, Homebrew's — is named after its file and a hash that changes with
  each build (`git-55554944…`). So such a program, picked by its file, goes
  by its path, and a **folder** picked takes every program under it: the
  folder `go build -o` or `cargo build` writes to keeps working across
  rebuilds. A link is followed when it is turned on, so Homebrew's
  `/opt/homebrew/bin/git` follows the version it points at. `go run` builds
  somewhere new each time, so it is the one thing this cannot reach.
- When tracewing closes, or the capture stops, so does the proxy. If tracewing
  is killed instead, the extension finds nothing answering for tracewing and
  every flow goes out as it would have; `tracewing extension uninstall`
  removes it.

The same from the command line, with no window, names the signing ids itself
— `codesign -dv /usr/bin/curl 2>&1 | grep Identifier` says one — or the
programs and folders by path, and records until Ctrl+C:

```sh
/Applications/tracewing.app/Contents/MacOS/tracewing capture \
  --app com.apple.curl --app com.anthropic.claude-code \
  --program ~/dev/api/bin
```

## A phone, over WireGuard

Every way in above tells a program where the proxy is. A phone is the case
where that stops working: there is one proxy setting for the whole device, an
app is free to ignore it, and plenty do. **Settings › WireGuard** takes the
other road — the phone joins a tunnel, and everything it sends arrives here
whether or not any app agreed to it.

Off by default. Switched on, tracewing serves a WireGuard tunnel on 51820
unless another number is given, and Settings shows a code to scan: install
WireGuard from either store, **Add tunnel › Create from QR code**, and point
the camera at it. The same configuration is beside the code as text, for a
phone that would rather be pasted into than aimed. The keys are made on the first run and kept in the system's
keychain — the same place the root CA's key goes — so the phone keeps working
across restarts and the configuration shown today is the one it was set up
with. It is read when tracewing starts, so the switch takes effect at the
next start.

**This port is open to your network, and it has to be:** the phone reaches
this computer across the Wi-Fi, so loopback would serve nobody. What makes
that safe is the protocol rather than the setting. An unauthenticated
datagram is answered with silence — a scan of the port gets nothing at all,
not even a refusal — and only the one phone whose key is in that
configuration can open a session. That is a stronger door than the SOCKS5
port has, which is why that one is loopback-only and this one is not.

Two things to know before relying on it:

- **The certificate still has to be trusted on the phone.** The tunnel
  decides what reaches tracewing, not what an app is willing to believe.
  Install tracewing's authority on the phone as well, and expect apps that
  pin their own certificates to fail rather than be read.
- **The tunnel carries everything**, including the phone's own background
  traffic. That is the point, and it also means a busy phone fills a session
  quickly.
- **Only TCP is captured, and QUIC is refused rather than ignored.** What
  tracewing can read is TCP, so a QUIC or HTTP/3 connection — which is UDP —
  is answered with "port unreachable" instead. That is deliberate and it is
  the difference between an app that works and an app that hangs: a datagram
  dropped in silence is retried until it times out, where a refusal arrives
  at once and the app opens a TCP connection instead, which tracewing reads.
  An app that insists on QUIC and will not fall back is one this does not
  capture — but it is no longer one you cannot see: a refused attempt
  becomes a row of its own, with the server name the client sent in the
  clear and a note that it was sent back to TCP. Nothing of the attempt is
  read beyond that opening packet. The row carries no app name here, and
  cannot: the socket is on the phone. Captured on this computer, the same
  row names the program that sent the datagram. `Hide servers' offers of HTTP/3` in Settings is the same gap
  seen from the desktop.

**Names are answered here.** A phone sends its DNS down the tunnel with
everything else, and DNS is UDP: the queries would be dropped with the rest,
which does not look like a DNS fault from the phone — it looks like the whole
network has gone, because everything it does starts with a name. So tracewing
answers them itself, and resolves each name **the way this computer resolves
names**, through the operating system. Whatever this computer is set to use —
a company's resolver, a Pi-hole, systemd-resolved — the phone gets the same
answers, and tracewing holds no opinion about DNS and sends your names to
nobody of its own choosing.

Both ends' keys are tracewing's own doing, which is unusual for WireGuard and
deliberate: the alternative is asking somebody to generate a key on the phone
and type its public half in by hand. The phone's private key is in the
configuration it is given, and is kept so the configuration can be shown
again rather than once.

## Where a program looks for authorities

A program can come through tracewing and still turn its certificate down, and
the two are worth telling apart: the tables above are about what *reaches*
tracewing, and this one is about what *trusts* it. A program that carries its
own list of authorities is not helped by the authority being installed on the
computer.

| | Where it looks | What makes it work |
|---|---|---|
| Safari, Mac and Windows apps, Chrome and Edge, Go on macOS and Windows | the system's store | Set up, step 1. `SSL_CERT_FILE` is not read there, so Go on those two needs the authority installed |
| **Firefox**, every profile its own | NSS, one store per profile | Set up installs it into each profile it finds, the one **Open on its own** makes included. On Linux that needs `certutil`, and step 1 says how to install it when it is missing |
| **Python** `requests`, `httpx`, `pip` | `certifi` | `REQUESTS_CA_BUNDLE` or `SSL_CERT_FILE`, both of which a terminal sets |
| **Node.js**, Deno, Bun | roots built into the binary | `NODE_EXTRA_CA_CERTS`, `DENO_CERT`, which a terminal sets; `--use-system-ca` on a new enough Node |
| **Java** | its own store, in its own format | `-Djavax.net.ssl.trustStore`, pointed at the PKCS#12 Set up writes. A certificate in a PKCS#12 is not enough: the entry has to be a *trustedCertEntry*, and the store's password cannot be empty, or a JVM reads it as holding nothing |
| **Flutter and Dart** | BoringSSL's own list | a `SecurityContext` in the app's code |
| A **Rust** program on rustls with `webpki-roots` | roots compiled in | nothing from outside: it has to be built against `rustls-native-certs` |

## What no way in can reach

- **A program that pins a server's certificate.** tracewing can tunnel it
  untouched and show the connection, and that is all anyone can honestly do.
- **Anything that is not HTTP**: DNS, SSH, SMTP, raw TCP and UDP, WebRTC.
- **A client that insists on HTTP/3**, except a program chosen on Windows or
  Linux. tracewing reads that program's HTTP/3 at both ends when Settings ›
  HTTP/3 is on (off by default); see [design/http3.md](design/http3.md).
  Everywhere else, tracewing takes `Alt-Svc` out of responses and can block
  UDP 443 so that clients fall back to HTTP/2, and a client that refuses to
  fall back is out of reach. Taking the offers out does not reach a client
  that learns of HTTP/3 from a DNS record instead, and most of Google's hosts
  are now that. Blocking the port, or refusing the datagrams as the local
  modes do, is what keeps such a client on TCP. Out of reach is not out of
  sight: each refused attempt is a row that says which server was asked for
  and, captured on this computer, which program asked. Nothing of it is read
  past that first packet.
- **The system's own traffic** — Windows Update, `softwareupdate`, Apple
  push — which tracewing tunnels untouched on purpose: decrypting it breaks
  updates and tells nobody anything.

Most of the "does not" entries on this page have the same root: tracewing
waits for a program to be told to use it. The two ways in that take programs
as they are, with nothing told to them, avoid that. How they work is in
[design/capture-modes.md](design/capture-modes.md).

## AI agents

What an agent asks of a model is laid out on its own in a flow: the system
prompt, the messages, the tools offered, what came back, from Anthropic's,
OpenAI's, Gemini's and Ollama's APIs and those that copy OpenAI's.

- **An agent on the command line** — Claude Code, Codex, Gemini CLI, aider —
  comes through when started in a terminal like the ones above, as they read
  `HTTPS_PROXY`. Claude Code reads `NODE_EXTRA_CA_CERTS` too, and its calls
  show decrypted. One that runs on the Node.js installed on the computer, and
  does not read the proxy itself, needs a Node that does, as above.
- **An agent in an editor** — VS Code's extensions, Cursor — runs with the
  editor's environment, which is the one the editor started with. Quit the
  editor, then start it from such a terminal: `code .`, `cursor .`. An editor
  already running hands the new window to the old one, which has no proxy.
- **An agent's desktop app** — Claude, ChatGPT — comes through by the system
  proxy.
- **An agent of your own**, on an SDK: run it in such a terminal. Python's
  `anthropic` and `openai` packages, on `httpx`, and Go's take it as they are.

tracewing's MCP server goes the other way: an agent reads what tracewing has
recorded. Settings › AI tools says how to add it.

## Phones and other computers

A phone, a tablet or another computer sets its Wi-Fi proxy to this computer;
an Android app or a Flutter app needs a line or two of its own. Set up, with
*my phone* as the answer, has a guide for each.
