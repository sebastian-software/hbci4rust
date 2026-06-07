# 0221 Port TermMultiUebSEPA As Scheduled Bulk SEPA Transfer

## Status
Accepted

## Context
`GVTermMultiUebSEPA` in hbci4java implements scheduled SEPA bulk transfers. It
extends `GVMultiUebSEPA`, uses the lowlevel job name `TermSammelUebSEPA`, and
uses `GVRTermUeb` as result class. Its constructor inherits the bulk SEPA
transfer constraints and adds:

- `date -> sepa.date`, required.

The inherited bulk transfer behavior keeps the PAIN job name `UebSEPA`, adds
`batchbook`, `Total.value`, and `Total.curr`, and computes `Total` through
`SepaUtil.sumBtgValueObject(sepaParams)` after generating the base PAIN
parameters.

In FinTS 3.0 the request segment is `TermSammelUebSEPA1` with code `HKCME`,
version 1. The response segment is `TermSammelUebSEPARes1` with code `HICME`,
version 1, and an optional `orderid`. The request segment has the same visible
shape as `SammelUebSEPA1`: source account, optional total amount, optional
`singletransfers`, PAIN descriptor, and PAIN payload. The scheduled execution
date is carried in the generated PAIN as `ReqdExctnDt`, not as a separate
segment-level date field.

On result extraction, hbci4java stores the request lowlevel parameters under
`termueb_<orderid>` when the bank returns a non-empty order id.

## Decision
Port `TermMultiUebSEPA` as the next original-near PinTAN job slice:

- expose frontend job name `TermMultiUebSEPA`;
- map inherited bulk transfer constraints to `TermSammelUebSEPA1`;
- add required `date -> TermSammelUebSEPA1.sepa.date`;
- use indexed PAIN.001 generation with the existing bulk-transfer total
  calculation;
- render `TermSammelUebSEPA1` as `HKCME` with source account, `Total.*`,
  descriptor, and PAIN.001 payload;
- parse `TermSammelUebSEPARes1.orderid` into the existing `TermUeb` result
  shape;
- persist the request snapshot under `termueb_<orderid>` using the same Rust
  SEPA snapshot shape already used by `TermUebSEPA`: source account, descriptor,
  generated PAIN, and non-`sepa.*` segment values.

## Consequences
This completes the scheduled bulk SEPA transfer job without introducing a new
result type. The port remains package-near while sharing renderer and snapshot
logic with the existing SEPA transfer jobs. Raw `sepa.*` parameters are not
persisted in the Rust snapshot, but the generated PAIN persisted in the snapshot
contains the scheduled execution date.

## References
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVTermMultiUebSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVMultiUebSEPA.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
