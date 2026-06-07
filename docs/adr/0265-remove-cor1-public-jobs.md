# ADR 0265: Remove COR1 Public Jobs

## Status

Accepted

## Context

ADR 0263 created a guarded cleanup path for compatibility-carried legacy jobs.
ADR 0264 identified `LastCOR1SEPA` and `MultiLastCOR1SEPA` as the smallest and
clearest first cleanup slice:

- they are SEPA jobs, so they are not covered by the simple "non-SEPA is
  legacy" shortcut;
- the European Payments Council says the `COR1` local instrument is no longer
  relevant for new SDD Core collections from 20 November 2016;
- modern CORE and B2B direct debit jobs remain present as `LastSEPA`,
  `MultiLastSEPA`, `LastB2BSEPA`, and `MultiLastB2BSEPA`.

## Decision

Remove `LastCOR1SEPA` and `MultiLastCOR1SEPA` from the public static
`PINTAN_JOB_NAMES` registry.

Keep the lower-level COR1 rendering and parsing code temporarily. It shares the
SEPA direct-debit implementation with CORE and B2B jobs, and deleting it in the
same slice would widen the change from a public-surface cleanup into a shared
implementation cleanup.

Update the source-surface audits so the intentional missing upstream `GV*.java`
jobs are now:

- `LastCOR1SEPA`;
- `MultiLastCOR1SEPA`;
- `Template`.

## Consequences

Callers using `HbciHandler::new_job("LastCOR1SEPA")` or
`HbciHandler::new_job("MultiLastCOR1SEPA")` now receive an unsupported or
out-of-scope error.

The public registry shrinks from 67 to 65 jobs. The modern registry count stays
at 46 and the compatibility-carried legacy count shrinks from 21 to 19.

Historical ADRs 0212 and 0218 still explain how the original-near COR1 code was
ported. This ADR supersedes them only for public v1 job availability.

## Links

- `src/gv/mod.rs`
- `scripts/audit-modern-scope.sh`
- `scripts/audit-job-coverage.sh`
- `docs/reference/modern-scope-audit.md`
- `docs/reference/legacy-job-relevance-audit.md`
- `docs/reference/unsupported-surfaces.md`
- ADR 0263: Guard Legacy Cleanup
- ADR 0264: Legacy Job Current Relevance
