# Modern Scope Audit

Snapshot date: 2026-06-07.

This audit separates the current Rust registry into modern v1 surfaces,
compatibility-carried legacy surfaces, and intentionally unsupported legacy
surfaces.

It does not remove code. It records which currently present hbci4java-compatible
surfaces should be treated as lower-relevance compatibility debt before public
API stabilization.

## Evidence Sources

Local evidence:

- `src/gv/mod.rs`, especially `PINTAN_JOB_NAMES`;
- `docs/architecture/job-coverage.md`;
- `docs/architecture/result-coverage.md`;
- `scripts/audit-modern-scope.sh`;
- `scripts/audit-job-coverage.sh`;
- `scripts/audit-result-coverage.sh`.

External evidence:

- Deutsche Bundesbank records the SEPA migration end date for national credit
  transfer and direct-debit schemes as 1 February 2014, with German transition
  allowances ending on 1 February 2016.
- Deutsche Bundesbank states that since 1 February 2016 credit transfers may
  only be accepted in SEPA format with IBAN.
- European Payments Council explains that the local instrument code `COR1` is
  no longer relevant from 20 November 2016; only `CORE` can be used for new SDD
  Core collections after that date.
- The Council of the EU adopted the Instant Payments Regulation in 2024,
  requiring euro instant payments to be made available by payment service
  providers that offer standard euro credit transfers.
- The ECB describes SEPA Instant Credit Transfer as the EU instant-payment
  scheme, launched in 2017 and targeted by the 2024 regulation.
- FinTS still documents signature-card support as part of the standard, but also
  documents two-step TAN procedures such as chipTAN and mobileTAN.
- Sparkasse describes classic HBCI chipcard banking as secure but laborious and
  says it is not recommended today.
- Sparkasse and ING document current standing-order workflows through online or
  app banking using recipient IBAN plus TAN/app approval, which supports the
  SEPA standing-order surface rather than the old national `Dauer*` jobs.
- ING documents FinTS/HBCI with PIN/password and explicitly says it does not
  offer HBCI with chipcard; its current FinTS payment support is bank-specific
  and narrower than its account-information support.
- Deutsche Bank documents foreign and foreign-currency payments as a current
  online-banking need, but its corporate ISO 20022 guidance treats DTAZV and
  other non-XML or old XML payment formats as legacy formats to migrate away
  from by November 2026.
- Swift records the cross-border payment-instruction migration toward ISO 20022
  and the November 2025 end of the MT/ISO 20022 coexistence period for payment
  instructions.

## Modern V1 Surface

These areas remain the recommended surface for new work:

| Area | Current examples | Why it stays strategic |
| --- | --- | --- |
| Account and status information | `SaldoReq`, `SaldoReqAll`, `AccInfo`, `Status`, `SEPAInfo`, `TANMediaList`, `TANList` | Common FinTS account-information and SCA metadata workflows. |
| Statements | `KUmsAllCamt`, `KUmsZeitSEPA`, `KUmsAll`, `KUmsNew`, `Kontoauszug`, `KontoauszugPdf` | CAMT is the modern direction; MT940/MT942 and statement PDFs remain common compatibility formats in financial software. |
| SEPA credit transfers | `UebSEPA`, `MultiUebSEPA`, `TermUebSEPA`, `TermMultiUebSEPA`, `UmbSEPA`, `InstUebSEPA` | SEPA replaced national transfer schemes; instant transfer is increasingly relevant under the Instant Payments Regulation. |
| SEPA direct debits | `LastSEPA`, `LastB2BSEPA`, `MultiLastSEPA`, `MultiLastB2BSEPA`, `DauerLastSEPANew`, `DauerLastSEPAList` | SEPA direct debit remains current, including B2B. |
| Standing orders | `DauerSEPANew`, `DauerSEPAEdit`, `DauerSEPADel`, `DauerSEPAList`, `TermUebSEPA*` | SEPA-based recurring and scheduled payments are the current payment rail. |
| Strong customer authentication | PinTAN, QR, matrix, flicker/chipTAN challenge data, photoTAN callback data, decoupled polling | These are modern SCA mechanisms within the PinTAN/HBCI-Plus runtime. |
| Verification and bank-specific query jobs | `VoP`, `VoPAuth`, `InfoList`, `InfoOrder`, `CardList`, `FestCondList`, `FestList`, `WPDepotList`, `WPDepotUms` | These are not necessarily legacy; availability is bank/BPD-dependent. |

## Compatibility-Carried Legacy Surface

These job names are currently present because the port was originally
package-near and original-near to hbci4java. They should not be used as examples
for new integrations and should be reviewed before public API stabilization.

A detailed per-job current-relevance audit lives in
`docs/reference/legacy-job-relevance-audit.md`.

| Category | Current job names | Reason for lower relevance |
| --- | --- | --- |
| Classic foreign transfer | `UebForeign` | Foreign and foreign-currency payments are current, but this HKAOM/UebForeign2 job is an old FinTS shape rather than a modern ISO 20022-oriented cross-border payment surface. |

## Intentionally Unsupported Legacy Surface

These surfaces remain out of scope, not merely deferred:

- HBCI signature-card runtime support;
- PCSC, CTAPI, DDV, native card-reader integration;
- RDH, RAH, and RSA key-file live support;
- Java passport import/export;
- `LastCOR1SEPA` and `MultiLastCOR1SEPA`;
- `MultiUeb` and `MultiLast`;
- `Last` and `StornoLast`;
- `Donation`, `Ueb`, `UebBZU`, `UebEil`, `UebGar`, and `Umb`;
- `DauerNew`, `DauerEdit`, `DauerDel`, and `DauerList`;
- `TermUeb`, `TermUebEdit`, `TermUebDel`, and `TermUebList`;
- arbitrary dynamic lowlevel jobs through public `newLowlevelJob(...)`;
- `GVTemplate`;
- `WPStammData` / lowlevel `WPStammList`.

## Follow-Up Rules

Before publishing a stable public API, each compatibility-carried legacy
category should get an explicit decision:

- keep as compatibility surface;
- hide behind an opt-in feature;
- remove from the public registry while preserving internal protocol tests;
- or document as unsupported and update the job/result audit expectations.

Any such change requires a new ADR because it changes the meaning of the current
original-near coverage claim.

## Machine Check

The registry partition is checked by:

```sh
scripts/audit-modern-scope.sh
```

Current output:

```text
registry=47
modern=46
legacy=1
duplicates=<none>
unclassified=<none>
stale=<none>
```

The audit must be updated in the same slice as any job registry cleanup. It is a
guard against accidentally removing or reclassifying modern jobs while cleaning
legacy-carried jobs.

## Source Links

- Deutsche Bundesbank SEPA content:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content`
- Deutsche Bundesbank SEPA migration completed:
  `https://www.bundesbank.de/de/presse/pressenotizen/sepa-umstellung-erfolgreich-abgeschlossen-664750`
- Deutsche Bundesbank SEPA credit transfer:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content/sepa-credit-transfer-626664`
- Deutsche Bundesbank SEPA direct debit:
  `https://www.bundesbank.de/en/tasks/payment-systems/services/sepa/content/sepa-direct-debit-626654`
- Sparkasse standing orders:
  `https://www.sparkasse.de/pk/produkte/konten-und-karten/banking/online-services/dauerauftrag.html`
- Sparkasse transfers:
  `https://www.sparkasse.de/pk/produkte/konten-und-karten/banking/ueberweisung.html`
- ING standing orders:
  `https://www.ing.de/hilfe/zahlungsverkehr/ueberweisen/dauerauftraege/`
- European Payments Council COR1 note:
  `https://www.europeanpaymentscouncil.eu/document-library/guidance-documents/explanatory-note-use-cor1-and-smnda-sdd-r-transactions`
- Swift ISO 20022 for financial institutions:
  `https://www.swift.com/standards/iso-20022/iso-20022-programme`
- Deutsche Bank ISO 20022 migration:
  `https://www.deutsche-bank.de/ub/unsere-loesungen/konto-zahlungsverkehr/iso-20022.html`
- Deutsche Bank foreign and foreign-currency payment FAQ:
  `https://www.deutsche-bank.de/pk/service-und-kontakt/services/fragen-antworten/konto-und-debitkarten/wie-ueberweise-ich-ausserhalb-europas-oder-in-einer-fremdwaehrung.html`
- Council of the EU instant payments regulation:
  `https://www.consilium.europa.eu/en/press/press-releases/2024/02/26/council-adopts-regulation-on-instant-payments/`
- ECB instant payments:
  `https://www.ecb.europa.eu/paym/retail/instant_payments/html/index.en.html`
- FinTS specification page:
  `https://www.fints.org/de/spezifikation`
- Sparkasse HBCI:
  `https://www.sparkasse.de/pk/ratgeber/finanzglossar/hbci-verfahren.html`
- ING FinTS/HBCI:
  `https://www.ing.de/hilfe/log-in/fints/`
