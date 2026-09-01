# Open-source release checklist

Use this list before changing the repository to public or announcing the first
release.

## Required before public access

- [x] Add the GNU Affero General Public License v3.0-only as `LICENSE` and set
  the Cargo SPDX expression to `AGPL-3.0-only`.
- [x] Review license metadata for every package in the locked Cargo dependency
  graph. The 2026-08-05 audit found no dependency with a missing license
  declaration; repeat the audit whenever dependencies change.
- [ ] Confirm that the repository owner wrote the bundled default prompts or
  otherwise has permission to redistribute them under `AGPL-3.0-only`.
- [x] Scan the current tree and full Git history for credentials, private URLs,
  customer data, and unintended author metadata. A manual pattern scan was
  completed on 2026-08-05; repeat it with a dedicated secret scanner immediately
  before publication.
- [ ] Enable GitHub private vulnerability reporting so the link in
  `SECURITY.md` accepts reports.
- [ ] Review the public issue tracker, pull requests, releases, repository
  description, topics, and default branch settings.

## Repository readiness

- [x] README explains installation, configuration, usage, boundaries, and exit
  status.
- [x] Contribution and security-reporting guides are present.
- [x] Architecture and product-scope documents are linked from the README.
- [x] Local tests do not require a real API key or model service.
- [x] Add continuous integration for formatting, Clippy, and tests on supported
  operating systems.
- [ ] Decide whether to add a code of conduct, issue templates, and a pull
  request template before inviting broad community participation.

## First release

- [x] Use GitHub binaries for rolling `main` snapshots. Keep crates.io and
  versioned-release policy as separate decisions before the first stable tag.
- [ ] If publishing to crates.io, confirm package-name availability and complete
  the package metadata.
- [ ] Test the documented installation and quick-start commands in a clean
  environment.
- [x] Build and test every advertised native platform or clearly label unverified
  targets.
- [x] Review `cargo package --list` so local workspaces, logs, credentials, and
  build output cannot enter the package. An offline package verification on
  2026-08-05 produced 34 files and rebuilt successfully from the packaged
  source.
- [ ] Create a versioned changelog or release notes, tag the exact release
  commit, and publish checksums for downloadable binaries.

## Verification commands

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo package --offline
git status --short
```

Run an external secret scanner before publication when one is available. Manual
pattern matching is useful but cannot prove that a repository contains no
sensitive information.
