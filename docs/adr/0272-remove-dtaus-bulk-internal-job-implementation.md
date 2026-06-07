# ADR 0272: Remove DTAUS Bulk Internal Job Implementation

Date: 2026-06-07

## Status

Accepted.

## Context

ADR 0266 removed `MultiUeb` and `MultiLast` from the public v1 job registry
because they are DTAUS bulk jobs over the old German national payment rails.
Current v1 bulk-payment support is SEPA-based.

ADR 0270 now enforces the public registry boundary at queueing and rendering
time. After that guard, the remaining `MultiUeb` and `MultiLast` constraints,
binary DTAUS data conversion, renderer branches, orderhash metadata, and
classic bulk renderer were dead implementation for unsupported jobs.

## Decision

Delete the internal job implementation branches for:

- `MultiUeb`
- `MultiLast`

Keep the modern SEPA bulk jobs:

- `MultiUebSEPA`
- `MultiLastSEPA`
- `MultiLastB2BSEPA`

## Consequences

- DTAUS bulk jobs remain visible only as intentional unsupported audit gaps and
  regression tests.
- The handler no longer contains `SammelUeb6` or `SammelLast6` render/orderhash
  paths.
- SEPA bulk transfer and direct-debit behavior remains unchanged.
- Result coverage remains unchanged.

## References

- `docs/adr/0266-remove-dtaus-bulk-public-jobs.md`
- `docs/adr/0270-enforce-public-job-registry-boundary.md`
- `docs/architecture/legacy-cleanup-plan.md`
- `src/gv/mod.rs`
- `src/manager/handler.rs`
