# Updates

tracewing tells its user when a newer version is out: a badge on the title bar,
as packwing's, and a line in *Settings › About*. It looks once a day while *Look for a new
version once a day* is on (`check_updates` in `settings.toml`, on by default),
and when asked (*Check now*). Nothing is downloaded or installed until the user
says so (see [Installing](#installing)).

What a look sends is one HTTPS request to `raw.githubusercontent.com`, for the
signed list below, made straight and not through any proxy (the system's or
`HTTPS_PROXY`, which may be this very tracewing). GitHub sees this computer's
address and nothing else. The window tests never make it: a build for them
looks only at a list a test names.

The badge never takes the keyboard. It says *Update available*, or *Restart to
update* once a release is downloaded, and a click on it opens About, where the
release is got, installed or skipped (*Skip this version* offers it no more).
With the daily look off, it shows only a release already downloaded. Where the
system draws the title bar (X11 with no client-side decorations), a dot on the
logo at the top of the rail says it instead, and the logo opens About.

The design is shellwing's (its `docs/updates.md`), ported: the same list, the
same envelope, the same keys, under a context of tracewing's own.

## Channels

A version is **stable** (`v0.2.0`) or a **beta** (`v0.3.0-beta2`); the app's
channel is its own version's.

- On stable, the app is offered newer stable releases only, never a beta.
- On a beta, it is offered the newer of the newest beta and the newest stable: a
  release supersedes the betas before it (`v0.3.0` after `v0.3.0-beta2`), and from
  there the app is on stable.

## The list of releases

What the app reads is `release.signed.json` on the repository's `release` branch:

```
https://raw.githubusercontent.com/hivepaas/tracewing/release/release.signed.json
```

It is `release.json` (the readable copy, reviewed in the repository), base64,
with its hash and its signatures beside it in one file, so that a CDN caching
files one by one can never serve a new list with an old signature. `release.json`:

```json
{
  "channels": { "stable": "v0.1.0" },
  "releases": {
    "v0.1.0": {
      "releaseDate": "2026-11-24",
      "archives": {
        "macos_aarch64": { "file": "tracewing_0.1.0_macos_aarch64.app.tar.gz", "sha256": "…" },
        "macos_x86_64": { "file": "tracewing_0.1.0_macos_x86_64.app.tar.gz", "sha256": "…" },
        "windows_x86_64": { "file": "tracewing_0.1.0_windows_x86_64-setup.exe", "sha256": "…" }
      },
      "packages": {
        "deb_x86_64": { "file": "tracewing_0.1.0_amd64.deb", "sha256": "…" },
        "deb_aarch64": { "file": "tracewing_0.1.0_arm64.deb", "sha256": "…" },
        "rpm_x86_64": { "file": "tracewing_0.1.0_x86_64.rpm", "sha256": "…" },
        "rpm_aarch64": { "file": "tracewing_0.1.0_aarch64.rpm", "sha256": "…" }
      }
    }
  }
}
```

`scripts/release_json.py` writes a release's entry from the draft's
`SHA256SUMS`, so the hashes are copied by nothing but a script.

The app refuses a list that does not hold together: a version not written
`vX.Y.Z` or `vX.Y.Z-betaN`, a channel naming a release it does not list, a beta on
stable, a SHA-256 that is not one, a file not named as its platform's is
(below).

**A system package never goes in `archives`.** An app refuses a list whose
`archives` names a platform it does not know, so that would end updates for every
copy already installed. `packages` is a separate field, and an app passes over a
kind of package it does not know. For the same reason the app already knows the
names of platforms tracewing has no file for yet (`linux_x86_64`, an AppImage;
`windows_aarch64`): a later release may add one without stopping the copies out
there.

## Installing

What the app does with a release depends on how it was installed, as told from
where it runs:

| installed as | the app |
|---|---|
| by its installer on Windows (`…\AppData\Local\Programs\tracewing\`, `…\Program Files\tracewing\`) | downloads the installer and runs it, quiet (`/S`), once tracewing has quit |
| `tracewing.app` on macOS | downloads the `.app` (tarred), checks it with `codesign` (valid, signed by the team that signed this one), and swaps the two |
| from the release's `.deb` or `.rpm` (`/usr/bin/tracewing`) | shows the commands that install the next one, to copy into a terminal (see below) |
| by Homebrew, Scoop, Flatpak, Snap, a store | says what updates it (`brew upgrade tracewing`, …) |
| by hand (the Windows `.zip`, a build of one's own) | opens the release's page |

A release has one file for each platform it replaces (`v0.1.0`):

| platform | file |
|---|---|
| `macos_aarch64`, `macos_x86_64` | `tracewing_0.1.0_macos_aarch64.app.tar.gz` |
| `windows_x86_64` | `tracewing_0.1.0_windows_x86_64-setup.exe` |

They are the release's assets on GitHub
(`https://github.com/hivepaas/tracewing/releases/download/v0.1.0/<file>`); the
list names each one's SHA-256, and the app keeps a download only when its hash is
that one. The signed list is read again, and checked again, before each download.

*Download and install* downloads the file to `updates/` in the data folder, and
says the release is ready. It is installed as tracewing quits, after the system
proxy and everything else tracewing changed has been put back: at *Restart now*,
which starts the new one, or whenever the user quits. A release downloaded by an
earlier run is still ready at the next start; once the app running is that
version or newer, its file goes.

**On Windows** the installer runs once tracewing's own process has gone
(`Wait-Process` on its id, a minute at most): it copies `tracewing.exe` over the
old one and gives up after five seconds of trying when quiet, and quitting takes
as long as the proxy takes to stop. It is told this copy's mode and folder
(`/CURRENTUSER` or `/ALLUSERS`, and `/D=`), since left to itself it goes where
the registry names an install for everyone first, and on a computer with both
the user's copy would update the other. An install for everyone asks Windows
for an administrator's rights, so the user is asked. Installing over a copy
never runs the old uninstaller, so the certificate authority and the settings
stay as they are.

**Another tracewing from the same folder still running stops it**: a
`tracewing mcp` an AI tool started, a `tracewing capture` in a terminal. The
installer cannot replace the program, gives up as it does when quiet, and
`Start-Process` does not pass its exit code on. tracewing starts again as it
was; the downloaded installer is still there, so the next start says the
update is ready again, and *Restart now* tries once more.

**On macOS** the network extension the old app installed keeps running until
the new app asks for it again, which it does when its way in is switched on:
macOS replaces an extension by one of another version
(`actionForReplacingExtension` answers *Replace*).

### The release's `.deb` and `.rpm`

Installing a package takes root, so the app does not do it: it shows the commands
and the user runs them. They name the right file for the system, with the SHA-256
from the signed list:

```sh
cd "$(mktemp -d)" &&
curl -fLO https://github.com/hivepaas/tracewing/releases/download/v0.2.0/tracewing_0.2.0_amd64.deb &&
echo '<sha256>  tracewing_0.2.0_amd64.deb' | sha256sum -c &&
sudo apt install ./tracewing_0.2.0_amd64.deb
```

The tool is told from the system: `apt` when dpkg lists the package, else `zypper`
or `dnf`, whichever it has. zypper is given `--no-gpg-checks`, as the packages are
not signed; the SHA-256 vouches for the file. If the list names no package for the
system, the app opens the release's page.

## Signing

The list is signed twice, with **Ed25519** (Ed25519ctx) and **ML-DSA-65**, under
the context `tracewing-release-v1`; the app requires both, by the keys built into
it (`crates/update/keys/`, whose README says how the keys are made and rotated).

**The keys are shellwing's.** One pair of offline keys signs both apps' lists;
the context is what keeps a list signed for one from being taken by the other,
since both algorithms bind it into the signature.

The tool is `tools/releasesign/main.go`: one file, the Go standard library only.
It is never run from the working tree: `scripts/release_sign.sh` builds it from
the reviewed commit in `tools/releasesign/PIN`, with the network off, signs, and
then checks the result again with OpenSSL 3.5 or later (the payload byte for byte,
each signature against the public key OpenSSL derives from the private key, and
that key against the one built into tracewing under its id).

```sh
# On the signing machine, in a clone of the repository:
scripts/release_sign.sh /offline/2026-ed.key /offline/2026-ml.key
```

Moving `PIN` to a newer commit is how a change to the tool reaches the keys: only
after it is reviewed.

## Publishing a release's list

1. Add the release to `release.json` (`scripts/release_json.py`, from the
   draft's `SHA256SUMS`), and its channel; commit it.
2. Sign it (above); commit `release.signed.json` with it.
3. Move the `release` branch to that commit: from then on the apps see it.

Before the `release` branch exists the app finds no list, and says it is up to
date.

## Checking it

- `cd tools/releasesign && go test` checks the tool: what it signs verifies, one
  algorithm alone is refused, a key is never written over.
- `cargo test -p tracewing-update` checks the app's side against a list the tool
  signed (`crates/update/testdata/make.sh` makes it, with test keys thrown away),
  and checks the repository's own `release.json` and `release.signed.json` once
  they exist.
- `cargo test -p tracewing --features headless --test updates` serves that list
  on this machine and goes through the window: told of, said in About and over
  the traffic, downloaded, checked, installed as tracewing quits, skipped, and a
  list signed by keys this build does not trust refused.
- A build can be pointed at a list, files and keys of its own, at build time:
  `TRACEWING_UPDATE_LIST` (the list's URL), `TRACEWING_UPDATE_FILES` (where
  `<version>/<file>` are), `TRACEWING_RELEASE_KEYS` (a folder of `.pub.pem`).
  That is how the installer and the `.app` swap are tried on a Windows and a
  macOS machine before a release relies on them.
