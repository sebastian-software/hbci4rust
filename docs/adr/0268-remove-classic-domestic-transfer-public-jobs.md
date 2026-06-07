# 0268 Remove Classic Domestic Transfer Jobs From Public Registry

Date: 2026-06-07

## Status

Accepted.

## Context

ADR 0264 identified `Ueb`, `UebEil`, `UebGar`, `UebBZU`, `Umb`, and
`Donation` as classic domestic credit-transfer or account-transfer jobs among
the original 21 compatibility-carried legacy job candidates.

The local Rust field shape confirms that these jobs are tied to the old national
German payment rail:

- `Ueb` and `Donation` map to `Ueb5` / `HKUEB`;
- `UebEil` maps to `UebEil1` / `HKEIL`;
- `UebGar` maps to `UebGar1` / `HKGUB`;
- `UebBZU` is a public hbci4java variant over the `Ueb5` request shape with
  special `bzudata` reference and check-digit validation;
- `Umb` maps to `Umb2` / `HKUMB`.

They all expose account-number/bank-sort-code account identities, classic KTV
fields, or DTAUS-style usage fields instead of the current SEPA payment rail.

The modern v1 alternatives already exist in the public registry:

- `UebSEPA` for SEPA credit transfers;
- `InstUebSEPA` for instant SEPA credit transfers;
- `MultiUebSEPA` for SEPA bulk credit transfers;
- `TermUebSEPA` and `TermMultiUebSEPA` for scheduled SEPA transfers;
- `UmbSEPA` for SEPA account transfers.

Donation remittance remains possible through SEPA transfer purpose or remittance
data chosen by the caller. It does not need the historical `Donation` alias over
the classic `Ueb5` segment.

The external evidence used for ADR 0264 and the scope audit remains the same:
the Bundesbank records that SEPA was intended to replace national payment
schemes, that the binding end date for national credit-transfer and
direct-debit schemes was 1 February 2014, and that since 1 February 2016 German
consumer credit transfers use IBAN/SEPA instead of account number plus bank sort
code.

## Decision

Remove `Ueb`, `UebEil`, `UebGar`, `UebBZU`, `Umb`, and `Donation` from
`PINTAN_JOB_NAMES` and from the compatibility-carried legacy partition in
`scripts/audit-modern-scope.sh`.

Callers using these job names through `HbciHandler::new_job(...)` now receive an
unsupported or out-of-scope error.

Update `scripts/audit-job-coverage.sh` so the intentional missing upstream
`GV*.java` jobs are:

- `Donation`;
- `Last`;
- `LastCOR1SEPA`;
- `MultiLast`;
- `MultiLastCOR1SEPA`;
- `MultiUeb`;
- `StornoLast`;
- `Template`;
- `Ueb`;
- `UebBZU`;
- `UebEil`;
- `UebGar`;
- `Umb`.

Keep the lower-level classic transfer and account-transfer constraint/rendering
helpers temporarily. They are now unreachable through the public registry, but
deleting them should be a separate mechanical cleanup after modern SEPA
transfer, instant-transfer, account-transfer, and job coverage audits stay green.

## Consequences

The public job registry shrinks from 61 to 55 names. The modern v1 partition
stays at 46 names, while the compatibility-carried legacy partition shrinks from
15 to 9 names.

The affected old replay tests are removed because they would imply public
support. New tests assert that the removed job names are out of scope. The
existing `UebSEPA`, `InstUebSEPA`, `MultiUebSEPA`, `TermUebSEPA`,
`TermMultiUebSEPA`, and `UmbSEPA` tests remain the regression surface for
current domestic transfer and account-transfer workflows.

`UebForeign` is intentionally not removed by this ADR. Foreign and
foreign-currency payments remain a current banking need, and the old
HKAOM/UebForeign2 shape needs a separate product-boundary decision.

This ADR supersedes the public-support parts of ADR 0201, ADR 0202, ADR 0203,
ADR 0204, ADR 0226, and ADR 0227. Those ADRs remain useful as historical port
notes for the original-near implementation slices.

## References

- `docs/adr/0201-ueb-job.md`
- `docs/adr/0202-ueb-eil-job.md`
- `docs/adr/0203-ueb-bzu-job.md`
- `docs/adr/0204-umb-job.md`
- `docs/adr/0226-donation-job.md`
- `docs/adr/0227-ueb-gar-job.md`
- `docs/adr/0264-legacy-job-current-relevance.md`
- `docs/architecture/legacy-cleanup-plan.md`
- `docs/reference/modern-scope-audit.md`
- `docs/reference/unsupported-surfaces.md`
- Deutsche Bundesbank SEPA content:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content`
- Deutsche Bundesbank SEPA credit transfer:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content/sepa-credit-transfer-626664`
- Deutsche Bundesbank SEPA migration completed:
  `https://www.bundesbank.de/de/presse/pressenotizen/sepa-umstellung-erfolgreich-abgeschlossen-664750`
