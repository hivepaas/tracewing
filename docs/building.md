# Building tracewing

tracewing is a [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) app, built with the Rust version in `rust-toolchain.toml` (rustup installs it by itself). On Linux, the window also needs the packages listed in `.github/workflows/ci.yml`. It builds for Wayland by default: add `--features x11` for X11.

```sh
make release    # the window; its proxy listens on 127.0.0.1:9090
make debug      # the same, built for debugging
```

Try it from a terminal, with the certificate authority tracewing made for this computer:

```sh
curl --proxy http://127.0.0.1:9090 \
     --cacert ~/.local/share/tracewing/ca.pem https://example.com/
```

## Tests

```sh
make test       # includes end-to-end tests through the proxy
cargo test --release -p tracewing --features headless --test screen   # the window with no display; pictures in target/shots
```

## Something to try it on

```sh
make playground          # a server: http://localhost:10000 lists its paths, and https://localhost:10443
make playground-trust    # the same, its authority trusted by tracewing first (restart tracewing)
```

Safari, and every app on macOS's own networking, sends `localhost` and
`127.0.0.1` straight to the server whatever the proxy settings say. From Safari,
with the system proxy on, open `http://playground.localhost:10000` or
`https://playground.localhost:10443`: a name under `.localhost` goes through the
proxy and still comes back to this computer.

## The network extension (macOS)

`crates/netext` is a transparent proxy that macOS runs in a system extension
of its own: it takes the flows of the apps it is told of and hands each to
tracewing's listener as a `CONNECT`, with the app's signing id and process id
in an `X-Tracewing-App` header, so that those apps come through with no proxy
setting. Every other flow, and every flow while nothing listens on tracewing's
port, goes out as it would have.

```sh
cargo build --release -p tracewing-netext
cargo test --release -p tracewing-netext   # what it decides, on every system
```

To run, it has to be in tracewing.app, signed with the Developer ID, with the
two provisioning profiles and notarized ([releasing.md](releasing.md#signing-for-macos)),
and the app in `/Applications`. macOS asks once to allow the extension
(System Settings › General › Login Items & Extensions › Network Extensions)
and once to allow its proxy configuration. Then:

```sh
systemextensionsctl list                   # installed, and allowed
/usr/bin/log stream --level debug --predicate 'subsystem == "com.hivepaas.tracewing.proxy"'
```

(`/usr/bin/log`, as `log` is something else in zsh.) The app installs it and
turns it on in a later change; until then it is packaged and does nothing.

## Packages

A build of your own is a binary; a release is a package. On Linux, with
`cargo-about` and `dpkg-dev rpm cpio desktop-file-utils` installed:

```sh
make package                                        # dist/: a .deb and an .rpm
scripts/check_linux_package.sh dist/tracewing_*.deb # what it holds, and that it runs
```

How a release is made and what it ships: [releasing.md](releasing.md).

## More

```sh
cargo run --release -p tracewing-dev -- settings   # every storage setting and its default
```

What the core has been measured to do, and why it is built this way: [measurements.md](measurements.md).
