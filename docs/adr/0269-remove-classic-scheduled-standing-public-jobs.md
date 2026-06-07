# ADR 0269: Remove Classic Scheduled And Standing Public Jobs

Date: 2026-06-07

## Status

Accepted.

## Context

ADR 0264 identified `TermUeb`, `TermUebEdit`, `TermUebDel`, `TermUebList`,
`DauerNew`, `DauerEdit`, `DauerDel`, and `DauerList` as compatibility-carried
legacy candidates.

The user need behind those jobs is still current: scheduled transfers and
standing orders remain normal online-banking workflows. The old hbci4java job
shapes are the legacy part, not the user workflow. These jobs expose national
account-number/bank-sort-code fields, DTAUS-style usage fields, and classic
stored order snapshots. Current domestic euro transfer and recurring-payment
workflows are SEPA/IBAN based.

The Rust v1 surface already contains the modern equivalents:

- scheduled SEPA transfers: `TermUebSEPA`, `TermUebSEPAEdit`,
  `TermUebSEPADel`, `TermUebSEPAList`, and `TermMultiUebSEPA`;
- SEPA standing orders: `DauerSEPANew`, `DauerSEPAEdit`, `DauerSEPADel`, and
  `DauerSEPAList`.

Current source evidence supports this distinction. Deutsche Bundesbank records
the migration from national credit-transfer/direct-debit schemes to SEPA and
the end of German transition allowances by 1 February 2016. Sparkasse and ING
document current standing-order creation/edit/delete flows through online or
app banking using recipient IBAN and TAN/app approval.

## Decision

Remove these classic scheduled-transfer and standing-order jobs from the public
PinTAN registry:

- `TermUeb`
- `TermUebEdit`
- `TermUebDel`
- `TermUebList`
- `DauerNew`
- `DauerEdit`
- `DauerDel`
- `DauerList`

`HbciHandler::new_job(...)` now reports them as unsupported or out of scope.

The job coverage audit now allows exactly these 21 upstream `GV*.java` gaps:
`DauerDel`, `DauerEdit`, `DauerList`, `DauerNew`, `Donation`, `Last`,
`LastCOR1SEPA`, `MultiLast`, `MultiLastCOR1SEPA`, `MultiUeb`, `StornoLast`,
`Template`, `TermUeb`, `TermUebDel`, `TermUebEdit`, `TermUebList`, `Ueb`,
`UebBZU`, `UebEil`, `UebGar`, and `Umb`.

Keep adjacent lowlevel helpers, constraints, result normalization, and parser
support temporarily. Those implementation details can be deleted only after a
separate mechanical cleanup proves no modern SEPA job or result path still
uses them.

## Consequences

- The public job registry shrinks from 55 to 47 names.
- The modern v1 partition remains 46 jobs.
- The compatibility-carried legacy partition shrinks from 9 jobs to 1:
  `UebForeign`.
- Classic scheduled-transfer and standing-order replay tests are removed or
  converted into unsupported-surface tests.
- Modern `TermUebSEPA*`, `TermMultiUebSEPA`, and `DauerSEPA*` tests remain in
  scope.
- `UebForeign` is intentionally not removed here because foreign and
  foreign-currency payments are current. Its old HKAOM job shape needs a
  dedicated product-boundary ADR before removal.

This ADR supersedes the public-support parts of ADRs 0198, 0199, 0200, 0205,
0206, 0207, 0208, and 0209 for these classic jobs. Those ADRs still document
historical porting behavior and implementation details.

## References

- `docs/adr/0264-legacy-job-current-relevance.md`
- `docs/reference/modern-scope-audit.md`
- `docs/reference/legacy-job-relevance-audit.md`
- `docs/architecture/legacy-cleanup-plan.md`
- `scripts/audit-modern-scope.sh`
- `scripts/audit-job-coverage.sh`
- Deutsche Bundesbank SEPA content:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content`
- Deutsche Bundesbank SEPA migration completed:
  `https://www.bundesbank.de/de/presse/pressenotizen/sepa-umstellung-erfolgreich-abgeschlossen-664750`
- Sparkasse standing orders:
  `https://www.sparkasse.de/pk/produkte/konten-und-karten/banking/online-services/dauerauftrag.html`
- Sparkasse transfers:
  `https://www.sparkasse.de/pk/produkte/konten-und-karten/banking/ueberweisung.html`
- ING standing orders:
  `https://www.ing.de/hilfe/zahlungsverkehr/ueberweisen/dauerauftraege/`
