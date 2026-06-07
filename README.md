# hbci4rust

`hbci4rust` is an original-near Rust port of
[`hbci4j/hbci4java`](https://github.com/hbci4j/hbci4java), pinned to tag
`hbci4j-core-4.1.11` / commit
`3b7ce667c73724daa1c836ed7333ed090c21a831`.

The v1 target is FinTS PinTAN / HBCI-Plus: modern HTTPS FinTS with PIN/password,
TAN, app approval, photoTAN, QR/chipTAN-style challenge data, and replayable
offline tests. The port intentionally keeps Java job names and parameter keys
visible, while Rust public types use Rust casing:

```rust
use hbci4rust::{HbciHandler, Konto, PinTanPassport, PinTanPassportData, ReplayCommClient};

fn main() -> hbci4rust::HbciResult<()> {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });

    let mut handler = HbciHandler::with_comm("300", passport, ReplayCommClient::default());
    let mut job = handler.new_job("SaldoReq")?;
    job.set_param_account(
        "my",
        &Konto {
            iban: Some("DE02123456780012345678".to_owned()),
            ..Konto::default()
        },
    );
    handler.try_add_to_queue(job)?;
    Ok(())
}
```

## Status

The scoped v1 PinTAN/HBCI-Plus port is release-candidate complete against
`docs/architecture/release-checklist.md`: **50 of 50 blocking checklist items
are resolved**.

This means the selected v1 surface is covered, not that every historical
hbci4java feature is strategic or recommended. The current source-surface audits
show:

| Audit | Upstream surface | Rust v1 coverage | Intentional gap |
| --- | --- | --- | --- |
| Jobs | 68 `GV*.java` classes | 67 registered jobs | `GVTemplate` |
| Results | 24 normalized `GVR*.java` shapes | 23 typed result shapes | `WPStammData` |

Both audit gaps are lowlevel boundaries. `GVTemplate` is Java's dynamic
`newLowlevelJob(...)` fallback, and `WPStammData` is tied to the lowlevel
`WPStammList` path. They are not hidden PinTAN implementation holes.

Some classic hbci4java payment jobs are currently still present for
original-near compatibility, but they are not the product direction. See
`docs/reference/modern-scope-audit.md` for the current split between modern v1
surface, compatibility-carried legacy surface, and unsupported legacy surface.
The cleanup path for those compatibility-carried jobs is documented in
`docs/architecture/legacy-cleanup-plan.md`.

## What V1 Includes

- Async `HbciHandler` dialog flow with explicit callback and communication
  traits.
- Default HTTPS communication plus replay clients for deterministic offline
  tests.
- Original protocol XML/DTD resource loading and FinTS wire parsing/rendering.
- Static registry for the PinTAN-compatible hbci4java job names.
- Typed result data for the supported high-level result shapes.
- SEPA/CAMT, SWIFT/MT940, status, structure, challenge, QR, matrix, and flicker
  helpers.
- Signed HBCI-Plus/PinTAN dialog init, execution, close, one-step TAN,
  process-1, process-2, decoupled polling, TAN media selection, and QR/photoTAN
  callback coverage.
- Rust-native encrypted PinTAN passport storage using an Argon2id and
  XChaCha20-Poly1305 envelope.

## What V1 Excludes

These hbci4java surfaces are deliberately outside v1:

- HBCI signature-card runtime support;
- PCSC, CTAPI, DDV, and native card-reader integration;
- RDH, RAH, and RSA key-file live support;
- Java passport import/export;
- arbitrary dynamic lowlevel jobs through a public `newLowlevelJob(...)`
  equivalent;
- live-bank credentials or live-bank tests as a release requirement.

`chipTAN` is still in scope as a PinTAN/SCA mechanism. The excluded "chipcard"
surface means classic HBCI signature-card support with card readers and
signature media, not a TAN generated from a debit card plus TAN generator.

## Non-Legacy Scope

The scope is based on both local port evidence and current external source
checks. This is not a "PinTAN first, legacy later" roadmap. The intended
publication stance is a useful modern FinTS PinTAN/HBCI-Plus port without
strategic support for historical security media or national pre-SEPA payment
rails.

Current evidence supports that stance:

- FinTS still includes signature-card and TAN-based paths, but banks commonly
  document PIN/TAN, app approval, photoTAN, BestSign, SecurePlus, pushTAN,
  chipTAN, or other SCA flows for current financial-software access.
- Sparkasse still explains classic HBCI chipcard as secure but laborious and
  says it is not recommended today.
- Bundesbank SEPA documentation records that national credit transfer and
  direct-debit schemes were replaced by SEPA, with German transition allowances
  ending in 2016.
- EPC guidance says the SEPA `COR1` local instrument is no longer relevant for
  new SDD Core collections from 20 November 2016.
- EU/ECB instant-payment guidance makes `InstUebSEPA` and verification-style
  work more relevant than classic domestic transfer variants.

The detailed evidence and source links live in
`docs/reference/security-media-scope.md` and
`docs/reference/modern-scope-audit.md`.

## Documentation Map

- `docs/architecture/porting-plan.md`: original-near porting plan and tracking
  links.
- `docs/architecture/v1-readiness.md`: evidence-backed v1 readiness matrix.
- `docs/architecture/release-checklist.md`: operational release acceptance
  checklist.
- `docs/architecture/job-coverage.md`: `GV*.java` to Rust job registry audit.
- `docs/architecture/result-coverage.md`: `GVR*.java` to Rust typed result
  audit.
- `docs/reference/public-api.md`: crate-root API review.
- `docs/reference/java-to-rust-mapping.md`: Java concept to Rust API mapping.
- `docs/reference/migration-examples.md`: checked high-risk workflow examples.
- `docs/reference/security-media-scope.md`: PinTAN-only scope evidence.
- `docs/reference/modern-scope-audit.md`: modern versus legacy-carried surface
  audit.
- `docs/architecture/legacy-cleanup-plan.md`: guarded cleanup order for
  compatibility-carried jobs.
- `docs/reference/unsupported-surfaces.md`: public v1 boundaries.
- `docs/reference/passport-storage-security.md`: storage security review.
- `docs/reference/packaging.md`: crate metadata and package review.
- `docs/adr/`: architectural decision records for the port.

## Development

```sh
cargo fmt
cargo test
cargo clippy --all-targets
scripts/run-release-candidate-checks.sh --package
```

CI is offline-only. Live bank access stays ignored and environment-gated.

Fetch the pinned Java reference locally when needed:

```sh
scripts/fetch-upstream.sh
```
