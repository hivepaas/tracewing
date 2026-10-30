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
| Turn the system proxy on and off: macOS through a privileged helper, Windows per user, GNOME and KDE on Linux | Beta | Free |
| Put the system proxy back when tracewing quits or crashes (a watchdog, and a check at the next start) | Beta | Free |
| Open a terminal or a browser that goes through tracewing, without touching the system proxy (environment variables, a separate browser profile) | Beta | Free |
| Pause and resume capture | Beta | Free |
| Choose which hosts to decrypt; tunnel the rest untouched | Beta | Free |
| Block UDP 443 so that clients fall back from HTTP/3 to HTTP/2 | Beta | Free |
| Go through another proxy: HTTP and SOCKS5 | 1.x | Free |
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
| Install and trust the CA, step by step, with a check that it worked: macOS, Windows, Linux, Firefox's own store | Beta | Free |
| The server's real certificate chain and TLS details for every flow; a warning when it is invalid | Beta | Free |
| Client certificates for servers that ask for one (mTLS) | 1.x | Pro |
| Use a CA signed by the company's own PKI (a CSR), for machines where IT forbids new roots | 1.x | Pro |
| Export TLS keys (`SSLKEYLOGFILE`) for Wireshark | 1.x | Pro |

## Protocols

| Feature | Milestone | Plan |
|---|---|---|
| HTTP/1.1 and HTTP/2, both sides | Gate 0 | Free |
| Server-sent events and other streams, shown as they arrive | Beta | Free |
| WebSocket: see every message | Beta | Free |
| GraphQL: the operation name as a column, queries and variables formatted | Beta | Free |
| WebSocket: breakpoints and sending messages | 1.x | Pro |
| gRPC and protobuf: decoded with a `.proto` file or without one, and editable | 1.x | Pro |
| HTTP/3 | 2.0 | Pro |
| Raw TCP and UDP | 2.0 | Pro |

## The flow list

| Feature | Milestone | Plan |
|---|---|---|
| A million flows scroll at the display's refresh rate | Gate 0 | Free |
| Columns to show, hide, resize and sort | Beta | Free |
| A sidebar of hosts and devices; pin a host, hide a noisy one | Beta | Free |
| Filter by host, method, status class and type; search every URL and body | Beta | Free |
| Sessions: name them, keep them, open them again | Beta | Free |
| A size limit per session, oldest flows dropped first | Beta | Free |
| Compare two flows side by side | 1.x | Pro |

## The inspector

| Feature | Milestone | Plan |
|---|---|---|
| Overview: timing (DNS, connect, TLS, waiting, download), sizes, protocol on each side, the client app | Beta | Free |
| Headers, cookies, query and form fields | Beta | Free |
| Bodies: JSON tree and text, HTML, JavaScript and CSS highlighted, images, hex; gzip, Brotli, zstd and deflate decoded for display | Beta | Free |
| JWTs decoded | Beta | Free |
| Bodies of 50 MB and more open without freezing the window | Beta | Free |
| Copy as cURL; save a body to a file | Beta | Free |
| Copy as HTTPie, `fetch`, Python `requests` and more | 1.0 | Pro |

## Changing traffic

| Feature | Milestone | Plan |
|---|---|---|
| Breakpoints on requests and responses: edit, then send on | Beta | Free |
| Compose a request; replay one, edited or not | Beta | Free |
| Rules: map to a local file or folder, map to another host, rewrite headers, body, status or query, block | Beta | Free: 3 on at once; Pro: no limit |
| Slow network: bandwidth, latency, packet loss | 1.0 | Pro |
| Scripts in JavaScript, run on requests and responses | 1.x | Pro |
| Rule sets to import and export | 1.0 | Pro |
| Rule sets shared by a team | 2.0 | Team |

## Phones and emulators

| Feature | Milestone | Plan |
|---|---|---|
| A setup page with a QR code: the proxy address, and the CA served by tracewing itself | Beta | Free |
| Guides for iOS trust settings, Android user certificates (with the `network_security_config` snippet) and Flutter | Beta | Free |
| Android emulator over ADB: set the proxy and install the CA in one step | Beta | Free |
| Android 14 and later on a rooted device or an emulator: install the CA as a system certificate | 1.x | Pro |
| iOS Simulator: install the CA (macOS) | 1.x | Free |
| Name devices and filter by device | Beta | Free |

## Import, export and sharing

| Feature | Milestone | Plan |
|---|---|---|
| HAR import and export, bodies whole, tokens and cookies removed on request | Beta | Free |
| Import Fiddler (`.saz`) and Charles sessions | Beta | Free |
| Encrypted share links that expire | 2.0 | Team |
| Turn captured traffic into an OpenAPI description or mocks | 2.0 | Pro |

## AI and automation

| Feature | Milestone | Plan |
|---|---|---|
| An MCP server: AI tools can list and search flows | 1.0 | Free |
| An MCP server that can also add rules and replay requests | 1.x | Pro |
| A view for LLM API traffic: streamed tokens and tool calls put together | 1.0 | Free |
| A command line: capture without a window, export | 1.x | Pro |

## The app

| Feature | Milestone | Plan |
|---|---|---|
| Light and dark; keyboard first | Beta | Free |
| English and Vietnamese | 1.0 | Free |
| Signed builds: Windows (Microsoft Store and an installer), macOS (notarized, with the helper), Linux (`.deb`, `.rpm`, AppImage) | 1.0 | — |
| Signed automatic updates | 1.0 | — |
| Licenses: Pro and Team keys, an offline license file, prices by region | 1.0 | — |
| Team: seats, invoices, packages set up in advance for IT, SSO later | 2.0 | Team |
| No telemetry; nothing leaves the machine unless asked | Beta | — |
