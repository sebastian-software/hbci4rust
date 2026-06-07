# hbci4rust

`hbci4rust` is an original-near Rust port scaffold for
[`hbci4j/hbci4java`](https://github.com/hbci4j/hbci4java), pinned to tag
`hbci4j-core-4.1.11`.

The first implementation target is FinTS PinTAN / HBCI-Plus. Chipcard, PCSC,
CTAPI, DDV, RDH, RAH, RSA key-file media, and Java passport import are
explicitly out of scope for v1.

This repository starts deliberately close to the Java package shape. Public Rust
types use Rust casing (`HbciHandler`, `HbciError`), while job names and property
keys remain Java-compatible (`SaldoReq`, `src.iban`, and similar).

## Current State

The v1 PinTAN/HBCI-Plus port is well beyond the initial bootstrap, but it is
not release-complete yet:

- ADRs record the major porting decisions.
- The crate exposes async callback and communication traits, including a replay
  transport for offline tests.
- Protocol XML/DTD resources are loaded and used for FinTS wire
  rendering/parsing.
- Signed PinTAN dialog init, custom message execution, dialog close, one-step
  TAN, and two-step TAN process helpers are implemented.
- The static job registry covers the v1 PinTAN-compatible Java job names.
- SEPA/CAMT, SWIFT/MT940, status, structures, and challenge helpers have
  original-near offline tests.
- Runtime configuration mirrors the original global `HBCIUtils` style.
- Rust-native PinTAN passport storage is implemented with a versioned encrypted
  envelope.

The remaining v1 work is mostly runtime confidence, broader replay fixtures,
public API hardening, release documentation, and packaging review. The current
readiness estimate and the Java migration map live in:

- `docs/architecture/v1-readiness.md`
- `docs/architecture/release-checklist.md`
- `docs/reference/public-api.md`
- `docs/reference/java-to-rust-mapping.md`
- `docs/reference/migration-examples.md`
- `docs/reference/error-reporting.md`
- `docs/reference/live-bank-tests.md`
- `docs/reference/packaging.md`
- `docs/reference/passport-storage-security.md`

## Development

```sh
cargo fmt
cargo test
cargo clippy --all-targets
```

CI is offline-only. Live bank access stays ignored and environment-gated.

Fetch the pinned Java reference locally when needed:

```sh
scripts/fetch-upstream.sh
```
