# Legacy Cleanup Plan

Snapshot date: 2026-06-07.

This plan explains how to remove, hide, or feature-gate compatibility-carried
legacy jobs without breaking the modern v1 PinTAN/HBCI-Plus surface.

The plan is intentionally conservative. It does not delete code by itself. It
creates a repeatable way to prove that future cleanup slices affect only the
legacy-carried surface documented in `docs/reference/modern-scope-audit.md`.

## Non-Negotiable Invariants

Cleanup slices must not break:

- PinTAN/HBCI-Plus dialog init, execution, close, TAN, SCA, QR, photoTAN, and
  decoupled polling behavior;
- passport storage and runtime PIN/TAN handling;
- account and status jobs such as `SaldoReq`, `SaldoReqAll`, `AccInfo`,
  `Status`, `SEPAInfo`, `TANMediaList`, and `TANList`;
- statement jobs such as `KUmsAllCamt`, `KUmsZeitSEPA`, `KUmsAll`, `KUmsNew`,
  `Kontoauszug`, and `KontoauszugPdf`;
- SEPA payment jobs such as `UebSEPA`, `MultiUebSEPA`, `TermUebSEPA`,
  `TermMultiUebSEPA`, `UmbSEPA`, `InstUebSEPA`, `LastSEPA`, `LastB2BSEPA`,
  `MultiLastSEPA`, `MultiLastB2BSEPA`, `DauerSEPA*`, and `DauerLastSEPA*`;
- replay fixtures and public migration examples for modern workflows.

Allowed breakage must be explicit and limited to the compatibility-carried
legacy category being removed or hidden.

## Guard Commands

Every cleanup slice must pass:

```sh
cargo fmt --check
cargo clippy --all-targets
cargo test
cargo test -- --list
scripts/audit-modern-scope.sh
scripts/audit-job-coverage.sh
scripts/audit-result-coverage.sh
git diff --check
```

For a release candidate, run:

```sh
scripts/run-release-candidate-checks.sh --package
```

## Current Registry Partition

The machine-checkable source of truth is `scripts/audit-modern-scope.sh`.

Current snapshot:

```text
registry=47
modern=46
legacy=1
unclassified=<none>
stale=<none>
duplicates=<none>
```

The human-readable rationale lives in `docs/reference/modern-scope-audit.md`
and `docs/reference/legacy-job-relevance-audit.md`.

## Cleanup Order

Recommended cleanup order:

Completed cleanup:

| Category | Job names | Decision |
| --- | --- | --- |
| COR1 variants | `LastCOR1SEPA`, `MultiLastCOR1SEPA` | Removed from the public registry in ADR 0265; internal job implementation branches removed in ADR 0271. |
| DTAUS bulk jobs | `MultiUeb`, `MultiLast` | Removed from the public registry in ADR 0266; internal job implementation branches removed in ADR 0272. |
| Classic direct debit and objection | `Last`, `StornoLast` | Removed from the public registry in ADR 0267; internal job implementation branches removed in ADR 0273. |
| Classic domestic credit and account transfers | `Ueb`, `UebEil`, `UebGar`, `UebBZU`, `Umb`, `Donation` | Removed from the public registry in ADR 0268; internal job implementation branches removed in ADR 0275. |
| Classic scheduled transfers and standing orders | `TermUeb`, `TermUebEdit`, `TermUebDel`, `TermUebList`, `DauerNew`, `DauerEdit`, `DauerDel`, `DauerList` | Removed from the public registry in ADR 0269; shared lowlevel helpers, result normalization, and parser support remain temporarily. |

Remaining recommended cleanup order:

| Order | Category | Job names | Why this order |
| --- | --- | --- | --- |
| 1 | Classic foreign transfer | `UebForeign` | Foreign and foreign-currency payments remain current, so this old HKAOM/UebForeign2 job needs a separate product-boundary ADR before removal. |

## Slice Template

Each cleanup slice should:

- add or reference an ADR naming the exact job category;
- update `scripts/audit-modern-scope.sh`;
- update `docs/reference/modern-scope-audit.md`;
- update `docs/reference/unsupported-surfaces.md` when a public job becomes
  unsupported;
- update `docs/reference/java-to-rust-mapping.md` or migration examples if the
  public migration story changes;
- keep modern tests intact and remove, feature-gate, or reclassify only the
  affected legacy tests;
- run the guard commands above.

## Preferred Implementation Shape

Prefer this sequence for each category:

1. Remove the legacy job name from the public registry or move it behind an
   explicit opt-in seam.
2. Keep shared lowlevel helpers temporarily if modern jobs still use adjacent
   code paths.
3. Convert any still-useful protocol examples into internal fixtures that do
   not imply public support.
4. Delete now-unreachable helper code only after `cargo test -- --list` proves
   no remaining tests reference the old job names.

This keeps locality: public job availability changes first, then implementation
deletion follows with evidence.
