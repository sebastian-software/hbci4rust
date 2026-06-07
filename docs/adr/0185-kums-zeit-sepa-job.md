# ADR 0185: SEPA Account Statement Job

## Status

Accepted

## Context

`GVKUmsZeitSEPA` is hbci4java's high-level job for retrieving booked and
unbooked statements for a SEPA account. It extends `GVKUmsAll` and therefore
uses the same `GVRKUms` result type and MT940 / MT942 extraction behavior.

The pinned protocol resources define `KUmsZeitSEPA7` / `HKKAZ` and
`KUmsZeitSEPARes7` / `HIKAZ` in `hbci-plus.xml`. The request segment contains
an international account (`KTVInt`), `allaccounts`, optional `startdate`,
`enddate`, `maxentries`, and `offset`. The response segment contains booked
and optional not-booked binary statement data, matching the existing
`KUmsAll` result family.

`GVKUmsZeitSEPA` adds explicit constraints for `my.bic`, `my.iban`,
`startdate`, `enddate`, `maxentries`, `offset`, and `all`.

## Decision

Port `KUmsZeitSEPA` as the next statement retrieval PinTAN slice:

- expose original-near constraints for `KUmsZeitSEPA7`, including the SEPA
  account fields, optional date range, `maxentries`, `offset`, and
  `allaccounts` defaulting to `N`;
- render `KUmsZeitSEPA7` as `HKKAZ`, preserving the segment order account,
  all-accounts flag, date range, entry limit, and offset;
- reuse the existing `GvrKUms` / `HbciJobResultData::KUms` result shape and
  MT940 / MT942 raw data collection;
- preserve raw content result data from `KUmsZeitSEPARes7`.

Do not port new CAMT formats, deeper statement-line parsing, BPD parameter
handling for `KUmsZeitSEPAPar7`, or non-SEPA statement variants in this slice.

## Consequences

The Rust port can now replay-test the dedicated SEPA account statement segment
without changing the existing MT940 / MT942 parser boundary.

The implementation intentionally keeps `KUmsZeitSEPA` as a separate lowlevel
segment mapping instead of folding it into `KUmsAll`, because hbci4java exposes
it as a distinct job name.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVKUmsZeitSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVKUmsAll.java`
- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `docs/adr/0018-transaction-statement-parsing.md`
