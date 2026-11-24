#!/usr/bin/env python3
"""Adds a release to release.json, from the SHA256SUMS its draft carries.

    scripts/release_json.py v0.1.0 SHA256SUMS [--date 2026-11-24] [--file release.json]

The release's entry names the files an installed copy's updater downloads
(`archives`) and the system packages whose commands it shows (`packages`), each
with the SHA-256 the draft's own SHA256SUMS gives it, so that no hash is copied
by hand. The channel the version is on (stable, or beta for `-betaN`) is moved
to it. The file is written as json.dumps(indent=2) writes it, with a newline at
the end, which is what scripts/release_sign.sh signs byte for byte.

Every file this names has to be in SHA256SUMS: a release missing one is
refused rather than listed without it, since a copy that cannot find its file
is a copy that never updates. See docs/updates.md.
"""

import argparse
import datetime
import json
import re
import sys
from pathlib import Path

# What crates/update/src/platform.rs names, for what tracewing's releases carry.
ARCHIVES = {
    "macos_aarch64": "tracewing_{v}_macos_aarch64.app.tar.gz",
    "macos_x86_64": "tracewing_{v}_macos_x86_64.app.tar.gz",
    "windows_x86_64": "tracewing_{v}_windows_x86_64-setup.exe",
}
PACKAGES = {
    "deb_x86_64": "tracewing_{v}_amd64.deb",
    "deb_aarch64": "tracewing_{v}_arm64.deb",
    "rpm_x86_64": "tracewing_{v}_x86_64.rpm",
    "rpm_aarch64": "tracewing_{v}_aarch64.rpm",
}
VERSION = re.compile(r"^v\d+\.\d+\.\d+(-beta\d+)?$")


def sums(path):
    """file -> sha256, as `sha256sum` writes them."""
    out = {}
    for line in Path(path).read_text().splitlines():
        if not line.strip():
            continue
        sha, name = line.split(None, 1)
        name = name.lstrip("*")
        if not re.fullmatch(r"[0-9a-f]{64}", sha):
            sys.exit(f"release_json: {path}: not a SHA-256 for {name}: {sha}")
        out[name] = sha
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("version", help="the tag, v0.1.0 or v0.2.0-beta1")
    ap.add_argument("sums", help="the draft's SHA256SUMS")
    ap.add_argument("--date", default=datetime.date.today().isoformat())
    ap.add_argument("--file", default="release.json")
    args = ap.parse_args()
    if not VERSION.match(args.version):
        sys.exit(f"release_json: {args.version} is not a version such as v1.2.3 or v1.2.3-beta1")
    plain = args.version[1:]
    have = sums(args.sums)

    def entries(names):
        out = {}
        for key, pattern in names.items():
            file = pattern.format(v=plain)
            if file not in have:
                sys.exit(f"release_json: {args.sums} has no {file}")
            out[key] = {"file": file, "sha256": have[file]}
        return out

    path = Path(args.file)
    data = json.loads(path.read_text()) if path.exists() else {"channels": {}, "releases": {}}
    data.setdefault("channels", {})
    data.setdefault("releases", {})
    data["releases"][args.version] = {
        "releaseDate": args.date,
        "archives": entries(ARCHIVES),
        "packages": entries(PACKAGES),
    }
    channel = "beta" if "-beta" in args.version else "stable"
    data["channels"][channel] = args.version
    path.write_text(json.dumps(data, indent=2) + "\n")
    print(f"{path}: {args.version} on {channel}, {len(ARCHIVES)} archives, {len(PACKAGES)} packages")


if __name__ == "__main__":
    main()
