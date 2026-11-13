# Releasing

A release is a tag and the files CI builds for it. Nothing reaches anybody
until the draft the tag makes is published by hand.

## What a release ships

| Platform | Files | Made by |
| --- | --- | --- |
| Linux x86_64, aarch64 | `tracewing_<v>_<amd64\|arm64>.deb`, `tracewing_<v>_<x86_64\|aarch64>.rpm` | `scripts/package_linux.sh` |
| macOS aarch64, x86_64 | not yet: `tracewing_<v>_macos_<arch>.dmg` | — |
| Windows x86_64 | `tracewing_<v>_windows_x86_64-setup.exe`, `tracewing_<v>_windows_x86_64.zip` | `scripts/package_windows.sh` |

and `SHA256SUMS`.

One binary holds everything — the window, the proxy, the command line and the
MCP server — so a package is the program, its launcher and icon, and the
licenses of what it is built with.

- **Linux**: built on Ubuntu 22.04, so the packages run on glibc 2.35 and newer
  (Ubuntu 22.04, Debian 12). The `.deb` is for Debian and Ubuntu, the `.rpm`
  for Fedora and openSUSE; the `.rpm` is made with `rpmbuild` on Ubuntu, so it
  names the glibc it needs and an older system refuses it at install rather
  than tracewing failing when it starts.
- Neither Linux package is signed: `dnf` warns and goes on, and `zypper` needs
  `--no-gpg-checks`.
- **Windows**: an NSIS installer, which asks whom for — everyone on the
  computer, by default, into `C:\Program Files\tracewing` with an
  administrator's permission (Windows asks), or the user alone into
  `%LOCALAPPDATA%\Programs\tracewing` with none. `/ALLUSERS` or
  `/CURRENTUSER` says it without the page, as `/S` does for a quiet install;
  where tracewing is installed already, it goes there again, as it was. Its
  folder goes in the PATH, so a terminal opened after runs `tracewing
  capture`, and uninstalling takes it out again and runs `tracewing uninstall`
  — the certificate authority out of the stores that trust it, and the system
  proxy back. It is not signed: SmartScreen asks the first time (More info ›
  Run anyway). The `.zip` is the same program to run from a folder, with no
  installer.
- **macOS** is a row in the matrix of
  [release.yml](../.github/workflows/release.yml) and a packaging script. The
  download page names its files already, so a release before it exists must
  say so there (`landing/src/data/downloads.ts` has `comingSoon` for it).

## Cutting one

1. `version` in `Cargo.toml`, in a commit of its own.
2. `docs/releases/vX.Y.Z.md`: the notes the release shows. The workflow stops
   if the file is not there.
3. The download page, in
   [tracewing-website](https://github.com/hivepaas/tracewing-website):
   `VERSION` in `landing/src/data/downloads.ts`, and the four things the
   comment at the top of that file lists — the file names, which Linux
   packages are built, whether each build is signed, and the oldest glibc the
   Linux build runs on.
4. The tag, which is what starts the release:

   ```sh
   git tag -a v0.1.0 -m "tracewing 0.1.0"
   git push origin v0.1.0
   ```

5. CI builds every platform, checks each package and installs it, and leaves a
   **draft** release with the files and their SHA-256.
6. Read the draft, download one package and open it, then publish it by hand.

The tag and `Cargo.toml` must agree: the workflow's first job fails if they do
not, before anything is built.

## A rehearsal, with no release

A pull request that changes the packaging (the paths in
[release.yml](../.github/workflows/release.yml)), or **Actions > Release > Run
workflow**, builds and packages the branch exactly as a tag does and keeps the
files as the run's artifacts. Nothing is drafted and no tag is needed.

## By hand, on this machine

```sh
make package                                        # dist/, for this system
scripts/check_linux_package.sh dist/tracewing_*.deb # what it holds
scripts/check_linux_package.sh dist/tracewing_*.rpm
```

It needs `cargo-about` (`cargo install cargo-about --locked --features cli`)
and, on Debian or Ubuntu, `dpkg-dev rpm cpio desktop-file-utils`.

On Windows, from Git Bash or MSYS2 with NSIS 3 installed
(`choco install nsis`):

```sh
cargo build --release --locked -p tracewing
scripts/package_windows.sh
pwsh scripts/check_windows_installer.ps1 dist/tracewing_*-setup.exe
```

That check installs tracewing on the machine it runs on, both ways, and
uninstalls it again.

`--install` goes further: it installs the package as a user does — the `.deb`
here with `sudo`, the `.rpm` in a Fedora container with Docker — and then runs
the program from it with no window, recording a session of its own and writing
a HAR. That is what CI runs.
