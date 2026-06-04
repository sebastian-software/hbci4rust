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

This is the bootstrap slice for the port:

- ADRs record the major porting decisions.
- The crate exposes async callback and communication traits.
- A static job registry reserves the PinTAN-compatible Java job names.
- Runtime configuration mirrors the original global `HBCIUtils` style.
- Rust-native PinTAN passport storage is implemented with a versioned encrypted
  envelope.

Protocol execution, full message generation/parsing, SEPA/CAMT parity, and live
PinTAN dialogs are the next porting slices.

## Development

```sh
cargo fmt
cargo test
cargo clippy --all-targets
```

CI is offline-only. Live bank access, when added, must stay ignored and
environment-gated.

Fetch the pinned Java reference locally when needed:

```sh
scripts/fetch-upstream.sh
```
