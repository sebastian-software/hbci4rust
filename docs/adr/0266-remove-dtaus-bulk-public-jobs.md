# 0266 Remove DTAUS Bulk Jobs From Public Registry

Date: 2026-06-07

## Status

Accepted.

## Context

ADR 0264 identified `MultiUeb` and `MultiLast` as DTAUS bulk-payment jobs among
the original 21 compatibility-carried legacy job candidates. They differ from
the SEPA bulk jobs because they accept an already serialized DTAUS payload as
binary FinTS data:

- `MultiUeb` maps to `SammelUeb6` / `HKSUB`;
- `MultiLast` maps to `SammelLast6` / `HKSLA`.

The modern v1 alternatives already exist in the public registry:

- `MultiUebSEPA` for SEPA bulk credit transfers;
- `MultiLastSEPA` for SEPA Core bulk direct debits;
- `MultiLastB2BSEPA` for SEPA B2B bulk direct debits.

The external evidence used for ADR 0264 and the scope audit remains the same:
German national credit-transfer and direct-debit procedures were migrated to
SEPA, with the normal euro-area deadline in 2014 and German transition
allowances ending in 2016. Keeping public jobs that require caller-provided
DTAUS payloads would make the crate look broader and older than the intended
non-legacy FinTS PinTAN/HBCI-Plus v1 surface.

## Decision

Remove `MultiUeb` and `MultiLast` from `PINTAN_JOB_NAMES` and from the
compatibility-carried legacy partition in `scripts/audit-modern-scope.sh`.

Callers using `HbciHandler::new_job("MultiUeb")` or
`HbciHandler::new_job("MultiLast")` now receive an unsupported or out-of-scope
error.

Update `scripts/audit-job-coverage.sh` so the intentional missing upstream
`GV*.java` jobs are:

- `LastCOR1SEPA`;
- `MultiLast`;
- `MultiLastCOR1SEPA`;
- `MultiUeb`;
- `Template`.

Keep the lower-level DTAUS constraint and rendering helpers temporarily. They
are now unreachable through the public registry, but deleting them should be a
separate mechanical cleanup after the modern SEPA bulk regression tests and job
coverage audits stay green.

## Consequences

The public job registry shrinks from 65 to 63 names. The modern v1 partition
stays at 46 names, while the compatibility-carried legacy partition shrinks from
19 to 17 names.

The affected old replay tests are removed because they would imply public
support. New tests assert that `MultiUeb` and `MultiLast` are out of scope, and
the existing `MultiUebSEPA`, `MultiLastSEPA`, and `MultiLastB2BSEPA` tests
remain the regression surface for current bulk payments.

This ADR supersedes the public-support parts of ADR 0229 and ADR 0230. Those
ADRs remain useful as historical port notes for the original-near DTAUS
implementation slices.

## References

- `docs/adr/0229-multi-ueb-job.md`
- `docs/adr/0230-multi-last-job.md`
- `docs/adr/0264-legacy-job-current-relevance.md`
- `docs/architecture/legacy-cleanup-plan.md`
- `docs/reference/modern-scope-audit.md`
- `docs/reference/unsupported-surfaces.md`
- Deutsche Bundesbank SEPA credit transfer:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content/sepa-credit-transfer-626664`
- Deutsche Bundesbank SEPA direct debit:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content/sepa-direct-debit-626654`
- European Payments Council SEPA timeline:
  `https://www.europeanpaymentscouncil.eu/about-sepa/sepa-timeline`
