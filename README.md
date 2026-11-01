# tracewing

**A native HTTP(S) debugging proxy for Windows, Linux and macOS that keeps up with big sessions.**

Built in Rust with [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui).

> **Status:** early development. There is nothing to download yet. The core works from the command line (see [Build](#-build)); the window is next.

## 🎯 Plans

- **See every request:** HTTP/1.1, HTTP/2, WebSocket and server-sent events, over HTTPS through a certificate authority made for your machine alone.
- **Big sessions stay smooth:** a million requests scroll at the display's refresh rate, and large bodies stream through instead of piling up in memory.
- **Change traffic on the way:** breakpoints, map to a local file or another host, rewrite rules.
- **Phones and emulators too:** set up from a QR code, with help for Android's certificate rules.

## 🔨 Build

Needs Rust; `rust-toolchain.toml` picks the version.

```sh
cargo test --release                        # includes end-to-end tests through the proxy
cargo run --release -p tracewing-dev -- proxy          # listens on 127.0.0.1:9090
curl --proxy http://127.0.0.1:9090 \
     --cacert ~/.local/share/tracewing/ca.pem https://example.com/
cargo run --release -p tracewing-dev -- settings       # every storage setting and its default
```

What the core has been measured to do, and why it is built this way: [docs/measurements.md](docs/measurements.md).

## 🐝 Made by HivePaaS

tracewing is built and maintained by HivePaaS, who also make:

- [hivepaas](https://github.com/hivepaas/hivepaas): a self-hosted PaaS.
- [shellwing](https://shellwing.app): a fast, native SSH client for macOS, Windows and Linux.
- [packwing](https://github.com/hivepaas/packwing): a file archiver for macOS, Windows and Linux that opens archives safely.
- [authwing](https://github.com/hivepaas/authwing): a two-factor code app for macOS, Windows and Linux that knows the window you are in.

## 📜 License

tracewing is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License, version 3 or (at your option) any later version. See [LICENSE](LICENSE). Copyright (C) 2026 HivePaaS.

The name and logo are not covered by the GPL: see [TRADEMARKS.md](TRADEMARKS.md). Contributions are welcome under the terms in [CONTRIBUTING.md](CONTRIBUTING.md).
