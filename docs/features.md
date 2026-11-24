# Features

What tracewing does, and what is still to come. **0.1.0** in the right-hand
column means the feature is in the first release. **Planned** means it is not
built yet.

Everything in 0.1.0 is free, with nothing to buy and nothing to sign in to,
and may be used at work. Whether any later feature will be sold, and which,
has not been decided. The rows below say what tracewing does, not what it
costs.

## Capture

| Feature | In |
|---|---|
| Proxy listener: HTTP in absolute form, HTTPS through `CONNECT` | 0.1.0 |
| Turn the system proxy on and off: macOS for an administrator, Windows per user, GNOME and KDE on Linux | 0.1.0 |
| macOS for a standard account, through a privileged helper (needs signed builds) | Planned |
| Put the system proxy back when tracewing quits or crashes (a watchdog, and a check at the next start) | 0.1.0 |
| Open a terminal or a browser that goes through tracewing, without touching the system proxy (environment variables; a separate profile for any Chromium-based browser or Firefox), servers on this computer (localhost) included; `tracewing env` sets the same variables in a terminal of the user's own (sh, bash, zsh, fish, PowerShell, cmd), and Set up shows the line to run with the program's full path; a Node.js that ignores them (before 22.21, and 23 to 24.4) is named on Set up and by `tracewing env`. [Capturing an app](capturing.md) says which way takes which programs | 0.1.0 |
| Listen for SOCKS5 clients too, on a port of its own (RFC 1928, no authentication, `CONNECT`): for what reads `ALL_PROXY`, `git` with a `socks5h://` proxy, Firefox's own SOCKS setting, proxychains; the name the client asked for kept as the flow's host; off by default, switched on in Settings with the port, on this computer only, read when tracewing starts | 0.1.0 |
| Serve a WireGuard tunnel a phone can join (userspace, `boringtun`, one peer): the phone installs the WireGuard app and takes the configuration Settings shows, and everything it sends is captured — apps that ignore its proxy setting included; both ends' keys made here and kept in the system's keychain, else in a file only the user can read; shown as a QR code the WireGuard app scans and as text to paste; names asked for down the tunnel answered here, resolved the way this computer resolves them; off by default, switched on in Settings with the port (51820), open to the network by necessity and to that one peer only, read when tracewing starts | 0.1.0 |
| Pause and resume capture | 0.1.0 |
| Hosts not to decrypt, tunneled untouched (Apple and Windows Update to start with) | 0.1.0 |
| Take offers of HTTP/3 (`Alt-Svc`) out of responses, so an app that learns of HTTP/3 that way stays on HTTP/2 | 0.1.0 |
| Block UDP 443 so that clients fall back from HTTP/3 to HTTP/2 | 0.1.0 |
| Show a refused HTTP/3 attempt as a flow, with the server name it asked for and the program that sent it | 0.1.0 |
| Go through another proxy, as a company's network asks: HTTP (HTTPS through CONNECT, plain HTTP passed on whole) or SOCKS5, with a user name and password (kept in the system's keychain, else in a file only the user can read), and hosts to reach straight as `NO_PROXY` has them; the server's name left to the proxy to look up.; set in Settings, kept in `relay.toml`, applied to the next connection | 0.1.0 |
| Go through a company proxy that asks for NTLM: NTLMv2 on one connection, the password never sent; a proxy that asked once is asked with NTLM straight away, plain HTTP through a tunnel then | 0.1.0 |
| Go through a company proxy with Kerberos (Negotiate), as the user signed in to Windows or with a ticket | Planned |
| Capture one app without proxy settings, on macOS: a Network Extension, apps chosen in Settings › Capture › Apps or on Set up, or `tracewing capture --app <signing id>` / `--program <path>` from the command line, each flow named by the program that made it; a program with no signing id of its own, such as what `go build` makes, by its file or its folder. [Capturing an app](capturing.md#one-app-as-it-is-macos) | 0.1.0 |
| Capture this machine without proxy settings: a Wintun adapter on Windows, a TUN device on Linux. The operating system routes by **destination**, and no route can say "this program", so what arrives is every connection that leaves this machine rather than one chosen app — which programs to keep is then a filter tracewing applies to what is already its own. A program left out is spliced straight through, so its TLS is never terminated and nothing of it is stored | 0.1.0 |
| Containers: Docker and Kubernetes helpers | Planned |
| A headless agent with a remote window, for servers and CI | Planned |

## Certificates

| Feature | In |
|---|---|
| A root CA made for this machine alone, its key kept in the system keychain (Keychain, DPAPI, Secret Service) | 0.1.0 |
| Short-lived host certificates (30 days), a key separate from the CA's, IP addresses as well as names | 0.1.0 |
| Each browser on Set up, with whether it trusts the CA (each Firefox profile on Linux) and has reached the test page; install into the stores still missing it, and open the test page in that browser | 0.1.0 |
| Remove the CA from every store that holds it, after asking; Set up says what the machine lacks (certutil and the command that installs it on Linux, a desktop with no system proxy) | 0.1.0 |
| Install and trust the CA, step by step, with a check that it worked: macOS, Windows, Linux, Firefox's own store | 0.1.0 |
| The server's real certificate chain and TLS details for every flow; a warning when it is invalid | 0.1.0 |
| Trust servers whose certificates come from an authority this computer trusts, such as a company's own, or from one put in the `trusted-servers` folder; Settings lists them, and those it left out | 0.1.0 |
| Client certificates for servers that ask for one (mTLS): a PEM certificate and key per host pattern, in Settings; a server that asks when none is given says so in its flow. The playground has an mTLS port (10444) and a client certificate to try it | 0.1.0 |
| Use a CA signed by the company's own PKI, for machines where IT forbids new roots: Settings makes a request (CSR) for the CA's key, takes in what IT signs as an intermediate CA, and sends it with what leads to the company's root after each leaf; back to tracewing's own at any time | 0.1.0 |
| Export TLS keys for Wireshark: switched on in Settings (off again at the next start), or where `SSLKEYLOGFILE` says; both connections of a flow, from the app and to the server | 0.1.0 |

## Protocols

| Feature | In |
|---|---|
| HTTP/1.1 and HTTP/2, both sides | 0.1.0 |
| Server-sent events and JSON lines (NDJSON, JSON Lines, JSON text sequences), packed (gzip, Brotli, zstd, deflate) or not, listed as they start and cut into messages as they arrive | 0.1.0 |
| Any other response still going after 2 s (long polls, logs as they are written, big downloads) listed while it lasts | 0.1.0 |
| WebSocket: see every message; opened to the server by tracewing itself, so it goes as other connections go (a relay too) and its row knows its connection, the subprotocol the server takes is the client's, and a server's refusal reaches the client | 0.1.0 |
| GraphQL: the operation name as a column (shown on request), queries laid out and colored, and variables formatted, batches and persisted queries too | 0.1.0 |
| JSON-RPC: the method as a column (`eth_call` for an Ethereum node, `getAccountInfo` for Solana, `tools/call` for MCP over HTTP), the other calls of a batch counted, and a call that failed marked although it answered 200; a socket named by the calls in its messages, while it is still open | 0.1.0 |
| Solana read as Solana: addresses and signatures as the base58 they are, lamports as SOL, well-known programs by name, and the transaction `sendTransaction` carries laid out from its base64, of either shape: the one from before SIMD-0385 and the one after it | 0.1.0 |
| Bitcoin read as Bitcoin: satoshis as BTC, and the transaction `sendrawtransaction` carries laid out from its hex, with its txid worked out from its own bytes and every output's address written the way the chain writes it, base58 or bech32 or bech32m | 0.1.0 |
| WebSocket: send a message of your own into an open socket, to the server or to the app, from the inspector's Messages; it is listed with the others; a call written out in it — `balanceOf(0xd8dA…)` — sent as the calldata it stands for, as in Compose | 0.1.0 |
| WebSocket: breakpoints on messages, each way by a rule ("Pause a socket's messages to the server" or "to the app"): each held on the Paused screen, to send on as it came, changed, or not at all | 0.1.0 |
| gRPC and protobuf: decoded with a `.proto` file or without one, and editable | 0.1.0 |
| HTTP/3 on both ends, for a program chosen in the local mode: its QUIC terminated on this machine's own adapter, the HTTP/3 in it recorded as any other flow, and the server asked over HTTP/3 too. Off by default; a program nobody chose is refused and falls back to HTTP/2, as before. [What capturing HTTP/3 takes](design/http3.md) | 0.1.0 |
| Raw TCP and UDP | Planned |

## The flow list

| Feature | In |
|---|---|
| A million flows scroll at the display's refresh rate | 0.1.0 |
| Columns to show, hide, resize and sort | 0.1.0 |
| A sidebar of apps, devices and hosts; pin a host, hide a noisy one. Each list scrolls by itself, with a scroll bar, under a heading that stays: Apps and Devices take at most a share of the height, the hosts the rest, under their filter and All hosts | 0.1.0 |
| Filter by host, method, status class, gRPC status, JSON-RPC method and type; search every URL and body | 0.1.0 |
| Sessions: name them, keep them, open them again; start a new one without restarting, which is how recording leaves a session you want to delete and how it comes back after the proxy has stopped; one that cannot be read says which it is — written by another version, or damaged — and a check reads its pages and says what SQLite found, when you ask for it | 0.1.0 |
| A size limit per session, oldest flows dropped first | 0.1.0 |

## Layout

Every panel can be moved, resized or hidden, and the window remembers it.
Dragging is never the only way: each move is also in the View menu and on the
keyboard.

| Feature | In |
|---|---|
| The inspector beside the list or below it: drag it by its handle, use the toolbar's layout buttons, or a shortcut | 0.1.0 |
| Resize panels by dragging the dividers; hide the sidebar or the inspector. In a window too small for the inspector as it was made, the inspector gives way to the list's least rather than going past the window's edge, and is as it was again when there is room | 0.1.0 |
| A crowded inspector keeps room for what it shows: its head (title, chips, timing, tabs, actions) takes at most half its height and scrolls by itself past that; narrow, the actions keep Send again, Edit and send and Copy as… and put the rest in a ⋯ menu, and the decrypt hint becomes the button's tooltip; short, the timing starts folded until it is opened or folded by hand | 0.1.0 |
| The inspector smaller or larger by a tenth of the window, at least 100 px, with the − and + buttons at the right of its title: its height below the list, its width beside it; never under 100 px, nor so large that the list has less than its least | 0.1.0 |
| Find in the inspector: the body, a CSV or TSV table's cells, the parts drawn whole (headers, query, form, cookies, connection, LLM, GraphQL, JSON-RPC), and a stream's messages (the messages that hold it, opened one at a time, it marked in the list and in the message): Ctrl+F after a press in the inspector (the list's filter otherwise), or the magnifier, either again to close it; match case, whole word, regular expression; every match marked, the one at brighter, Enter and Shift+Enter (or the arrows) to step, a fold that hides one opened and the view scrolled down and sideways to it; counted as it searches in the background, up to 10,000; Esc closes | 0.1.0 |
| Scroll bars on every part of the inspector that is longer than its pane (headers, connection, query, form, cookies, LLM, GraphQL, JSON-RPC, body): dragged, or pressed above or below the thumb to jump there. A body's lines wider than its pane go sideways, with a wheel that goes sideways, Shift and the wheel, or a bar along the bottom; the line numbers stay, and so does a note at the end of a line | 0.1.0 |
| Request and response as tabs, side by side, or one above the other | 0.1.0 |
| The layout is kept and restored at the next start | 0.1.0 |
| Settings in parts, a menu beside the one open: General (language, look), Network, Certificates, Decoding, Storage, AI tools, About; Back to defaults for the part open; the part last open is kept | 0.1.0 |
| About, where the logo at the top of the rail leads: the version (copied in one click for a problem report), the license, the code, a problem report and the release notes, and the other apps from HivePaaS (HivePaaS itself, shellwing and packwing), each with its site and its code | 0.1.0 |
| The inspector in a window of its own, for a second display: the toolbar's window button; closing that window puts it back | 0.1.0 |
| Named layouts to switch between (for example "Phone" and "API"): the toolbar's layouts button keeps the panels, their sizes, the columns shown and the inspector's own window under a name | 0.1.0 |

## The inspector

| Feature | In |
|---|---|
| Overview: timing (DNS, connect, TLS, waiting, download), the size each way, protocol on each side, the client app | 0.1.0 |
| Headers, cookies, query and form fields | 0.1.0 |
| Bodies: JSON laid out and folded by object and array, XML a tag a line and folded by element, MessagePack, CBOR, Protobuf and gRPC (with gRPC-Web) shown as the JSON they hold, Protobuf fields named by .proto files or descriptor sets chosen in Settings (the message found by gRPC path or messageType, or chosen by hand for a path), else by number; fetched from a server by gRPC reflection; written as JSON in Compose and sent as Protobuf (bodies of any size); gRPC status and trailers kept, a failed call counted as an error; CSV and TSV as a table (quoted cells, line breaks in cells; a million rows; its cells selected as text is and copied as TSV, a row or a column copied from the right-click menu; scroll bars down and sideways; its head stays at the top as the rows go under it), or as text; Pretty, Raw as sent and Hex for any body; long lines of text wrapped; HTML, JavaScript and CSS highlighted; images; gzip, Brotli, zstd and deflate decoded for display | 0.1.0 |
| JWTs decoded | 0.1.0 |
| JSON-RPC calls read: the function a `data` field names and its arguments, a signed transaction out of its RLP, with who signed it worked out from the signature, an unsigned one a dev node is asked to sign (`eth_sendTransaction`) laid out the same way by the fields it names (amounts in ether, fees in gwei, addresses with their EIP-55 capitals), the reason a call reverted, and each call of a batch with its own reply; over a WebSocket too, where a subscription and what it sends back are read from the messages | 0.1.0 |
| Arguments of any shape read, down to the calls inside a multicall | 0.1.0 |
| What a call answered read by the types it answers with, and a reply's logs said as the events they are, out of their topics and their data | 0.1.0 |
| A receipt, a transaction or a block a node answered with laid out field by field, with a transaction that was mined and did not go through saying so | 0.1.0 |
| Contracts named by ABI JSON chosen in Settings, as solc, Hardhat and Foundry write it; read again when it changes, and never fetched from anywhere | 0.1.0 |
| Bodies of 50 MB and more open without freezing the window | 0.1.0 |
| Copy as cURL; save a body to a file | 0.1.0 |
| Copy as HTTPie, `fetch`, Python `requests` and more | 0.1.0 |
| Select text in a body, headers, cookies, query, form, connection or an LLM call, and in Compose's answer; copy it or all of it from a right-click menu or the keyboard | 0.1.0 |
| Text fields have a right-click menu: cut, copy, paste, select all | 0.1.0 |
| Values read where they stand: select a number, a token or a blob and the right-click menu's first item says what it is — a Unix time in seconds, milliseconds, microseconds or nanoseconds as local time and as "4 minutes ago", ISO 8601 and HTTP-date the same way, a duration as `2h 30m`, base64, base64url, hex, percent-encoding and `\uXXXX` taken off, JSON escaped inside a string laid out, gzip, Brotli, zstd and deflate opened, a UUID's version with the time inside a v1, v6 or v7, a ULID's, Snowflake's or ObjectId's time, a bitmask as the flags it sets, a byte count as `1 MiB`, a URL and a query or form body in parts, `Basic` as its user and password, `Cache-Control` as a sentence, a status code by meaning, `traceparent` as trace and span; drawn next to the value, folded under it as a JWT's claims are, or opened as a body of its own with Pretty, Raw and Hex; chained where one holds another (`base64 → gzip → JSON`); remembered for that field on that host, so the next request reads itself; in headers, cookies, query and form fields as well as bodies | 0.1.0 |
| More of them read: a selected blob taken as protobuf off the wire with no `.proto`, or as MessagePack, CBOR or BSON, DER and PEM as a certificate or a public key, SAML out of its base64 and deflate, PASETO, a hash named by its length, bcrypt and argon2 parameters, an IP as an integer and IPv6 either way round, a CIDR as its range, a port by service, a MAC by vendor, Windows FILETIME, .NET ticks and the other epochs, a base64 image as a thumbnail, and magic bytes as `PNG, 128×128` | Some in 0.1.0 |
| Values that need a secret: a JWT's signature verified, a framework's signed cookie opened (Rails, Django, Laravel, Flask), bytes decrypted with a key given by hand | Planned |

## Changing traffic

| Feature | In |
|---|---|
| Breakpoints on requests and responses: edit, then send on | 0.1.0 |
| Compose a request; replay one, edited or not; stop one on its way, which closes its connection rather than wait out the ten minutes a breakpoint is given; a call written out in a JSON-RPC body — `balanceOf(0xd8dA…)` — sent as the calldata it stands for, by the calls every chain shares or by an ABI chosen in Settings; the headers the client works out for itself named, so what was written for them is not taken for what went; a warning when an edit has left a signed transaction's signature over bytes that are gone, with whom the signature now works out to instead — tracewing holds no keys and signs nothing; what is being composed is kept in `compose.toml` and taken up again at the next start, body and all, unless the body is larger than 256 KB or stands for bytes that are not the text shown, which it says rather than coming back with half a request; the version to ask with — as agreed, HTTP/1.1, or HTTP/2 without asking first (h2c in the clear) — which settles the leg to tracewing, the server's own leg being settled with the server as for every other flow | 0.1.0 |
| Compare two flows side by side (Ctrl/⌘-click a second row, "Compare with…", or "Compare with the call before" to the same method, host and path): method, URL, status, sizes and time; query and headers matched by name; bodies line by line, JSON (and what MessagePack, CBOR and Protobuf hold) written with its keys in order first, the words that changed in a line marked, the same lines folded; or JSON by field, only the fields that differ under the objects and arrays that hold them, items with an id matched by it | 0.1.0 |
| Rules: map to a local file or folder, map to another host (a WebSocket too, written as `ws://` or `http://`), set or remove headers, change the body (find and replace, or all of it), the status or the query, block, ask a server with HTTP/1.1 or HTTP/2 (by host, since the version is agreed as the connection is made; a cleartext connection is HTTP/1.1 either way) | 0.1.0 |
| Slow network: bandwidth, latency, packet loss, for WebSocket messages too | 0.1.0 |
| Scripts in JavaScript, run on requests and responses by a rule ("Run a script"): `onRequest` and `onResponse` change headers, body, status, URL or method, or answer with no server asked; `state` kept between calls, `console.log` shown under the rule; QuickJS, one second and 64 MiB a call, read again when the file changes. See [scripts.md](scripts.md) | 0.1.0 |
| Rule sets to import and export | 0.1.0 |
| Rule sets shared by a team | Planned |

## Phones and emulators

| Feature | In |
|---|---|
| A setup page with a QR code: the proxy address, and the CA served by tracewing itself | 0.1.0 |
| Guides for iOS trust settings, Android user certificates (with the `network_security_config` snippet) and Flutter, each with how to undo it when done; a reminder of the devices that still have the proxy set | 0.1.0 |
| Let other devices in, or out again, without a restart: tracewing follows this computer's addresses as networks come and go | 0.1.0 |
| Android emulator over ADB: set the proxy and install the CA in one step, on every device found at once (Set up every device) or on one; Undo, or closing tracewing, puts each back | 0.1.0 |
| Android 14 and later on a device adb can root, as an emulator with a Google APIs image: the CA among the system's, where apps look (the Conscrypt APEX), for apps running and those started after, until the device restarts | 0.1.0 |
| iOS Simulator (macOS), in one step: Set up the simulators trusts the CA in every simulator running with `xcrun simctl keychain … add-root-cert`, and turns the system proxy on if it is off, since that is what a simulator goes through; Set up again for one started since | 0.1.0 |
| Name devices and filter by device | 0.1.0 |

## Import, export and sharing

| Feature | In |
|---|---|
| HAR import and export, bodies whole; on request, cookies, tokens, keys and passwords left out of headers, URLs, bodies and WebSocket messages | 0.1.0 |
| Import Fiddler (`.saz`) and Charles JSON (`.chlsj`) sessions | 0.1.0 |
| Encrypted share links that expire | Planned |
| Turn captured traffic into an OpenAPI description or mocks | Planned |

## AI and automation

| Feature | In |
|---|---|
| An MCP server: AI tools can list and search flows, read one whole, and compare two (only what differs; a JSON body by the paths of its fields; the call before when no second is named) | 0.1.0 |
| The MCP server can also list, add and remove rules (each marked "AI: "), and send requests through tracewing, a recorded one again or changed; only once Settings allow AI tools to act, off by default. The window takes in a rule added this way at once | 0.1.0 |
| A view for LLM API traffic: streamed tokens and tool calls put together | 0.1.0 |
| A command line: `tracewing capture` records with no window (a port, a time limit, rules, relay and client certificates as the window has them, the CA's key in a file for a server or CI with `--ca-key-in-file`), then writes a HAR, filtered as the search box filters; `tracewing export` writes any session as HAR; `tracewing sessions` lists them | 0.1.0 |
| The network comes back by itself within a minute after tracewing is killed with the system proxy on: a watchdog under a name of its own, and behind it something the OS keeps (a systemd user timer on Linux, a scheduled task on Windows, a launchd agent on macOS that puts it back within a second). `tracewing recover` does it at once; `tracewing uninstall` undoes what tracewing did to the computer — its certificate authority out of every store that trusts it, and the system proxy back — before tracewing is removed, and the Windows installer runs it when it uninstalls | 0.1.0 |

## The app

| Feature | In |
|---|---|
| Text size, from 11 to 20 px (14 by default), in Settings › General: lists, bodies and everything around them grow with it, as when zooming; panel widths dragged at one size keep their place at another; kept in `layout.toml` | 0.1.0 |
| Fonts, in Settings › General: the interface in tracewing's own (Nunito) or the system's; code (bodies, headers, paths, numbers) in tracewing's own (JetBrains Mono) or any monospaced font on the computer, each offered drawn in itself; applied at once, kept in `layout.toml`, and tracewing's own again if a chosen font is gone | 0.1.0 |
| Back and forward, as in a browser: the mouse's own buttons, or Alt+Left and Alt+Right, through the screens, the parts of Settings and the session on show; going somewhere new forgets what was ahead, and a text field keeps Alt+Left and Alt+Right | 0.1.0 |
| Light and dark; keyboard first: Tab through fields and buttons (Enter or Space presses one), keys for the list, the inspector's sides and parts, saving a body, and what is held on Paused | 0.1.0 |
| 15 languages: English, German, Spanish, French, Indonesian, Italian, Brazilian Portuguese, Turkish, Vietnamese, Russian, Ukrainian, Japanese, Korean, simplified and traditional Chinese | 0.1.0 |
| Signed and notarized builds for macOS | 0.1.0 |
| Signed builds for Windows (the Microsoft Store too) and Linux, and an AppImage | Planned |
| Updates: a signed list of releases read once a day (on by default, Settings › About turns it off) or by Check now, signed twice (Ed25519 and ML-DSA-65) with offline keys; a copy from the Windows installer runs the next installer as it quits, a macOS app swaps itself for the next one, a `.deb` or `.rpm` is given the commands that install it, any other copy the release's page. [Updates](updates.md) | 0.1.0 |
| Licenses: Pro and Team keys, an offline license file, prices by region | Planned |
| Team: seats, invoices, packages set up in advance for IT, SSO later | Planned |
| No telemetry. Nothing leaves the machine but the traffic sent through tracewing and the daily look for a new version, which can be turned off | 0.1.0 |
