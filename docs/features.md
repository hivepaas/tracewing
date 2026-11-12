# Features

What tracewing will do, in the order it will be built. Nothing here ships yet.

**Milestones**

| Milestone | When | Goal |
|---|---|---|
| **Gate 0** | Nov–Dec 2026 | Prove the core: intercept, record and scroll a million flows. |
| **Beta** | Jan–Mar 2027 | Everything a developer needs to debug their own app, free, to a closed beta. |
| **1.0** | Apr–May 2027 | Signed builds, updates and licenses: Pro goes on sale. |
| **1.x** | Jun–Dec 2027 | The Pro depth: scripting, gRPC, WebSocket editing, more devices. |
| **2.0** | 2028 | Capture without proxy settings, teams, HTTP/3. |

**Plans:** Free has no time limit and may be used at work. Pro is a one-time
purchase with a year of updates. Team adds sharing and administration.

Gate 0 already has a prototype, kept outside this repository for now: a million
requests through the proxy at about 88k/s with 38 MiB of memory, and a GPUI list
of a million rows that keeps a 120 Hz display at full rate.

## Capture

| Feature | Milestone | Plan |
|---|---|---|
| Proxy listener: HTTP in absolute form, HTTPS through `CONNECT` | Gate 0 | Free |
| Turn the system proxy on and off: macOS for an administrator, Windows per user, GNOME and KDE on Linux | Beta | Free |
| macOS for a standard account, through a privileged helper (needs signed builds) | 1.0 | Free |
| Put the system proxy back when tracewing quits or crashes (a watchdog, and a check at the next start) | Beta | Free |
| Open a terminal or a browser that goes through tracewing, without touching the system proxy (environment variables; a separate profile for any Chromium-based browser or Firefox), servers on this computer (localhost) included | Beta | Free |
| Pause and resume capture | Beta | Free |
| Hosts not to decrypt, tunneled untouched (Apple and Windows Update to start with) | Beta | Free |
| Take offers of HTTP/3 (`Alt-Svc`) out of responses, so apps stay on HTTP/2 | Beta | Free |
| Block UDP 443 so that clients fall back from HTTP/3 to HTTP/2 | Beta | Free |
| Go through another proxy, as a company's network asks: HTTP (HTTPS through CONNECT, plain HTTP passed on whole) or SOCKS5, with a user name and password (kept in the system's keychain, else in a file only the user can read), and hosts to reach straight as `NO_PROXY` has them; the server's name left to the proxy to look up.; set in Settings, kept in `relay.toml`, applied to the next connection | Beta | Free |
| Go through a company proxy with NTLM or Kerberos | 1.x | Pro |
| WireGuard mode: phones send everything through tracewing with the stock WireGuard app | 1.x | Pro |
| Capture one app without proxy settings: Network Extension on macOS, WinDivert or WFP on Windows, eBPF on Linux | 2.0 | Pro |
| Containers: Docker and Kubernetes helpers | 2.0 | Pro |
| A headless agent with a remote window, for servers and CI | 2.0 | Team |

## Certificates

| Feature | Milestone | Plan |
|---|---|---|
| A root CA made for this machine alone, its key kept in the system keychain (Keychain, DPAPI, Secret Service) | Gate 0 / Beta | Free |
| Short-lived host certificates (30 days), a key separate from the CA's, IP addresses as well as names | Gate 0 | Free |
| Each browser on Set up, with whether it trusts the CA (each Firefox profile on Linux) and has reached the test page; install into the stores still missing it, and open the test page in that browser | Beta | Free |
| Remove the CA from every store that holds it, after asking; Set up says what the machine lacks (certutil and the command that installs it on Linux, a desktop with no system proxy) | Beta | Free |
| Install and trust the CA, step by step, with a check that it worked: macOS, Windows, Linux, Firefox's own store | Beta | Free |
| The server's real certificate chain and TLS details for every flow; a warning when it is invalid | Beta | Free |
| Trust servers whose certificates come from an authority this computer trusts, such as a company's own, or from one put in the `trusted-servers` folder; Settings lists them, and those it left out | Beta | Free |
| Client certificates for servers that ask for one (mTLS): a PEM certificate and key per host pattern, in Settings; a server that asks when none is given says so in its flow. The playground has an mTLS port (10444) and a client certificate to try it | Beta | Pro |
| Use a CA signed by the company's own PKI (a CSR), for machines where IT forbids new roots | 1.x | Pro |
| Export TLS keys for Wireshark: switched on in Settings (off again at the next start), or where `SSLKEYLOGFILE` says; both connections of a flow, from the app and to the server | Beta | Pro |

## Protocols

| Feature | Milestone | Plan |
|---|---|---|
| HTTP/1.1 and HTTP/2, both sides | Gate 0 | Free |
| Server-sent events and JSON lines (NDJSON, JSON Lines, JSON text sequences), packed (gzip, Brotli, zstd, deflate) or not, listed as they start and cut into messages as they arrive | Beta | Free |
| Any other response still going after 2 s (long polls, logs as they are written, big downloads) listed while it lasts | Beta | Free |
| WebSocket: see every message; opened to the server by tracewing itself, so it goes as other connections go (a relay too) and its row knows its connection, the subprotocol the server takes is the client's, and a server's refusal reaches the client | Beta | Free |
| GraphQL: the operation name as a column (shown on request), queries laid out and colored, and variables formatted, batches and persisted queries too | Beta | Free |
| JSON-RPC: the method as a column (`eth_call` for an Ethereum node, `getAccountInfo` for Solana, `tools/call` for MCP over HTTP), the other calls of a batch counted, and a call that failed marked although it answered 200; a socket named by the calls in its messages, while it is still open | Beta | Free |
| Solana read as Solana: addresses and signatures as the base58 they are, lamports as SOL, well-known programs by name, and the transaction `sendTransaction` carries laid out from its base64, of either shape: the one from before SIMD-0385 and the one after it | Beta | Free |
| Bitcoin read as Bitcoin: satoshis as BTC, and the transaction `sendrawtransaction` carries laid out from its hex, with its txid worked out from its own bytes and every output's address written the way the chain writes it, base58 or bech32 or bech32m | Beta | Free |
| WebSocket: send a message of your own into an open socket, to the server or to the app, from the inspector's Messages; it is listed with the others; a call written out in it — `balanceOf(0xd8dA…)` — sent as the calldata it stands for, as in Compose | Beta | Pro |
| WebSocket: breakpoints on messages, each way by a rule ("Pause a socket's messages to the server" or "to the app"): each held on the Paused screen, to send on as it came, changed, or not at all | Beta | Pro |
| gRPC and protobuf: decoded with a `.proto` file or without one, and editable | 1.x | Pro |
| HTTP/3 | 2.0 | Pro |
| Raw TCP and UDP | 2.0 | Pro |

## The flow list

| Feature | Milestone | Plan |
|---|---|---|
| A million flows scroll at the display's refresh rate | Gate 0 | Free |
| Columns to show, hide, resize and sort | Beta | Free |
| A sidebar of hosts and devices; pin a host, hide a noisy one | Beta | Free |
| Filter by host, method, status class, gRPC status, JSON-RPC method and type; search every URL and body | Beta | Free |
| Sessions: name them, keep them, open them again | Beta | Free |
| A size limit per session, oldest flows dropped first | Beta | Free |
| Compare two flows side by side | 1.x | Pro |

## Layout

Every panel can be moved, resized or hidden, and the window remembers it.
Dragging is never the only way: each move is also in the View menu and on the
keyboard.

| Feature | Milestone | Plan |
|---|---|---|
| The inspector beside the list or below it: drag it by its handle, use the toolbar's layout buttons, or a shortcut | Beta | Free |
| Resize panels by dragging the dividers; hide the sidebar or the inspector | Beta | Free |
| Request and response as tabs, side by side, or one above the other | Beta | Free |
| The layout is kept and restored at the next start | Beta | Free |
| The inspector in a window of its own, for a second display: the toolbar's window button; closing that window puts it back | Beta | Free |
| Named layouts to switch between (for example "Phone" and "API"): the toolbar's layouts button keeps the panels, their sizes, the columns shown and the inspector's own window under a name | Beta | Free |

## The inspector

| Feature | Milestone | Plan |
|---|---|---|
| Overview: timing (DNS, connect, TLS, waiting, download), the size each way, protocol on each side, the client app | Beta | Free |
| Headers, cookies, query and form fields | Beta | Free |
| Bodies: JSON laid out and folded by object and array, XML a tag a line and folded by element, MessagePack, CBOR, Protobuf and gRPC (with gRPC-Web) shown as the JSON they hold, Protobuf fields named by .proto files or descriptor sets chosen in Settings (the message found by gRPC path or messageType, or chosen by hand for a path), else by number; fetched from a server by gRPC reflection; written as JSON in Compose and sent as Protobuf (bodies of any size); gRPC status and trailers kept, a failed call counted as an error; CSV and TSV as a table (quoted cells, line breaks in cells; a million rows), or as text; Pretty, Raw as sent and Hex for any body; long lines of text wrapped; HTML, JavaScript and CSS highlighted; images; gzip, Brotli, zstd and deflate decoded for display | Beta | Free |
| JWTs decoded | Beta | Free |
| JSON-RPC calls read: the function a `data` field names and its arguments, a signed transaction out of its RLP, with who signed it worked out from the signature (amounts in ether, fees in gwei, addresses with their EIP-55 capitals), the reason a call reverted, and each call of a batch with its own reply; over a WebSocket too, where a subscription and what it sends back are read from the messages | Beta | Free |
| Arguments of any shape read, down to the calls inside a multicall | Beta | Free |
| What a call answered read by the types it answers with, and a reply's logs said as the events they are, out of their topics and their data | Beta | Free |
| A receipt, a transaction or a block a node answered with laid out field by field, with a transaction that was mined and did not go through saying so | Beta | Free |
| Contracts named by ABI JSON chosen in Settings, as solc, Hardhat and Foundry write it; read again when it changes, and never fetched from anywhere | Beta | Free |
| Bodies of 50 MB and more open without freezing the window | Beta | Free |
| Copy as cURL; save a body to a file | Beta | Free |
| Copy as HTTPie, `fetch`, Python `requests` and more | 1.0 | Pro |
| Select text in a body, headers, cookies, query, form, connection or an LLM call, and in Compose's answer; copy it or all of it from a right-click menu or the keyboard | 1.0 | Free |
| Text fields have a right-click menu: cut, copy, paste, select all | 1.0 | Free |

## Changing traffic

| Feature | Milestone | Plan |
|---|---|---|
| Breakpoints on requests and responses: edit, then send on | Beta | Free |
| Compose a request; replay one, edited or not; stop one on its way, which closes its connection rather than wait out the ten minutes a breakpoint is given; a call written out in a JSON-RPC body — `balanceOf(0xd8dA…)` — sent as the calldata it stands for, by the calls every chain shares or by an ABI chosen in Settings; the headers the client works out for itself named, so what was written for them is not taken for what went | Beta | Free |
| Compare two flows side by side (Ctrl/⌘-click a second row, "Compare with…", or "Compare with the call before" to the same method, host and path): method, URL, status, sizes and time; query and headers matched by name; bodies line by line, JSON (and what MessagePack, CBOR and Protobuf hold) written with its keys in order first, the words that changed in a line marked, the same lines folded; or JSON by field, only the fields that differ under the objects and arrays that hold them, items with an id matched by it | Beta | Free |
| Rules: map to a local file or folder, map to another host (a WebSocket too, written as `ws://` or `http://`), set or remove headers, change the body (find and replace, or all of it), the status or the query, block | Beta | Free: 3 on at once; Pro: no limit |
| Slow network: bandwidth, latency, packet loss, for WebSocket messages too | 1.0 | Pro |
| Scripts in JavaScript, run on requests and responses | 1.x | Pro |
| Rule sets to import and export | 1.0 | Pro |
| Rule sets shared by a team | 2.0 | Team |

## Phones and emulators

| Feature | Milestone | Plan |
|---|---|---|
| A setup page with a QR code: the proxy address, and the CA served by tracewing itself | Beta | Free |
| Guides for iOS trust settings, Android user certificates (with the `network_security_config` snippet) and Flutter, each with how to undo it when done; a reminder of the devices that still have the proxy set | Beta | Free |
| Let other devices in, or out again, without a restart: tracewing follows this computer's addresses as networks come and go | Beta | Free |
| Android emulator over ADB: set the proxy and install the CA in one step | Beta | Free |
| Android 14 and later on a rooted device or an emulator: install the CA as a system certificate | 1.x | Pro |
| iOS Simulator: install the CA (macOS) | 1.x | Free |
| Name devices and filter by device | Beta | Free |

## Import, export and sharing

| Feature | Milestone | Plan |
|---|---|---|
| HAR import and export, bodies whole; on request, cookies, tokens, keys and passwords left out of headers, URLs, bodies and WebSocket messages | Beta | Free |
| Import Fiddler (`.saz`) and Charles JSON (`.chlsj`) sessions | Beta | Free |
| Encrypted share links that expire | 2.0 | Team |
| Turn captured traffic into an OpenAPI description or mocks | 2.0 | Pro |

## AI and automation

| Feature | Milestone | Plan |
|---|---|---|
| An MCP server: AI tools can list and search flows, read one whole, and compare two (only what differs; a JSON body by the paths of its fields; the call before when no second is named) | 1.0 | Free |
| An MCP server that can also add rules and replay requests | 1.x | Pro |
| A view for LLM API traffic: streamed tokens and tool calls put together | 1.0 | Free |
| A command line: capture without a window, export | 1.x | Pro |

## The app

| Feature | Milestone | Plan |
|---|---|---|
| Light and dark; keyboard first: Tab through fields and buttons (Enter or Space presses one), keys for the list, the inspector's sides and parts, saving a body, and what is held on Paused | Beta | Free |
| 15 languages: English, German, Spanish, French, Indonesian, Italian, Brazilian Portuguese, Turkish, Vietnamese, Russian, Ukrainian, Japanese, Korean, simplified and traditional Chinese | 1.0 | Free |
| Signed builds: Windows (Microsoft Store and an installer), macOS (notarized, with the helper), Linux (`.deb`, `.rpm`, AppImage) | 1.0 | — |
| Signed automatic updates | 1.0 | — |
| Licenses: Pro and Team keys, an offline license file, prices by region | 1.0 | — |
| Team: seats, invoices, packages set up in advance for IT, SSO later | 2.0 | Team |
| No telemetry; nothing leaves the machine unless asked | Beta | — |
