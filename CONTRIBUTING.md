# Contributing to tracewing

Thank you for helping. Issues and pull requests are welcome; for anything large,
open an issue first so we can agree on the shape before you write it.

## Before a pull request

- `make lint` and `make test` pass. Prefer them over calling `cargo fmt` and
  `cargo clippy` yourself: `make lint` also checks the translations and runs
  clippy a second time with the window tests compiled in, both of which CI
  checks and neither of which plain `cargo clippy` sees. The message check in
  particular fails on the *order* of the strings, so **moving** a `tr!` call
  is enough to break it: `python3 scripts/i18n.py update` puts it right.
- **A pull request aimed at a branch rather than at `main` gets no CI run.**
  The workflow takes pushes to `main` and pull requests aimed at `main`, so a
  branch stacked on another branch sits there with no tick at all — which
  looks much like a tick that has not arrived yet. Start one by hand:

  ```sh
  gh workflow run CI --ref <your-branch> -f platforms=linux
  ```

  It runs by itself once the branch underneath merges and the pull request
  retargets to `main`.
- Changes people see come with a line in `docs/features.md`, and in
  `docs/building.md` when they change how tracewing is built or tested. The
  README is the project's front page: it says what tracewing is in a line a
  feature, and changes only for something that changes that picture.

## Licensing your contribution

tracewing is licensed under GPL-3.0-or-later and maintained by HivePaaS, who
also ships builds where the GPL cannot be used as is (for example on app
stores). So that this stays possible, by submitting a contribution you agree
that:

1. You wrote it, or otherwise have the right to submit it under these terms
   (the [Developer Certificate of Origin 1.1](https://developercertificate.org/)).
2. It is licensed to everyone under GPL-3.0-or-later, like the rest of the code.
3. You also grant HivePaaS a perpetual, worldwide, non-exclusive, royalty-free,
   irrevocable license to use, modify and distribute it under other terms as
   well, including proprietary ones, and a license to any of your patents that
   it necessarily practises.

You keep the copyright in your work. Sign off your commits (`git commit -s`) to
confirm the above.
