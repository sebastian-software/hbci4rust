# Unsupported V1 Surfaces

Snapshot date: 2026-06-07.

This page is the public reference for hbci4java surfaces that the scoped v1
PinTAN/HBCI-Plus port deliberately does not support. It matches the current job
and result coverage audit exclusions and the release checklist scope.

The page does not describe permanent non-goals. It describes the v1 boundary. A
future release may widen the boundary, but only after a new ADR records the
decision and the affected audits, docs, and tests are updated.

## In Scope

v1 targets original-near FinTS PinTAN / HBCI-Plus behavior:

- Java-compatible job names such as `SaldoReq`;
- Java-compatible parameter keys such as `src.iban`;
- original protocol XML/DTD resource loading;
- offline SEPA, CAMT, SWIFT/MT940, status, structure, challenge, and message
  behavior covered by fixtures and replay tests;
- async PinTAN dialog, callback, communication, and replay clients;
- Rust-native encrypted PinTAN passport storage.

## Security Media And Passport Boundaries

The following hbci4java security media are outside v1:

- chipcard;
- PCSC;
- CTAPI;
- DDV;
- RDH;
- RAH;
- RSA key-file live support.

The v1 port also does not import, export, or interpret Java passport files. The
only persisted passport format is the Rust-native PinTAN envelope documented in
`docs/reference/passport-storage-security.md`.

## Dynamic Lowlevel Boundary

The public v1 job surface is a static PinTAN-compatible registry. It does not
expose hbci4java's arbitrary lowlevel job creation API:

- no public `newLowlevelJob(...)` equivalent;
- no public `GVTemplate` registry entry;
- no arbitrary caller-selected lowlevel segment names.

`CustomMsg` remains in scope because `GVCustomMsg` is a concrete hbci4java job
class. It is not treated as permission to construct arbitrary lowlevel jobs.

`HbciJob::lowlevel_param(...)` and related lowlevel inspection helpers stay
available for original-near rendering, result inspection, and tests. They do
not widen v1 into a dynamic lowlevel API.

The current job coverage audit therefore allows exactly one missing upstream
`GV*.java` class: `Template`.

## Typed Result Boundary

The current typed-result surface intentionally excludes `WPStammData`.
hbci4java documents `GVRWPStammData` as requiring the lowlevel `WPStammList`
path rather than a normal high-level job.

The v1 port may still preserve raw result data for jobs that do not yet have a
dedicated typed payload. That is separate from promising a public typed
`WPStammData` result shape.

The current result coverage audit therefore allows exactly one missing
normalized upstream `GVR*.java` shape: `WPStammData`.

## Runtime And Testing Boundaries

CI acceptance is offline-only:

- `cargo fmt --check`;
- `cargo clippy --all-targets`;
- `cargo test`;
- `cargo test -- --list`;
- source-surface audit scripts when the local upstream reference is available.

Live-bank hooks are optional, ignored, and environment-gated. They must not
store real credentials, and v1 acceptance does not depend on live bank access.
The current live observation log records no manual observations for v1
acceptance.

Bank-specific SCA variants discovered during manual live smoke testing must be
converted into deterministic replay fixtures or documented as explicit
limitations before they influence acceptance.

## Release And Packaging Boundaries

These items remain release-candidate work, not hidden implementation support:

- final release-candidate offline gate output must be recorded after the last
  release commit;
- upstream header inconsistencies must be rechecked before publishing;
- risky parser or generator behavior needs Java golden artifacts or explicit
  limitation entries, tracked in `docs/reference/parser-generator-goldens.md`;
- malformed bank responses added before v1 need deterministic replay or fixture
  coverage, tracked in `docs/reference/malformed-bank-responses.md`;
- a persisted-format migration test is required before the first storage format
  revision.

## Widening Rules

Before any unsupported surface above becomes part of the public API:

- add a new ADR;
- follow the baseline/scope guard in
  `docs/adr/0258-baseline-and-scope-change-guard.md`;
- update this page;
- update `docs/architecture/release-checklist.md`;
- update job or result coverage notes when the source audit boundary changes;
- add focused offline tests or replay fixtures;
- add migration notes when Java users need a different Rust call shape.

## References

- `docs/architecture/release-checklist.md`
- `docs/architecture/job-coverage.md`
- `docs/architecture/result-coverage.md`
- `docs/reference/public-api.md`
- `docs/reference/java-to-rust-mapping.md`
- `docs/reference/live-bank-tests.md`
- `docs/reference/malformed-bank-responses.md`
- `docs/reference/passport-storage-security.md`
- ADR 0003: V1 PinTAN Scope
- ADR 0232: CustomMsg Job And Template Boundary
- ADR 0233: GV Job Coverage Audit
- ADR 0234: GV Result Coverage Audit
- ADR 0236: Optional Live Bank Test Hooks
- ADR 0246: V1 Release Checklist
- ADR 0252: Unsupported V1 Surface Reference
- ADR 0255: Malformed Bank Response Evidence
- ADR 0257: Live Smoke Observation Boundary
- ADR 0258: Baseline And Scope Change Guard
