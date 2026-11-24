# Release signing keys

The public keys tracewing believes its list of releases (`release.signed.json`)
from. Every `<key-id>.pub.pem` here is built into the app (`build.rs`); its key
id is its file name. The list must carry a valid **Ed25519** (Ed25519ctx) and a
valid **ML-DSA-65** signature, each by a key here, under the context
`tracewing-release-v1`.

A build with no key here cannot tell of updates: it does not look, and
*Settings › About* says so.

## Shared with shellwing

These are **shellwing's release keys**, the same two files as shellwing's
`crates/update/keys/` (`2026-ed`, `2026-ml`): one pair of offline keys signs
both apps' lists. What keeps a list signed for one app from being taken by the
other is the **context** each signature is made under: `tracewing-release-v1`
here, `shellwing-release-v1` there. Both Ed25519ctx and ML-DSA bind the context
into the signature, so a shellwing list fails tracewing's check, and the other
way round (`envelope`'s tests open the test list under shellwing's context and
are refused).

`scripts/release_sign.sh` checks that each private key it is given is the one
built in here under its id, so a list cannot be signed with a key no copy of
tracewing believes.

## Making the keys

They already exist: they were made for shellwing, on its offline signing
machine, with the tool `tools/releasesign` (this repository has the same tool,
with tracewing's context):

```sh
releasesign keygen -alg ed25519   -key-id 2026-ed -dir /offline
releasesign keygen -alg ml-dsa-65 -key-id 2026-ml -dir /offline
```

Only the two `.pub.pem` files are copied here. The `.key` files never leave that
machine; back them up as you would any secret that cannot be made again: an app
trusts the keys it was built with, and a lost key is replaced only by shipping
apps that trust another.

## Rotating a key

Rotating is done for both apps together, since they share the keys:

1. Make the new key, copy its `.pub.pem` here and into shellwing beside the old
   one, and release both.
2. Sign with both the old and the new key for as long as apps built before (1)
   are about.
3. Then take the old key out of both, and stop signing with it.
