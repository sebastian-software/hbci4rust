# ADR 0169: SEPA Info Job

## Status

Accepted

## Context

The v1 scope keeps the port close to hbci4java while focusing on PinTAN/HBCI-Plus. hbci4java
exposes `SEPAInfo` through `GVSEPAInfo`, rendering `HKSPA` and parsing `HISPA`. Unlike jobs such as
`SaldoReq` or `TANMediaList`, upstream does not define a specialized `GV_Result` type for this job;
it uses the generic `HBCIJobResultImpl` and primarily applies a side effect to the passport UPD:
accounts that already exist in UPD are enriched with BIC/IBAN returned by `HISPA`.

The current Rust port already imports UPD accounts during dialog initialization and preserves
missing SEPA fields across later UPD imports. It does not yet render `SEPAInfo`, extract `HISPA`
content, or apply the hbci4java-style BIC/IBAN enrichment.

## Decision

Port `SEPAInfo` as an original-near high-level job:

- keep the public job name `SEPAInfo`;
- expose no Rust-specialized `HbciJobResultData` variant for now, matching hbci4java's generic
  result class;
- render `SEPAInfo1` (`HKSPA` version 1) as an empty account query first;
- store parsed `SEPAInfoRes1` (`HISPA` version 1) fields in `HbciJobResult.result_data`;
- update existing PinTAN passport accounts when a `HISPA` account has `sepa != "N"` and matches an
  existing account by country, bank code, and account number;
- copy non-empty returned IBAN and BIC independently, mirroring `GVSEPAInfo.extractResults(...)`.

The dedicated hbci4java `HBCIDialogSepaInfo` workflow, fetched marker, and immediate passport save
side effects are out of this slice. They can be added once dialog orchestration grows beyond simple
queued jobs.

## Consequences

This makes the port more useful for later SEPA payment jobs because accounts can be enriched before
pain message generation needs BIC/IBAN data. It also keeps the result-model surface close to
hbci4java by not inventing a typed SEPA-info result where upstream uses the generic job result.

Open follow-up work:

- add a dedicated SEPA-info refresh helper/dialog if needed;
- port HISPAS parameter handling for SEPA job decisions such as `cannationalacc`;
- support single-account `HKSPA` queries if fixtures or live-bank behavior require them.
