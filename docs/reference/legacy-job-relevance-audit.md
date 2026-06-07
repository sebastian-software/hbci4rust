# Legacy Job Relevance Audit

Snapshot date: 2026-06-07.

This audit checks the 21 compatibility-carried legacy job candidates from
`scripts/audit-modern-scope.sh` for current relevance before any removal or
feature-gating work.

## Finding

The short statement "non-SEPA is legacy" is mostly correct for the classic
German domestic payment jobs, but it is not precise enough for the whole set.

- 18 candidates are classic national or DTAUS jobs. They use
  account-number/bank-sort-code, classic FinTS segments, DTAUS-style usage
  fields, or already serialized DTAUS payloads. Their modern v1 counterparts are
  SEPA jobs with IBAN/BIC and PAIN payloads.
- 2 candidates are SEPA `COR1` variants. They are not non-SEPA, but the `COR1`
  local instrument is no longer relevant for new SDD Core collections.
- 1 candidate, `UebForeign`, represents a still-current user need: foreign or
  foreign-currency payments. The current port is nevertheless legacy-carried
  because it is the old HKAOM/UebForeign2 job shape, not a modern ISO 20022
  cross-border payment surface.

## Local Evidence

The current Rust port keeps the original-near field shapes:

- classic transfer, scheduled-transfer, and standing-order jobs expose
  `*.KIK.blz`, `*.number`, `*.subnumber`, `name2`, `key`, and repeated DTAUS
  `usage` lines;
- `MultiUeb` and `MultiLast` accept an already serialized DTAUS payload as
  binary FinTS data;
- `UebForeign` maps to `HKAOM` / `UebForeign2`, uses a national source account,
  and exposes receiver bank/name/currency/cost bearer fields, but no modern
  structured address or ISO 20022 payment-initiation payload.

## Candidate Matrix

| Job | Current relevance finding | Modern or strategic alternative | Cleanup stance |
| --- | --- | --- | --- |
| `DauerDel` | Classic standing-order deletion with national account fields and stored classic order data. | `DauerSEPADel` | Legacy; remove or hide with the classic standing-order slice. |
| `DauerEdit` | Classic standing-order edit with national account fields and DTAUS-style usage. | `DauerSEPAEdit` | Legacy; remove or hide with the classic standing-order slice. |
| `DauerList` | Classic standing-order list query keyed by national account identity. | `DauerSEPAList` | Legacy; remove or hide with the classic standing-order slice. |
| `DauerNew` | Classic standing-order creation with national account fields and DTAUS-style usage. | `DauerSEPANew` | Legacy; remove or hide with the classic standing-order slice. |
| `Donation` | hbci4java alias over classic `Ueb5` with donation-specific DTAUS usage fields. | `UebSEPA` plus caller-side remittance purpose handling | Legacy; remove or hide with classic domestic transfer aliases. |
| `Last` | Classic domestic direct-debit submission predating the SEPA direct-debit rail. | `LastSEPA`, `LastB2BSEPA` | Legacy; remove or hide with the classic direct-debit slice. |
| `LastCOR1SEPA` | SEPA job, but `COR1` is obsolete for new SDD Core collections. | `LastSEPA` with `CORE` | Legacy; clean first because the external evidence is strong and the surface is small. |
| `MultiLast` | Classic domestic bulk direct debit with serialized DTAUS payload. | `MultiLastSEPA`, `MultiLastB2BSEPA` | Legacy; remove or hide with DTAUS bulk jobs. |
| `MultiLastCOR1SEPA` | SEPA bulk job, but `COR1` is obsolete for new SDD Core collections. | `MultiLastSEPA` with `CORE` | Legacy; clean first with `LastCOR1SEPA`. |
| `MultiUeb` | Classic domestic bulk transfer with serialized DTAUS payload. | `MultiUebSEPA` | Legacy; remove or hide with DTAUS bulk jobs. |
| `StornoLast` | Classic domestic direct-debit objection path, tied to the pre-SEPA direct-debit surface. | Explicit future dispute/return workflow if needed | Legacy; remove or hide with the classic direct-debit slice. |
| `TermUeb` | Classic scheduled transfer with national source and destination account fields. | `TermUebSEPA`, `TermMultiUebSEPA` | Legacy; remove or hide with classic scheduled transfers. |
| `TermUebDel` | Classic scheduled-transfer deletion using stored classic order data. | `TermUebSEPADel` | Legacy; remove or hide with classic scheduled transfers. |
| `TermUebEdit` | Classic scheduled-transfer edit with national account fields and DTAUS-style usage. | `TermUebSEPAEdit` | Legacy; remove or hide with classic scheduled transfers. |
| `TermUebList` | Classic scheduled-transfer list query keyed by national account identity. | `TermUebSEPAList` | Legacy; remove or hide with classic scheduled transfers. |
| `Ueb` | Classic domestic credit transfer using account-number/bank-sort-code and DTAUS-style usage. | `UebSEPA`, `InstUebSEPA` | Legacy; remove or hide with classic domestic transfers. |
| `UebBZU` | Classic domestic transfer variant over `Ueb5` with special reference/check-digit data. | `UebSEPA` plus caller-side remittance handling when applicable | Legacy; remove or hide with classic domestic transfer variants. |
| `UebEil` | Classic urgent domestic transfer segment. | `InstUebSEPA` or bank-specific modern urgent-payment support | Legacy; remove or hide after instant/SEPA transfer tests stay green. |
| `UebForeign` | Foreign and foreign-currency payments are current, but this HKAOM/UebForeign2 job is an old FinTS shape and lacks modern ISO 20022 structured-data support. | A future explicitly scoped foreign-payment surface, likely ISO 20022/EBICS-oriented rather than this original-near job | Legacy-carried but product-sensitive; defer cleanup to a dedicated ADR. |
| `UebGar` | Classic domestic guaranteed-transfer variant. | `UebSEPA`, `InstUebSEPA`, or bank-specific modern urgent-payment support | Legacy; remove or hide with classic domestic transfer variants. |
| `Umb` | Classic domestic account transfer using national account fields. | `UmbSEPA` | Legacy; remove or hide with classic domestic account-transfer jobs. |

## Cleanup Implications

Recommended cleanup order remains conservative, with one correction: `UebForeign`
is not a domestic transfer and should be handled last or behind a dedicated
product-boundary ADR.

1. `LastCOR1SEPA`, `MultiLastCOR1SEPA`
2. `MultiUeb`, `MultiLast`
3. `Last`, `StornoLast`
4. `Ueb`, `UebEil`, `UebGar`, `UebBZU`, `Umb`, `Donation`
5. `TermUeb`, `TermUebEdit`, `TermUebDel`, `TermUebList`, `DauerNew`,
   `DauerEdit`, `DauerDel`, `DauerList`
6. `UebForeign`

## Source Links

- Deutsche Bundesbank SEPA content:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content`
- Deutsche Bundesbank SEPA credit transfer:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content/sepa-credit-transfer-626664`
- Deutsche Bundesbank SEPA direct debit:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content/sepa-direct-debit-626654`
- Deutsche Bundesbank SEPA migration completed:
  `https://www.bundesbank.de/de/presse/pressenotizen/sepa-umstellung-erfolgreich-abgeschlossen-664750`
- European Payments Council COR1 note:
  `https://www.europeanpaymentscouncil.eu/document-library/guidance-documents/explanatory-note-use-cor1-and-smnda-sdd-r-transactions`
- Swift ISO 20022 for financial institutions:
  `https://www.swift.com/standards/iso-20022/iso-20022-programme`
- Deutsche Bank ISO 20022 migration:
  `https://www.deutsche-bank.de/ub/unsere-loesungen/konto-zahlungsverkehr/iso-20022.html`
- Deutsche Bank foreign and foreign-currency payment FAQ:
  `https://www.deutsche-bank.de/pk/service-und-kontakt/services/fragen-antworten/konto-und-debitkarten/wie-ueberweise-ich-ausserhalb-europas-oder-in-einer-fremdwaehrung.html`
