# Measurements

What the core was measured to do, and how each design choice was made. These
come from the prototype this code grew out of (`scripts/bench.sh`,
`scripts/disk_limits.sh` and `crates/dev/src/bin` repeat them), on a Linux desktop
with NVMe, unless a section says otherwise. The sections on the window name the
prototype's `ui/` crate, which is on its way to `crates/app`.

## Results (2026-10-29)

Linux, 32 threads, NVMe ext4, everything on loopback, 1 KiB JSON responses,
64 concurrent clients, origin with HTTP/2.

| Run | Throughput | p50 | p99 | Proxy peak RSS | Stored |
|---|---|---|---|---|---|
| 1M requests, client HTTP/2 | 88,258 req/s | 0.71 ms | 1.03 ms | 38 MiB* | 1,000,000, 0 dropped |
| 1M requests, client HTTP/1.1 | 102,045 req/s | 0.61 ms | 0.91 ms | 40 MiB* | 1,000,000, 0 dropped |
| 20 × 50 MB bodies | 26 req/s (1 GB) | 143 ms | 213 ms | 67 MiB* | 20, 64 KiB kept each |

\* Sampled from `VmRSS`, which missed the peaks. See "Peak memory, measured
properly" below for what `VmHWM` says.

The 1M-request database is 1.4 GB (64 KiB cap, so the whole 1 KiB body is kept).
Real sites (example.com, google.com, github.com) were intercepted over HTTP/2
through curl.

Two fixes the benchmark forced:

- **TCP_NODELAY.** Without it, p99 was ~42 ms and HTTP/2 managed 9.8k req/s
  (Nagle plus delayed ACK). It is set on the listener (Linux copies it to
  accepted sockets) and on the upstream connector.
- **Boxed queue items.** `sync_channel` allocates its whole capacity up front;
  with 360-byte records, the 100k queue alone cost ~35 MiB. Boxing cut the peak
  RSS from 73 to 38 MiB.

## UI: a million rows in GPUI (`ui/`)

`ui/` is a separate crate, `flowlist`: a flow table on the GPUI snapshot that
packwing uses (gpui-pre 0.3.9), with a header, one `uniform_list` over every
row of `flows.db`, and a status line. Rows come from a cache of 64 pages of
256 rows; a miss is one rowid range query.

```sh
target/release/tracewing-dev seed --db target/ui-bench/flows-1m.db --rows 1000000
cd ui
cargo run --release -- ../target/ui-bench/flows-1m.db                 # look at it
ZED_MEASUREMENTS=1 cargo run --release -- ../target/ui-bench/flows-1m.db --autoscroll
FLOWS_DB=$PWD/../target/ui-bench/flows-1m.db \
    cargo test --release --features headless -- --nocapture          # headless + target/shots
```

`seed` writes varied rows (hosts, paths with ids and Vietnamese query words,
statuses, sizes) through the proxy's writer: 1M rows in 2.9 s, 215 MB. Both
benchmarks run the same script: 600 frames scrolling 40 px each from the top,
300 frames landing on random rows (a scrollbar drag), 120 frames ending on the
last row. Window 1440×900 at scale 2.

| Measure (2026-10-30) | Smooth | Jump | End |
|---|---|---|---|
| Headless CPU per frame (layout+prepaint+paint), p50 / p99 / max | 1.39 / 1.60 / 2.76 ms | 2.10 / 2.39 / 2.98 ms | 1.39 / 1.58 / 2.83 ms |
| Real Wayland window, 120 Hz: frame interval p50 / p99 | 8.33 / 8.74 ms | 8.33 / 8.60 ms | 8.33 / 8.59 ms |
| Real window: GPUI draw+present (`ZED_MEASUREMENTS`), all frames | p50 2.42, p99 4.56, max 11.76 ms | | |

- Opening the database (`count(*)` over 1M rows) takes 23 ms; the first frame 59 ms.
- 363 page reads over the run, 0.1–0.2 ms each (0.5 ms on a cold disk cache).
- The only frames over 16.7 ms are the first 5–6 after the window appears
  (17–59 ms), plus one of 25 ms at frame 39 in one of two runs; GPUI itself never
  took longer than 11.8 ms. Every frame after that met 120 Hz.
- The last rows line up (`target/shots/last.png`). Offsets are f32 pixels:
  1M rows × 24 px = 24M px, where f32 steps by 2 px, harmless here because row
  tops are even. Past ~2M rows the step becomes 4 px and more; a list that size
  would need its own mapping from scroll position to rows.

## System proxy and CA (`src/system/`, 2026-10-31)

```sh
tracewing-dev ca status | ca install [--scope user|machine] | ca uninstall
tracewing-dev proxy --system-proxy      # recovers, takes over, gives back
tracewing-dev sysproxy status | recover
```

Windows 11 and macOS 15 were tested in the VMs; Linux in throwaway containers
(the desktop's settings through `GSETTINGS_BACKEND=keyfile`, so no real session).
A flow in the database is the proof that a client went through the proxy: a 200
on its own proves nothing, because several clients go direct and still get one.

**Recovery.** Before anything changes, the old settings go in a journal. A
watchdog process waits on a pipe from the app, which closes however the app
ends. The OS runs a recovery at the next login in case the watchdog dies too.

| Case | Windows | macOS | GNOME | KDE |
|---|---|---|---|---|
| Clean exit | restored | restored | not run | not run |
| App killed: watchdog restores | yes | yes | yes | yes |
| PAC/bypass settings come back exactly | yes | yes | yes | yes |
| App and watchdog killed, then reboot and login | `RunOnce`: yes | LaunchAgent: yes | autostart written, login not run | same |
| User chose another proxy meanwhile: left alone | yes | yes, after a fix | yes | not run |

With the setting left on and nothing listening, the web really is gone
("Unable to connect"), which is why all of this exists.

macOS needed the fix because a setting lives per network service: "Built-in
Serial Port" still pointed at the proxy, so "still ours" was true and the user's
choice on Ethernet was overwritten. It now restores per service and per protocol.

**Which clients go through it** (each confirmed by a flow, or by its absence):

| OS | Through the system proxy | Direct |
|---|---|---|
| Windows | Edge, Firefox, .NET `Invoke-WebRequest`, MSXML/WinINet | curl (needs `--proxy` or `HTTPS_PROXY`), WinHTTP including `SETPROXY_PRECONFIG`, BITS as run over SSH |
| macOS | Safari, Firefox, URLSession in the desktop session, Python `urllib` | curl without flags; URLSession started from an SSH session |
| Linux | Chromium, Firefox | curl without flags |

Chromium on KDE needs `KDE_SESSION_VERSION`, which Plasma sets: without it, it
goes direct while the page still loads. Plasma 6 was not tried.

**Trust.** The check that counts is the OS's own verifier (`ca status`): it told
apart "in the keychain" from "trusted" when an install was left half done.

| OS | What the user sees | Notes |
|---|---|---|
| Windows, user | "Security Warning" dialog, **No** is the default; removal has its own | No rights needed. No gives error 1223, to handle as "cancelled". Firefox 159 trusts it on a new profile |
| Windows, machine | Nothing beyond UAC | Over SSH as admin it needs no dialog at all |
| macOS | Password dialog titled "security" | Over SSH it fails, even with `sudo`. A failed install leaves the certificate in the keychain, untrusted. Firefox 159 trusts it on a new profile |
| Linux, machine | polkit | Debian 12, Ubuntu 24.04, Fedora 42, openSUSE Tumbleweed, Arch: curl and Python accept it, reject it again after removal |
| Linux, user | Nothing, needs `certutil` | Chromium and Firefox refuse with only the system store, and need NSS (`~/.pki/nssdb`, each Firefox profile) |

**What this changes for the product:**

- **curl on Windows.** Schannel refuses a certificate whose revocation it cannot
  check (`CRYPT_E_NO_REVOCATION_CHECK`), so leaves now name a CRL the proxy serves
  at `/tracewing.crl`; curl works with no `--ssl-no-revoke`. The URL should carry
  a hash of the CA, because Windows caches CRLs by URL.
- **"Background Items Added" on macOS.** Writing the LaunchAgent raises this
  notification each time, so turning the proxy on twice shows it twice. Install
  it once, keep it, and have it do nothing without a journal.
- **macOS helper.** `networksetup` needs no password for an admin user and refuses
  a standard one ("requires admin privileges"). The helper planned in
  `docs/features.md` is only needed for standard users.
- **System services show up.** With the proxy on, Windows Update, widgets and
  Edge's background calls were most flows. Grouping and hiding hosts matters from
  the first minute.
- **Hosts.** HTTP/1.1 in a tunnel says `example.com:443`, HTTP/2 `example.com`;
  the default port is now dropped.
- **Sway and similar desktops** have nothing to set: offer the per-terminal and
  per-browser route instead.
- **Status with no response.** Requests cut off by the client have no status and
  no error, so the list needs a state for them.

**Not tested:** a standard (non-admin) user end to end on macOS, a real GNOME or
Plasma login, the elevated Windows path through UAC, GPUI on either VM.

Cross builds, from Linux: `cargo zigbuild --release --target x86_64-pc-windows-gnu`.
GPUI does not build this way (it wants `windres`).

## A 50 MB body in the window (`ui/src/body.rs`, 2026-10-31)

```sh
cd ui
target/release/bodyview gen target/bodies 50     # the worst cases
target/release/bodyview target/bodies/api.json [--autoscroll]
BODIES=$PWD/target/bodies cargo test --release --features headless --test body -- --nocapture
```

The body is mapped, not read. A background thread splits it into display lines
where a formatter would, with no tree, and keeps one mark per 256 lines. The first
version kept one `u64` per line: 447 MB for a 500 MB API answer and 2 GB for 500 MB
of single digits. Marks took those to 1.8 MB and 7.9 MB, at no visible cost per
frame. A test on random input checks that reading from a mark finds what the scan
found, which caught one real bug.

The view keeps its own position (a line and an offset) because `uniform_list`'s
`f32` offset has an 8 px step at 6 million lines; 262 million lines here still
line up. Lines are cut at 512 bytes, with a note saying how much is left.

Linux, headless CPU per frame (the same scroll script as the flow list):

| 50 MB body | Lines | Index | Scan | Smooth p99 | Jump p50 / p99 |
|---|---|---|---|---|---|
| Typical API answer, minified, Vietnamese | 5.9M | 288 KB | 47 ms | 0.50 ms | 0.61 / 0.67 ms |
| Single digits: most lines per byte | 26.2M | 928 KB | 116 ms | 0.46 ms | 0.49 / 0.53 ms |
| Every line wider than the window | 13K | 128 KB | 2 ms | 1.30 ms | 1.25 / 1.61 ms |
| Lines of 200 KB: marks far apart | 266 | 128 KB | 2 ms | 2.31 ms | 1.73 / 2.46 ms |
| One string, the whole file | 3 | 128 KB | 2 ms | 1.26 ms | 0.80 / 1.50 ms |
| A million levels of nesting | 7.6M | 352 KB | 43 ms | 0.41 ms | 0.51 / 0.57 ms |
| Already formatted | 4.3M | 256 KB | 40 ms | 0.53 ms | 0.63 / 0.69 ms |
| Base64 on one line | 12.8K | 128 KB | 3 ms | 1.72 ms | 6.02 / 6.67 ms |
| Binary, as a hex dump | 3.3M | 96 KB | none | 0.85 ms | 2.01 / 2.13 ms |
| CJK and emoji, none alike: font fallback | 235K | 128 KB | 3 ms | 3.62 ms | 15.67 / 16.64 ms |

- First lines appear 10–100 ms after opening, whatever the size. No frame during
  any scan took over 2 ms. 500 MB: first lines 45 ms, scan 0.47 s.
- Memory at 500 MB: 612 MiB resident, of which 39 MiB is the heap. The rest is
  the file in the page cache, which the OS takes back when it needs it.
- Real Wayland window at 120 Hz: the API answer held 8.33 ms (p99 8.83 ms; only
  the first frames after opening are longer). CJK held it while scrolling and fell
  to 52 fps while jumping on every frame, as when dragging the scrollbar.
- Binary was 89 ms a frame shown as text, which is why it is a hex dump.
- A cold page cache did not matter on NVMe, since the scan warms it. Slower disks
  were not tried; a page fault here happens on the window's thread.

Not built: horizontal scrolling, selection, search inside a body, expand and
collapse.

## Whole bodies on disk (`src/spool.rs`)

Bodies over `log_max_kib` go to files through one writer thread, as the
same buffers the client is sent. A byte budget bounds what waits; a body over it
stops being kept and is marked, so that a file with a hole is never shown as whole.

| 20 × 50 MB | req/s | p99 | Peak (`VmHWM`) |
|---|---|---|---|
| No spool | 30 | not recorded | 30 MiB |
| Spool (1 GB written) | 29 | 193 ms | 32 MiB, nothing lost |

The test compares the file with what the client got, byte for byte.

## Peak memory, measured properly

The benchmark sampled `VmRSS` and missed the peaks. Reading `VmHWM` found a real
bug: the flow queue was bounded by count, so 100,000 flows of 64 KiB took **6,051
MiB** while the disk caught up. It now has a byte budget as well (256 MiB). Past
that a flow keeps its row and its sizes, loses its body and says so.

| Peak | Before | With the budget |
|---|---|---|
| 100k requests of 64 KiB, 26k req/s (1.7 GB/s) | 6,051 MiB | 552 MiB, 58% of bodies let go |
| 1M requests of 1 KiB, 89k req/s | 41–341 MiB over 4 runs | 42–46 MiB over 3 runs, none let go |

The 320–341 MiB runs did not come back in the three runs after the fix, and what
caused them was not found. The budget is what bounds them now.

## Filtering a million flows (`src/search.rs`, `src/bin/filterbench.rs`)

`tracewing-dev seed --rich`: bodies of about 2 KB (2 GB in all) of varied words,
Vietnamese among them, each with a unique trace id; database 3 GB.

| Find | Found | First 200 | All ids |
|---|---|---|---|
| One host (index) | 3,315 | under 0.01 ms | 0.1 ms |
| Errors, `status >= 500` | 15,156 | 3.4 ms | 285 ms |
| POST + errors + one host | 58 | 107 ms | 107 ms |
| URL piece, `LIKE` | 100,265 | 0.7 ms | 342 ms |
| URL rare id, `LIKE`, nothing there | 0 | 335 ms | 335 ms |
| URL piece, trigram index | 100,265 | 0.18 ms | 7.2 ms |
| URL rare id, trigram index, nothing there | 0 | 4.6 ms | 4.6 ms |
| Body, `LIKE`, nothing there | 0 | 1,159 ms | 1,159 ms |
| Body, common word, word index | 902,819 | 0.25 ms | 96 ms |
| Body, rare phrase, word index | 1,016 | 0.07 ms | 0.1 ms |

| Index | Build, one thread | Size | Finds pieces of words |
|---|---|---|---|
| URL, trigram | 228,000 rows/s | 130 MB | yes |
| Body, words | 21,400 rows/s | 820 MB | no |
| Body, words without phrases | 31,000 rows/s | 275 MB | no, and no phrases |
| Body, trigram | 6,800 rows/s | 4,175 MB | yes |

**Choice:** trigram for URLs, words for bodies. Finding a piece of a word in a
body (part of a trace id) falls back to scanning in the background, a little over
a second at worst here. A body trigram index is larger than the bodies.

- **`đ`.** SQLite's `remove_diacritics` does not treat it as `d`, so "dat hang"
  found nothing. Text and queries now go through `search::normalize` first:
  902,819 found, the same as "đặt hàng". (Trigram has the same gap.)
- **The index has its own database.** SQLite allows one writer per file, and a
  body index is four times slower than a flood of traffic. A thread follows the
  flows table instead, and falls behind without traffic noticing:

| 1M requests of about 2 KB | req/s | p99 | Dropped | Peak | Index done |
|---|---|---|---|---|---|
| No index | about 82k | 1.15 ms | 0 | 54 MiB | |
| Index following | about 81k | 1.22 ms | 0 | 154 MiB | 44.5 s after the traffic |

- The follower's read snapshots keep the flows WAL from resetting under a flood:
  the database on disk was 3.9 GB instead of 3.0 GB at the end of that run.
- The search is not in a window yet. Queries over about 16 ms need a background
  thread and a way to cancel, and results will have to arrive as they are found.

## How a session is stored (`src/session.rs`, 2026-11-01)

A session is a directory: `flows.db` holds rows and where their bodies are,
`bodies/N.log` holds small bodies one after another, `files/N.body` holds large
ones, and `search/N.db` holds the index in pieces. Bodies stay as the wire sent
them; text that came plain is compressed with zstd when the writer is keeping up.
Text is unpacked only to feed the index, and into a temporary file for viewing.

Every limit is a setting with a default the app picks (`src/settings.rs`). A file
sets only what it wants to change, and a misspelt name is an error.

| Setting | Default | Why this default |
|---|---|---|
| `log_max_kib` | 64 | Past this, memory per body in flight costs more than a file does (below) |
| `body_max_mib` | 1024 | One download cannot push everything else out of a session |
| `compress_stored`, `compress_level` | on, 1 | 2.4:1 on 2 KB JSON, no cost to traffic |
| `index_urls`, `index_bodies` | on | URL index is cheap; body index costs about as much disk as the bodies |
| `index_kib_per_body` | 64 | Without a limit the index outgrew the bodies: 794 MB, against 425 MB at 64 KiB and 622 MB of gzip |
| `view_max_mib` | 4096 | The most a body unpacks to for viewing |
| `session_max_mib` | 2048 | Oldest flows go past this |
| `disk_reserve_mib` | 2048 | Bodies stop being kept below this; rows go on |
| `queue_budget_mib`, `spool_budget_mib` | 256 | Bounds peak memory when the disk falls behind |

### Choosing the layout

The same mix of bodies (mean 11 KB), written up to 50 GB. Cold reads drop the page
cache first.

| Layout | Write MB/s | Page of the list, cold p50, at 10 → 50 GB | Drop the oldest tenth | Disk after that |
|---|---|---|---|---|
| Bodies in the rows | 159–281 | 1.51 → 2.65 ms | 6.4 s | unchanged; `VACUUM` 34 min, reported 108 GB |
| Bodies in a second database | 147–201 | 0.47 → 0.56 ms | 7.5 s | unchanged; `VACUUM` 36 min, reported 108 GB |
| Bodies in a log, 256 MiB segments | 250–324 | 0.38 → 0.49 ms | 4.9 s | 50.7 → 46.0 GB at once |

SQLite does reuse freed pages, so a database holds its size without a `VACUUM`, but
it never gets smaller. Log reads were less even than the others in this run: body
cold p50 ranged from 0.28 to 7.22 ms, p99 at most 15 ms. Writeback from the run
itself is the likely cause, though that was not checked.

A body by size, in each place. Linux is NVMe with ext4; Windows is the VM with NTFS
and Defender on, where "cold" is not really cold, since its cache was not dropped.

| Body | Write MB/s, Linux: SQLite / file each / log | Write MB/s, Windows: SQLite / file each / log |
|---|---|---|
| 1 KB | 573 / 155 / 2,563 | 122 / 4 / 315 |
| 16 KB | 969 / 1,870 / 5,307 | 214 / 51 / 1,454 |
| 64 KB | 1,037 / 4,046 / 6,488 | 226 / 212 / 2,590 |
| 256 KB | 1,081 / 6,193 / 6,481 | 238 / 732 / 3,575 |
| 4 MB | 664 / 7,425 / 6,969 | 247 / 2,950 / 2,969 |

A file for each small body would be the worst choice on Windows, by 80 times at
1 KB. At 64 KB a file is cheap enough on both, and a body larger than that should
not wait in memory until it has all arrived.

### The index in pieces

Taking the oldest tenth out of a million-row index, three times over:

| | Time | Disk | Common word |
|---|---|---|---|
| One database, `contentless_delete` | 8–32 s, up to 14.6 s for 5,000 rows | 829 → 1,393 MB | 102 → 282 ms |
| Ten databases, oldest deleted | 6 ms | 850 MB, steady | 105 → 102 ms |

### What it costs traffic

Through the proxy, HTTP/1.1, 64 connections, on NVMe. Index catch-up is how long
the index took after the traffic ended. Bodies come from a pool of 4,096 different
texts, so the index sizes here are a guide, not a promise.

| Body | Settings | req/s | p99 | Peak | Index catch-up | Disk: log + index |
|---|---|---|---|---|---|---|
| 2 KB plain | nothing extra | 104,409 | 0.87 ms | 39 MiB | | 603 MB + 0 |
| 2 KB plain | compressed | 100,327 | 0.94 ms | 170 MiB | | 248 MB + 0 |
| 2 KB plain | + URL index | 92,631 | 1.61 ms | 129 MiB | 0.05 s | 248 + 42 MB |
| 2 KB plain | defaults | 99,234 | 0.98 ms | 150 MiB | 15 s | 248 + 287 MB |
| 2 KB gzip | defaults | 98,707 | 1.01 ms | 96 MiB | 16 s | 233 + 282 MB |
| 2 KB Brotli | defaults | 100,600 | 0.92 ms | 46 MiB | 13 s | 228 + 281 MB |
| 2 KB zstd | defaults | 101,460 | 0.91 ms | 54 MiB | 15 s | 248 + 282 MB |
| 30 KB plain | nothing extra | 54,590 | 1.73 ms | 75 MiB | | limit reached |
| 30 KB gzip | defaults | 89,217 | 1.02 ms | 69 MiB | 51 s | 739 + 1,031 MB |
| 68 KB gzip, 300 KB of text | defaults | 25,942 | 4.02 ms | 54 MiB | 20 s | 1,365 + 422 MB |

Compression on the writer was the one real cost: at 1.6 GB/s of 30 KB bodies, 68%
of flows lost their bodies, none without it. The writer now skips it when more than
a quarter of the queue budget is waiting: nothing shed, 50,000 req/s, p99 2.1–2.4 ms.
The index for uniform 30 KB bodies came to 1.4 times their gzip size, higher than
the 0.68 of the mixed sizes above, and it counts towards the session's limit.

### A disk that runs out

A 300 MB tmpfs, 1.8 GB of traffic, 60,000 requests. In all of them no request
failed, latency did not move (p99 1.7–2.1 ms) and the proxy exited cleanly.

| Settings | Rows kept | Bodies | Disk at the end |
|---|---|---|---|
| `disk_reserve_mib = 50` | all 60,001 | 51,523 flows say theirs was not kept | 45 MB free |
| `disk_reserve_mib = 0` | 10,601 | fill the disk | 0 free |
| The same, with the body index on | 9,886 | fill the disk | 0 free |
| `session_max_mib = 128` | newest 1,761 | all of those kept | 207 MB free; peak 180 MB against a 134 MB limit |

The reserve is what keeps rows flowing, so it should not be set to 0 lightly. The
first version looked at the disk every 500 ms and filled it anyway at this rate; it
now looks after every 4 MB written. At the 2 GiB default the peak was 3% over.

### A disk that cannot keep up

`--device-write-bps` in a container with 4 GB of memory (`dd` measured 21 MB/s
against a limit of 20, direct and with `fsync`). 100,000 requests of 30 KB of text.

| Disk | Plain: req/s, p99, peak, bodies not kept | Gzip: req/s, p99, peak, bodies not kept | Longest stop |
|---|---|---|---|
| NVMe | 50,406, 2.05 ms, 196 MiB, 0 | 89,093, 1.02 ms, 74 MiB, 0 | 52 s |
| 100 MB/s | 52,482, 2.02 ms, 429 MiB, 65% | 88,382, 1.09 ms, 53 MiB, 0 | 95 s |
| 20 MB/s | 56,223, 1.70 ms, 424 MiB, 56% | 92,196, 0.99 ms, 366 MiB, 16% | 235 s |

Traffic does not feel the disk at all. What gives is bodies, in a burst far faster
than the disk, and the index, which took up to 217 s to catch up. Stopping waits for
it here so as to count it; the app must not.

## Known gaps

- hudsucker always offers `h2, http/1.1` to the client instead of mirroring
  what the origin chose, and builds leaves from the SNI/`CONNECT` host only.
- CONNECT tunnels run in detached tasks, so on shutdown the writer is closed
  explicitly and flows from still-open tunnels are counted as dropped.
- WebSocket messages are not recorded (only the upgrade request).
- A session cannot be reopened and carried on with; each run makes a new one. An
  app that quits during catch-up must finish the index next time it opens it.
- Text past `index_kib_per_body` is not found. The app needs a slower scan for it.
- The index can read a body from a file before the file has reached the disk,
  when the file writer is behind, and then holds only what had arrived.
- A power cut can leave rows pointing past the end of the log, which read as gone.
  A body in the log has no checksum, so one overwritten by the file system would
  not be noticed.
- `bodyview` opens a file; `body` writes one, but the two are not joined in a window.
- Not tested yet: mobile devices, the GPUI list on
  Windows and macOS (it needs a real GPU: neither VM
  has one, and the macOS VM has no Metal at all).
