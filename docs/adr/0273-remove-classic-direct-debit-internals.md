# ADR 0273: Remove Classic Direct Debit Internals

Date: 2026-06-07

## Status

Accepted.

## Context

ADR 0267 removed `Last` and `StornoLast` from the public v1 job registry
because they are classic national direct-debit jobs over the old German payment
rail. The supported direct-debit surface is SEPA Core and SEPA B2B.

ADR 0270 now enforces the registry boundary at queueing and rendering time, so
manually constructed unsupported jobs cannot reach executable handler paths.
After that guard, the remaining `Last` and `StornoLast` constraints, account
checks, renderers, and orderhash metadata were dead implementation for
unsupported jobs.

## Decision

Delete the internal job implementation branches for:

- `Last`
- `StornoLast`

Keep the modern SEPA direct-debit jobs:

- `LastSEPA`
- `LastB2BSEPA`
- `MultiLastSEPA`
- `MultiLastB2BSEPA`
- `DauerLastSEPANew`
- `DauerLastSEPAList`

## Consequences

- Classic national direct debit and direct-debit objection jobs remain visible
  only as intentional unsupported audit gaps and regression tests.
- The handler no longer contains `Last5` or `LastObjection2` render/orderhash
  paths.
- SEPA direct-debit behavior remains unchanged.
- Any future dispute, return, or objection workflow needs a new scoped decision
  instead of reviving the classic `StornoLast` path by default.

## References

- `docs/adr/0267-remove-classic-direct-debit-public-jobs.md`
- `docs/adr/0270-enforce-public-job-registry-boundary.md`
- `docs/architecture/legacy-cleanup-plan.md`
- `src/gv/mod.rs`
- `src/manager/handler.rs`
