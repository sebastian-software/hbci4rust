# Original-Near Porting Plan

The first milestone is not a full banking client. It is a stable, documented
foundation that makes incremental hbci4java parity work possible.

## Baseline

- Upstream repository: `hbci4j/hbci4java`
- Baseline tag: `hbci4j-core-4.1.11`
- Baseline commit: `3b7ce667c73724daa1c836ed7333ed090c21a831`

## v1 Scope

In:

- FinTS PinTAN / HBCI-Plus
- Java-compatible job names and property keys
- Protocol XML resources and parser/generator behavior
- SEPA, CAMT, SWIFT/MT940 offline parity
- Async callback and communication model
- Rust-native encrypted PinTAN passport storage

Out:

- Chipcard, PCSC, CTAPI, DDV
- RDH, RAH, RSA key-file live support
- Java passport import/export
- Live bank tests in CI

## Port Order

1. Bootstrap crate, ADRs, CI, scripts, public API skeleton.
2. Port foundational utilities, protocol structures, status types, and fixtures.
3. Port offline SEPA/CAMT/SWIFT/BPD/message behavior with Java goldens.
4. Port async PinTAN handler/dialog runtime and all PinTAN-compatible jobs.
5. Harden docs, optional live tests, and rustification candidates.

## Tracking

- `docs/architecture/v1-readiness.md` records the current evidence-backed v1
  readiness estimate and completion bar.
- `docs/architecture/job-coverage.md` records the current upstream `GV*.java`
  to Rust registry coverage and the intentional `GVTemplate` boundary.
- `docs/architecture/result-coverage.md` records the current upstream
  `GVR*.java` to Rust typed result coverage and the intentional
  `WPStammData` boundary.
- `docs/reference/java-to-rust-mapping.md` maps major hbci4java concepts to the
  current original-near Rust API.
- `docs/reference/public-api.md` reviews the crate-root v1 export surface.
- `docs/reference/migration-examples.md` records public examples for high-risk
  statement and SEPA workflows.
- `docs/reference/error-reporting.md` records how Rust errors, execution
  statuses, return values, and known return-code helpers map to hbci4java-style
  diagnostics.
- `docs/reference/unsupported-surfaces.md` records the public v1 boundaries for
  security media, Java passport compatibility, dynamic lowlevel jobs, typed
  result exclusions, live testing, and release limitations.
- `docs/reference/security-media-scope.md` records the local audit evidence and
  current bank/provider source snapshot behind the PinTAN-only security-media
  boundary.
- `docs/reference/parser-generator-goldens.md` records current parser/generator
  golden fixture coverage and explicit v1 limitations.
- `docs/reference/malformed-bank-responses.md` records deterministic coverage
  and limitations for malformed or unexpected bank responses.
- `docs/reference/live-bank-tests.md` describes the ignored, env-gated live
  PinTAN dialog smoke hook.
- `docs/reference/packaging.md` records crate metadata, package-list, and
  attribution review for release hardening.
- `docs/reference/passport-storage-security.md` records the KDF, AEAD, envelope,
  and validation review for Rust-native PinTAN passport storage.
- `docs/reference/upstream-header-review.md` records the current upstream
  license/header inconsistency recheck for the pinned baseline.
- `docs/architecture/release-checklist.md` records the operational v1 release
  acceptance checks.
- `scripts/run-release-candidate-checks.sh` runs the release-candidate gate set
  and can include final package checks with `--package`.
