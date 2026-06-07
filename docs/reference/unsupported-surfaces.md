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

This does not exclude chipTAN, QR-TAN, photoTAN, decoupled app approval, or TAN
media metadata. Those are PinTAN/SCA mechanisms and remain in scope. The
unsupported chipcard boundary refers to HBCI signature-card runtime support,
including PCSC/CTAPI/DDV-style access and card-reader UX.

The v1 port also does not import, export, or interpret Java passport files. The
only persisted passport format is the Rust-native PinTAN envelope documented in
`docs/reference/passport-storage-security.md`.

The current market/source evidence for this boundary is recorded in
`docs/reference/security-media-scope.md`.

Some historical payment jobs are still present for original-near compatibility.
They are tracked separately in `docs/reference/modern-scope-audit.md`; their
presence does not change the unsupported security-media boundary.

The guarded cleanup path for those compatibility-carried jobs is recorded in
`docs/architecture/legacy-cleanup-plan.md`.

## Removed Legacy Public Jobs

The following hbci4java high-level jobs are deliberately absent from the v1
public registry:

- `LastCOR1SEPA`;
- `MultiLastCOR1SEPA`;
- `MultiLast`;
- `MultiUeb`;
- `Last`;
- `StornoLast`.

`LastCOR1SEPA` and `MultiLastCOR1SEPA` were compatibility-carried SEPA `COR1`
direct-debit variants. EPC guidance states that `COR1` is no longer relevant
for new SDD Core collections from 20 November 2016. Use the modern CORE or B2B
SEPA direct-debit jobs instead: `LastSEPA`, `MultiLastSEPA`, `LastB2BSEPA`, or
`MultiLastB2BSEPA`.

`MultiLast` and `MultiUeb` were compatibility-carried DTAUS bulk jobs over the
old national payment rails. Use the modern SEPA bulk jobs instead:
`MultiUebSEPA`, `MultiLastSEPA`, or `MultiLastB2BSEPA`.

`Last` and `StornoLast` were compatibility-carried classic national
direct-debit jobs. Use `LastSEPA`, `LastB2BSEPA`, `MultiLastSEPA`, or
`MultiLastB2BSEPA` for current direct-debit initiation. Any future
direct-debit dispute or return workflow needs a new scoped decision rather than
the old `LastObjection2` job.

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

The current job coverage audit therefore allows exactly seven missing upstream
`GV*.java` classes: `Last`, `LastCOR1SEPA`, `MultiLast`,
`MultiLastCOR1SEPA`, `MultiUeb`, `StornoLast`, and `Template`.

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

These release and packaging boundaries remain explicit guardrails, not hidden
implementation support:

- final release-candidate offline gate output is recorded by
  `scripts/run-release-candidate-checks.sh --package`;
- upstream header inconsistencies must be rechecked again if the baseline or
  copied artifacts change before publishing;
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
- `docs/architecture/legacy-cleanup-plan.md`
- `docs/reference/passport-storage-security.md`
- `docs/reference/modern-scope-audit.md`
- `docs/reference/security-media-scope.md`
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
- ADR 0261: Security Media Scope Evidence
- ADR 0262: Non-Legacy Publication Scope
- ADR 0265: Remove COR1 Public Jobs
- ADR 0266: Remove DTAUS Bulk Public Jobs
- ADR 0267: Remove Classic Direct Debit Public Jobs
