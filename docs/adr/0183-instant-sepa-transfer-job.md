# ADR 0183: Instant SEPA Transfer Job

## Status

Accepted

## Context

`GVInstUebSEPA` is hbci4java's high-level job for submitting a SEPA instant
credit transfer. It extends `GVUebSEPA`, keeps `getPainJobName()` as
`UebSEPA`, and therefore reuses the same `pain.001.001.02` default PAIN
descriptor and PAIN.001 generation family.

The pinned protocol resources define `InstUebSEPA1` / `HKIPZ` and
`InstUebSEPARes1` / `HIIPZ` in `hbci-300.xml`. The request segment contains
the source account, PAIN descriptor, and PAIN binary block. The response
contains optional `orderid`, `ccode`, and `orderstatus` fields, exposed by
hbci4java through `GVRInstUebSEPA`.

`GVInstUebSEPA` also adds the same single-order `batchbook = 0` dummy
constraint as `GVUebSEPA`.

## Decision

Port `InstUebSEPA` as the next single-transfer PinTAN slice:

- expose original-near constraints for `InstUebSEPA1`, matching `UebSEPA`
  source-account fields, `_sepadescriptor`, `_sepapain`, SEPA dummy
  parameters, indexed destination / amount / usage fields, `batchbook`,
  `sepaid`, `pmtinfid`, `endtoendid`, and `purposecode`;
- render `InstUebSEPA1` as `HKIPZ`, preserving the segment order account,
  descriptor, and PAIN binary block;
- use the existing ADR 0177 `pain.001.001.02` single-transfer generator when
  `_sepapain` is absent;
- add a typed `GvrInstUebSepa` result containing `order_id`,
  `order_status`, and `cancellation_code`;
- preserve raw content result data from `InstUebSEPARes1`.

Do not port BPD parameter handling for `InstUebSEPAPar1`, newer PAIN.001
versions, instant-payment status semantics, cancellation-code interpretation,
or live-bank timing behavior in this slice.

## Consequences

The Rust port can now replay-test instant SEPA transfer submission with the
same PAIN generator used by `UebSEPA`, while still exposing the instant-specific
bank response fields.

The implementation remains original-near and intentionally duplicates the
single-transfer constraints for the new lowlevel segment instead of introducing
a shared transfer-job abstraction before more variants are ported.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVInstUebSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVUebSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRInstUebSEPA.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0177-pain001-transfer-generator.md`
- `docs/adr/0178-ueb-sepa-job.md`
