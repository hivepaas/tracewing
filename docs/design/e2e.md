# End-to-end tests on real systems

tracewing is tested a great deal. As of `ab34bb0` there are about 965 unit
tests, 41 integration and window test files in `crates/app/tests`, and an
in-process suite in `crates/dev/tests/e2e.rs` that sends a client through the
proxy to a TLS origin. That suite covers almost every protocol, rule and
inspector part.

All of it runs on Linux, and almost none of it touches the computer it runs
on. The system proxy is a fake `gsettings` script. The CA is installed into a
fake `certutil`. The keychain is `MemoryKeychain` or `KeyStore::File`. The
browsers are shell scripts that write down their arguments. `adb` and `xcrun`
are fakes too. This is deliberate: a test that changed the developer's own
proxy or trust store would be dangerous to run. It does mean that
the parts of tracewing which exist *to change the computer* have never been
seen doing it.

This page records what is covered, and what is not. It then describes the
suite that closes the gap: tests that change a real system on purpose, and
run only where that is safe.

## What CI runs

- `ci.yml` runs `cargo test --workspace --release` on `linux-x86_64` on every
  push and pull request, plus the headless window tests on Ubuntu. macOS and
  Windows run only when the workflow is started by hand, which has not
  happened. So every `#[cfg(windows)]` and `#[cfg(target_os = "macos")]`
  test, including those in `crates/system/src/windows.rs` and `macos.rs`, is
  compiled and run nowhere.
- `release.yml` runs on pull requests that touch packaging, and on tags. It
  builds and installs the packages on five runners, then runs:
  - Linux: `--version`, a one-second `capture` with no traffic, `sessions`
    and `recover`.
  - macOS: `codesign --verify` and a one-second `capture`.
  - Windows: `--version`, with installs and uninstalls both ways.

  It has never run on a tag.
- `#[ignore]`d live tests are never run in CI: `local_live.rs`,
  `local_live_linux.rs`, `local_live_http3.rs` and `wintun_live.rs`. Neither is
  `crates/system/tests/android_device.rs`, which needs
  `TRACEWING_ANDROID_SERIAL`.

## What nothing reaches

The rows of [features.md](../features.md) whose real behaviour no test sees,
ordered by how much a release depends on them. Each row was checked against
the code: "fake" names the stand-in the test uses.

| Feature | Covered today by | Never exercised for real |
|---|---|---|
| System proxy on and off | `connect.rs::the_system_proxy_and_the_settings_from_the_window`, with a fake `gsettings` | Real dconf/GNOME. KDE has no test. WinINet on Windows and macOS `networksetup` have no test at all. |
| System proxy put back after a kill: watchdog, systemd timer, Windows scheduled task, launchd agent, `tracewing recover` | `cli.rs::the_watchdog_puts_the_setting_back_when_the_app_goes`, `recover_says_what_it_did_and_leaves_a_stranger_alone`; unit tests of the unit-file, task XML and plist text | On a runner with no desktop these take the "cannot read the setting" branch and still pass. No test arms or fires the timer, task or agent. |
| CA installed and trusted, checked, removed | `connect.rs` (user scope, fake `certutil`); `cli.rs::uninstall_undoes_what_it_did_and_keeps_what_was_recorded` | The Linux system store (pkexec, `update-ca-certificates`), the Windows ROOT store and the macOS keychain trust are never written. `verify_ca` has no test. The uninstall test accepts "no longer trusts it" or "never did", and since its CA was never trusted, `ca_remove` is never reached. |
| The CA's key and other secrets in the OS keychain | `ca_key.rs::a_new_key_goes_to_the_keychain_and_no_file`, with `MemoryKeychain` | `OsKeychain` (Secret Service, DPAPI/Credential Manager, Keychain) is called by no test: CA key, relay passwords, WireGuard keys, company CA. |
| A real browser through tracewing | `launch.rs::the_browser_gets_its_own_profile_and_the_proxy`, with fake `google-chrome`/`firefox` scripts | No Chrome or Firefox ever loads a page through the proxy, trusts the CA, reaches the Set up test page, or stays on HTTP/2 because Alt-Svc was taken out. S5 now does all of this on Linux and Windows. |
| Blocking UDP 443 | `quic_linux.rs::failures_say_why` (text only) | Nothing calls `block_quic`: real `nft`, `pf` and the Windows firewall never run. `quic_windows.rs` has no tests. |
| The installed package records HTTPS | `check_linux_package.sh --install`, `check_macos_app.sh`, `check_windows_installer.ps1` | The packages capture one second with no traffic. The Windows check reaches `--version` only, and the installer's `tracewing uninstall` hook is run but its effect is never checked. S4 now covers the .deb, the .rpm, `setup.exe` both ways and the `.zip`. |
| Android over ADB: Set up, Undo, put back at close and at the next start | `devices.rs` with a fake `adb`; `android_device.rs` (env-gated) | `undo_android` and `android::recover` have no test. Real root and the Conscrypt APEX run only in the env-gated test. |
| iOS Simulator | `devices.rs` with a fake `xcrun` | Never run on a Mac. Turning the proxy on is macOS-only and untested. |
| Capture without proxy settings: TUN on Linux, Wintun on Windows, HTTP/3 through it | `stack.rs`, `tun.rs` with a fake device; `local_live*.rs` `#[ignore]` | The adapter join runs only by hand, under `unshare`, in an editor's shell (AppArmor refuses it elsewhere). Wintun and the Windows catcher have never run. |
| Platform UI: Cmd keys, the native save dialog, the clipboard, more than one window and display, DPI, window restore | window tests on Linux only | Everything outside Linux headless. |
| Performance claims: a million flows at display rate, bodies of 50 MB and more | `screen.rs::a_million_flows_while_traffic_keeps_coming` (env-gated, prints timings, asserts none) | No frame-time or responsiveness assertion. `big_body.rs` now opens 60 MB of JSON in the window (5.3 million lines): shown, scanned, laid out pretty, found in, and shown at its last line, with loose time bounds. It found the line numbers' first digit cut off past a million lines. |

Smaller gaps that need no real system, only a test:

- ~~HAR export of WebSocket messages, with and without secrets.~~ Done:
  `har.rs`'s `a_websockets_messages_go_out_both_ways_and_their_secrets_can_be_left_out`.
- ~~Solana and Bitcoin answers in a window.~~ Done: `rpc.rs`'s
  `solana_and_bitcoin_calls_are_read_in_the_inspector`, shots 96 and 97.
- ~~Block answers in `rpc_view.rs` (`block_lines`).~~ Done:
  `a_block_reads_as_which_one_and_how_full`, on a real mainnet block.
- ~~Find: Shift+Enter and the arrow keys, a fold opened on a hit, and the
  10,000 cap.~~ Done in `find.rs`, by keys: Shift+Enter, Shift+F3 and the
  wrap at both ends. The bar's arrow buttons call the same step and are
  not clicked.
- ~~Dragging the dividers and dropping the inspector handle. Hiding the
  inspector.~~ Done: `dividers.rs`, by mouse and shortcut, read back from
  the layout file.
- ~~Activating the items of a context menu. Copy as cURL from the window.~~
  Done in `inspect.rs`: the copy menu's first item pressed, and Copy as
  cURL by its shortcut.
- ~~The Settings part last open being kept.~~ Saved on a change
  (`text_size.rs`) and read back by `layout.rs`'s round trip, which now
  holds the part and the way in.

Rows in features.md with no code behind them, so nothing to test yet:

- The macOS privileged helper for standard accounts.
- Kerberos/Negotiate.
- Docker and Kubernetes helpers.
- The headless agent with a remote window.
- Raw TCP and UDP.
- Rule sets shared by a team.
- Signed updates.
- Licences, and so any Free/Pro limit, such as "3 rules on at once".

## The suite

**Where it lives.** A crate of its own, `crates/e2e`, whose tests change the
real system on purpose. Each test calls the same functions the window calls:

- `tracewing_system::take_over`, `ca_install`, `block_quic` and the keychain.
- The installed `tracewing` program, for everything the command line does.
- A real browser.

**Where it runs.** Nowhere by accident. Every test starts by asking for both
of these, and does nothing without them:

- `TRACEWING_E2E=1` in the environment;
- a marker file that says the machine may be changed:
  `/etc/tracewing-e2e-throwaway` on Linux, `C:\tracewing-e2e-throwaway` on
  Windows, `/Library/Application Support/tracewing-e2e-throwaway` on a Mac.

  A developer's own machine has no marker. A container, a VM or a CI runner
  is given one by the script that starts it.

`cargo test --workspace` still runs this crate, and its tests return at once.

**Runners.**

| System | Where, while developing | Where, in CI |
|---|---|---|
| Linux | Two containers, from `crates/e2e/linux`. `Dockerfile` is Ubuntu 26.04, the release the binaries are built on, with dbus, dconf and GNOME's schemas, KDE's kconfig tools, `libnss3-tools`, `nftables`, polkitd, gnome-keyring, Chrome's .deb and Firefox from Mozilla's repository. `Dockerfile.fedora` is Fedora 44 with the same machinery and no browsers. Ubuntu installs the .deb, Fedora the .rpm. Started by `scripts/e2e_linux.sh`, which builds the packages first, with `NET_ADMIN` for `nft`. | The same, on every pull request: `ci.yml`'s `linux-e2e` job |
| Windows | The `windows-11` VM, by `scripts/e2e_windows.ps1 -ThisMachineMayBeChanged -ConfirmTrustDialogs -Packages -Browsers 'chrome edge firefox'`, in its desktop session (a scheduled task with `/IT`), not over ssh: see below | `windows-latest`, by the same script, on a tag |
| macOS | The user's own Mac, by `scripts/e2e_macos.sh --this-machine-may-be-changed --attended`: somebody there answers each password dialog, and every scenario that has one on macOS runs | `macos-15`, by the same script without `--attended`, on a tag or by hand: what macOS asks nobody about (S2, S3, S6, and S7 as root) |

## Scenarios

Each scenario is one test. Each one puts back what it changed, even when it
fails.

| # | What | Linux | Windows | macOS |
|---|---|---|---|---|
| S1 | **Trust.** Install the CA for the user and for the system. `verify_ca` says trusted. A client that reads only the system's roots reaches an HTTPS origin through the proxy. Then `tracewing uninstall`: no store holds it, and `verify_ca` says so. | NSS + `/usr/local/share/ca-certificates` | CurrentUser and LocalMachine ROOT | keychain trust |
| S2 | **System proxy.** Turn it on: the real setting, read with the desktop's own tools, names the proxy. Turn it off: the setting is exactly what it was, including a proxy of the user's own that was there before. (A program that follows the setting is S5's.) | GNOME, KDE | WinINet (per-connection options) | `networksetup` |
| S3 | **Put back after a kill.** Turn it on, then `kill -9` tracewing. The watchdog restores the setting. Then kill the watchdog too: the timer, task or agent restores it within its period. `tracewing recover` restores it at once. | systemd user timer | scheduled task | launchd agent |
| S4 | **The installed package.** Install the package built for the release. Record HTTPS with `tracewing capture` from it: the HAR holds the decrypted body. Uninstall, and nothing is left behind (S1's stores, S2's setting, the timer, task or agent, PATH). | `.deb`, `.rpm` | `setup.exe` both ways, `.zip` | `.dmg` |
| S5 | **Browsers.** Headless Chrome and Firefox, through the system proxy and through Set up's own profile. HTTP/1.1, HTTP/2, a WebSocket and SSE are all recorded. The Set up test page says it was reached. With Alt-Svc taken out, nothing tries HTTP/3. | Chrome, Firefox | Edge, Chrome, Firefox | Safari by hand; Chrome, Firefox |
| S6 | **The keychain.** The CA's key, a relay password and the WireGuard key go into the real keychain and come back. | Secret Service | Credential Manager | Keychain |
| S7 | **Blocking UDP 443.** While it is on, a datagram to port 443 is refused. Released, or its holder killed outright, it gets through again. (The refused attempt as a row naming its program belongs to S8, which has the adapter that sees it.) | `nft` | Windows Firewall | `pf` |
| S8 | **Capture without proxy settings.** The live tests run here unignored, with HTTP/3 from Chrome. | TUN in a namespace | Wintun | network extension, by hand (a signed build) |
| S9 | **Android.** Run against the `tracewing-test` emulator, not in the container: Set up every device, Undo, then kill tracewing and start it again so it puts the device back. | host + AVD | — | — |
| S10 | **The window.** It opens, draws Set up, takes a key, opens the save dialog and the clipboard, and closes. The system proxy is back afterwards. | headless, as today | the VM (its adapter is accepted) | the user's Mac |
| S11 | **The title bar's caption buttons (Windows only).** Sweeping the bar, the window answers `HTMINBUTTON`, `HTMAXBUTTON` and `HTCLOSE` in that order, each a band about `CAPTION_W` wide, with an `HTCAPTION` band a button wide beside them; then a real click on minimise leaves `IsIconic` true. `scripts/check_windows_titlebar.ps1`, run from `e2e_windows.ps1`. | — | the VM's desktop session | — |

S10 on Windows needs a display adapter that GPUI's DirectX renderer accepts.
The VM's does: tracewing's window opened there, maximized, at 96 dpi, while
#381 was checked. So everything in the table runs in the VM.

S10 on Windows is `scripts/check_windows_window.ps1`, which `e2e_windows.ps1`
runs after S11. It works as a hand does, in the desktop session:
- It opens the window on the window's own data folder, set aside first and
  put back after.
- It checks that the window draws (more than a few colours, taken with
  `PrintWindow`).
- It presses Ctrl+2 and Ctrl+B, and the hidden sidebar is written to
  `layout.toml`.
- It sends a request through the window's proxy (its port from `tracewing
  env`), chooses the row with Home and copies it with Ctrl+Shift+C. The
  clipboard, read as another program reads it, holds the `curl` command.
- Ctrl+S opens Windows' own save dialog, which is closed again.
- The window is closed, its process ends, and the proxy setting is what it
  was.

It exits 2, not 0, where there is no input desktop. Its first run, on the
hosted runner, passed.

S11 is Windows alone because the thing it checks exists only there: the three
buttons are painted by tracewing and are a *hit test* to Windows, not windows
of their own, so no test on Linux or the Mac can press them and `windows` is
`cfg!(target_os = "windows")` in any case. It needs the **interactive desktop
session**, because the pointer has to move and GPUI answers for where the
pointer really is; an ssh logon has no input desktop, and there the check says
so and exits **2**, which is not a pass. It is measured both ways: it passes a
build with #381's fix and fails the same build with the bar's drag area put
back.

## Order of work

1. `crates/e2e` with its guard, the Linux container and its script, and S1
   and S2 on Linux. **Done**: both pass under GNOME and KDE, as an ordinary
   user who reaches root through `pkexec`.
2. S3, S6 and S7 on Linux. **Done**, under GNOME and KDE:
   - S3: the app is killed outright. The real watchdog puts the setting back.
     `tracewing recover --only-if-gone`, which is what the timer runs, leaves
     a running app alone, then puts the setting back once the app has gone.
   - S6: the CA's key, the WireGuard keys and a relay's password go into the
     Secret Service, where `secret-tool` sees them. `tracewing capture`, in
     another process, finds the key there.
   - S7: `nft` really refuses UDP 443 to an echo server on loopback. It is
     free again on release, and when its holder is killed.

   The systemd timer firing by itself is checked too, on the CI runner
   rather than in the container, which runs no systemd: `linux-e2e`'s
   "systemd timer" step keeps the runner user's manager up with linger and
   GNOME's setting in dconf. With the watchdog and then the app killed, the
   timer puts the setting back within its minute and then disarms itself.
3. S1–S3, S6 and S7 on Windows in the VM. **Done**: all pass. The
   workspace compiles there clean with `-D warnings`, the first Windows build
   since 16 Nov. Notes:
   - **A bug, found and fixed.** A CA put in both this user's root store and
     the machine's was said to be in the machine's alone, because the
     user's system store `ROOT` is a view that also holds the machine's
     roots. Remove in Settings and `tracewing uninstall` then took it out of
     the machine's store only, and the user's store went on trusting it.
     Each store is now asked alone: the user's is its physical store
     `ROOT\.Default`.
   - **The scheduled task fires.** With the watchdog killed first and then
     the app, the task the app armed puts the setting back within its
     minute.
   - **Not over ssh.** An ssh session is a network logon. Windows saves
     nothing in the Credential Manager from it ("Credentials cannot be
     saved from this logon session") and refuses to add to this user's root
     store ("The request is not supported"). A user runs tracewing in a
     desktop session, so the tests do too.
   - **The trust dialogs.** Putting a root in this user's store asks
     "Security Warning", and taking one out asks "Root Certificate Store".
     `-ConfirmTrustDialogs` answers Yes, and only to a dialog that names
     tracewing's CA. These are plain Win32 dialogs: UI Automation sees their
     buttons as panes with nothing to invoke, so the button with the id
     IDYES is pressed with `BM_CLICK`.
   - **What the app arms runs as itself.** The watchdog, the scheduled task,
     the systemd timer and the elevated helper of the Windows block are all
     started as `current_exe()`. So the app in S2, S3 and S7 is
     `tracewing e2e-hold`, a hidden command that holds what the window
     holds, and not the test program.
   - **Another data directory's CA.** This user's physical store
     `ROOT\.Default` lists this user's roots alone, none of the machine's.
     The CAs of two data directories on one computer have the same subject,
     `O=tracewing, CN=tracewing CA (HOST)`. So `tracewing uninstall` takes
     this CA out by its bytes and leaves any other. It says another is
     still trusted, found by that subject, and does not remove it, as on
     Linux.
   - **The machine is left as it was found.** Each test that sets a proxy of
     the user's own puts the original back when it ends, however it ends.
4. S4 and S5 on Linux. **Done**, under GNOME and KDE:
   - S4: the .deb in Ubuntu and the .rpm in Fedora, installed from the file
     through pkexec. `/usr/bin/tracewing` from the package records HTTPS: the
     HAR it exports holds the body sent and the body answered. It takes the
     system proxy and arms the login autostart, which runs
     `/usr/bin/tracewing`. Killed, it leaves the setting for `tracewing
     uninstall`, which puts it back, takes the CA out of every store and
     disarms the autostart, and keeps what was recorded. The package
     removed, none of its files is left, the package database does not
     know it, and nothing called `tracewing` is on the PATH. Fedora also
     runs S1–S3, S6 and S7, so S1 covers `update-ca-trust` there.
   - S5: Chrome and Firefox, headless, found as Set up finds them. Each
     opens a page over HTTP/1.1 whose script fetches JSON over HTTP/2,
     echoes a WebSocket message, reads three server-sent events and reaches
     `https://tracewing.test/`. Each does this twice: once in Set up's own
     profile, started with the very command Set up starts
     (`launch::browser_command`), and once in a plain profile through the
     system proxy. The HAR holds every one of these, decrypted, with the
     WebSocket's messages both ways. The server offers HTTP/3 in `Alt-Svc`
     and listens on UDP: the browser is never offered it, and no datagram
     arrives. Chrome shows the test page saying it works and that the CA is
     trusted. The server is on the container's own address, not loopback,
     which a browser sends past any proxy.
   - **A gap, found and closed.** Firefox on KDE did not follow the system
     proxy. It reads GNOME's settings alone, and tracewing wrote KDE's
     `kioslaverc` alone, so Firefox went straight to the server. On KDE,
     tracewing now sets GNOME's settings too where they can be read, and
     writes them down to put back. S2 checks both on KDE.
   - Chrome on KDE reads `kioslaverc` only with `KDE_SESSION_VERSION` set,
     as a Plasma session sets it. Without it, Chrome looks for KDE 4's
     settings, finds none and goes straight. The container sets it.

5. S4 and S5 on Windows, in the VM's desktop session. **Done**, all
   passing the first time they ran:
   - S4: `setup.exe /S`, both `/CURRENTUSER` and `/ALLUSERS`. The installed
     `tracewing.exe` records HTTPS into the window's data folder, which the
     test sets aside first and puts back after. It takes the system proxy
     and arms the scheduled task, which runs the installed program, and is
     killed. Then `uninstall.exe /S`, which runs `tracewing uninstall`
     itself, puts the setting back, takes the CA out of both stores and
     removes the task. The folder, the uninstall key, the Start menu
     shortcut and the PATH entry are gone, and what was recorded stays. The
     `.zip` runs from a folder, records HTTPS, and leaves the PATH alone.
   - S5: Chrome, Edge and Firefox, each in Set up's own profile and through
     WinINet's setting. Firefox reads the system's roots on Windows, so its
     user profile needs nothing of tracewing's.
   - `scripts/e2e_windows.ps1 -Packages` builds the installer and the zip
     with `scripts/package_windows.sh`, in Git Bash, with NSIS, cargo-about
     and 7-Zip. `-Browsers 'chrome edge firefox'` names what is installed.
6. S8 and S9.
   - S8 on Linux, **done**. The live tests (`local_live_linux.rs`,
     `local_live_http3.rs`, and `tun_linux.rs`'s real device) run on every
     pull request in `linux-e2e`, still `#[ignore]`d for `cargo test`. Each
     makes its adapter in a network namespace of its own, under
     `unshare --user --map-root-user`. The runner's AppArmor refuses that
     until `kernel.apparmor_restrict_unprivileged_userns` is 0, which the
     job sets on a runner that is thrown away after. They pass from an
     editor's shell here too: VS Code's AppArmor profile allows user
     namespaces.
   - S8 on Windows, **done** in the VM, over ssh, as an administrator:
     `wintun_live.rs`'s seven and `local_live.rs`'s one pass. Afterwards no
     Wintun adapter and no `/1` route are left, and the default route is
     the VM's own. `scripts/e2e_windows.ps1 -Wintun <wintun.dll>` runs them
     one test at a time, as they need.
   - S9, **done** against the `tracewing-test` emulator (API 36), by
     `crates/app/tests/android_live.rs`, which runs only with
     `TRACEWING_ANDROID_SERIAL` set, so not in CI. The device first has a
     proxy of its own. Set up every device points it at tracewing, with
     that proxy written down. Undo puts that proxy back. A device left set
     up by a tracewing that was killed is put back when tracewing starts
     again on that data folder. With `recover_android` doing nothing, it
     fails waiting for that.
   - HTTP/3 from Chrome through the adapter, **done**:
     `local_live_http3.rs`'s `chromes_http3_goes_through_tracewing_on_a_real_adapter`.
     Chrome, headless in the test's namespace, is told to speak QUIC to
     `h3.test`, which its resolver maps to the address the default route
     sends into the adapter. It trusts tracewing's CA from an NSS database
     in a home of the test's own. The page it gets is the server's own body,
     the server saw `/orders`, and the way in counted it. With another
     program kept instead of Chrome, it fails: Chrome gets nothing.
7. Run `release.yml` on a tag, with the suite as a job before the draft.
   **Done.** `release.yml`'s `windows-e2e` job runs `e2e_windows.ps1
   -ConfirmTrustDialogs -Packages -Browsers 'chrome edge firefox'` on
   `windows-latest`, on a tag and when the workflow is run by hand, and the
   draft waits for it. Not on the pull requests that rehearse the packaging:
   they would wait for it for nothing they change. Its first run, by hand,
   passed every test, the trust dialogs answered: a hosted runner's logon is
   a desktop one, which the Credential Manager and the user's root store
   need.
8. The smaller in-process gaps above, one pull request each.
9. macOS. `crates/e2e` builds there, and `scripts/e2e_macos.sh` runs it.
   - `desktop_macos.rs` reads and writes the setting with `networksetup`, for
     every service that is on, so S2 and S3 run.
   - S3 on macOS is the launchd agent putting the setting back the moment
     the app has gone. The minute-timer test is Linux's and Windows'.
   - S6 uses the login keychain. On a Mac it takes out only what it put
     there, found against what was there before: a Mac may be somebody's
     own, with tracewing's real key in it.
   - S1 and S7 make macOS ask: for a user's or an administrator's password
     for the trust settings, and for an administrator's for `pf`. They run
     only with `TRACEWING_E2E_ATTENDED=1` (`--attended`) or as root, which
     macOS does not ask; on a runner the script runs S7 with `sudo`.
   - Not yet on macOS: S4 (the `.dmg`, which `check_macos_app.sh` opens and
     runs), S5 (the browsers trust the CA through the keychain, which asks),
     S8 (the network extension, which wants a signed build) and S10.
