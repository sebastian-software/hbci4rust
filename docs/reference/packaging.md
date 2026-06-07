# Packaging Review

Snapshot date: 2026-06-07.

This page records the current crate packaging review for the scoped v1
PinTAN/HBCI-Plus port. Final local package evidence is captured by
`scripts/run-release-candidate-checks.sh --package`; rerun it after any
source-controlled change before publishing.

## Cargo Metadata

Reviewed package metadata:

- crate name: `hbci4rust`
- version: `0.1.0`
- edition: `2024`
- license: `LGPL-2.1-or-later`
- readme: `README.md`
- package repository: `https://github.com/sebastian-software/hbci4rust`
- description: original-near hbci4java port focused on FinTS
  PinTAN/HBCI-Plus

The package-level `repository` field points to the Rust port. The upstream Java
baseline remains recorded separately in `[package.metadata.hbci4rust.upstream]`:

- repository: `https://github.com/hbci4j/hbci4java.git`
- tag: `hbci4j-core-4.1.11`
- commit: `3b7ce667c73724daa1c836ed7333ed090c21a831`

## Package Contents

`cargo package --list` was reviewed for this snapshot. The package includes the
expected crate and porting artifacts:

- root metadata: `Cargo.toml`, `Cargo.lock`, `LICENSE`, `NOTICE`, `README.md`,
  `rustfmt.toml`, `.gitignore`, and generated Cargo package metadata;
- CI: `.github/workflows/ci.yml`;
- decision records: `docs/adr/`;
- architecture and release tracking: `docs/architecture/`;
- migration/reference notes: `docs/reference/`;
- rustification backlog: `docs/rustification/`;
- original bank-info resources: `resources/bank_info/`;
- original protocol resources: `resources/protocol/`;
- maintenance scripts: `scripts/`;
- library sources: `src/`;
- offline tests and copied upstream fixtures: `tests/`.

The `Cargo.toml.orig` entry in the package listing is Cargo's generated package
manifest copy. It is expected in `cargo package --list` output and is not a
tracked source file.

## Attribution

Direct upstream attribution is present in:

- `NOTICE`;
- `docs/adr/0002-license-and-attribution.md`;
- `docs/reference/upstream-header-review.md`;
- `resources/bank_info/README.md`;
- `resources/protocol/README.md`;
- `tests/fixtures/hbci4java/README.md`.

Copied bank-info resources, protocol XML/DTD resources, and offline fixtures
name the pinned hbci4java repository, tag, commit, and upstream source context.

## Publication Guardrails

Before publishing any crate release after further source-controlled changes:

- rerun `cargo package --list` on the final release-candidate commit;
- use `scripts/run-release-candidate-checks.sh --package` to run the offline
  gates and package checks with per-command logs under `target/release-gates/`;
- rerun the upstream header review if the baseline or copied artifacts changed
  since `docs/reference/upstream-header-review.md`;
- update `docs/architecture/release-checklist.md` if the release acceptance
  command set changes.
