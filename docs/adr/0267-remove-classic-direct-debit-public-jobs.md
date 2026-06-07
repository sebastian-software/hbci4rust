# 0267 Remove Classic Direct Debit Jobs From Public Registry

Date: 2026-06-07

## Status

Accepted.

## Context

ADR 0264 identified `Last` and `StornoLast` as classic national direct-debit
jobs among the original 21 compatibility-carried legacy job candidates.

The local Rust field shape confirms that these jobs are tied to the old
domestic payment rail:

- `Last` maps to `Last5` / `HKLAS` and uses account-number/bank-sort-code
  fields for creditor and debtor accounts;
- `StornoLast` maps to `LastObjection2` / `HKLSW` and uses the same national
  account identity shape for a direct-debit objection.

The modern v1 direct-debit jobs already exist in the public registry:

- `LastSEPA` for SEPA Core direct debits;
- `LastB2BSEPA` for SEPA B2B direct debits;
- `MultiLastSEPA` and `MultiLastB2BSEPA` for bulk direct debits.

German national direct-debit procedures were migrated to SEPA. The Bundesbank
records the SEPA migration end date for national credit-transfer and
direct-debit schemes as 1 February 2014, with German ELV transition allowances
ending on 1 February 2016. Keeping `Last` and `StornoLast` in the public
registry would imply support for a historic national direct-debit rail rather
than a non-legacy FinTS PinTAN/HBCI-Plus surface.

Direct-debit dispute or return workflows remain a real banking need, but this
ADR does not preserve `StornoLast` as a placeholder for that need. A future
dispute/return surface needs its own product-boundary ADR and current bank or
protocol evidence instead of relying on the old `LastObjection2` job.

## Decision

Remove `Last` and `StornoLast` from `PINTAN_JOB_NAMES` and from the
compatibility-carried legacy partition in `scripts/audit-modern-scope.sh`.

Callers using `HbciHandler::new_job("Last")` or
`HbciHandler::new_job("StornoLast")` now receive an unsupported or out-of-scope
error.

Update `scripts/audit-job-coverage.sh` so the intentional missing upstream
`GV*.java` jobs are:

- `Last`;
- `LastCOR1SEPA`;
- `MultiLast`;
- `MultiLastCOR1SEPA`;
- `MultiUeb`;
- `StornoLast`;
- `Template`.

Keep the lower-level `Last5` and `LastObjection2` constraint/rendering helpers
temporarily. They are now unreachable through the public registry, but deleting
them should be a separate mechanical cleanup after the modern SEPA direct-debit
regression tests and job coverage audits stay green.

## Consequences

The public job registry shrinks from 63 to 61 names. The modern v1 partition
stays at 46 names, while the compatibility-carried legacy partition shrinks from
17 to 15 names.

The affected old replay tests are removed because they would imply public
support. New tests assert that `Last` and `StornoLast` are out of scope, and the
existing `LastSEPA`, `LastB2BSEPA`, `MultiLastSEPA`, and `MultiLastB2BSEPA`
tests remain the regression surface for current direct debits.

This ADR supersedes the public-support parts of ADR 0228 and ADR 0231. Those
ADRs remain useful as historical port notes for the original-near classic
direct-debit implementation slices.

## References

- `docs/adr/0228-last-job.md`
- `docs/adr/0231-storno-last-job.md`
- `docs/adr/0264-legacy-job-current-relevance.md`
- `docs/architecture/legacy-cleanup-plan.md`
- `docs/reference/modern-scope-audit.md`
- `docs/reference/unsupported-surfaces.md`
- Deutsche Bundesbank SEPA content:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content`
- Deutsche Bundesbank SEPA direct debit:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content/sepa-direct-debit-626654`
- Deutsche Bundesbank SEPA migration not yet complete in February:
  `https://www.bundesbank.de/en/press/press-releases/sepa-migration-not-yet-complete-in-february-670576`
