# ADR 0180: Scheduled SEPA Transfer Edit Job

## Status

Accepted

## Context

`GVTermUebSEPAEdit` is hbci4java's high-level job for changing an existing
scheduled SEPA credit transfer. It is an `AbstractSEPAGV` subclass and uses
`TermUebSEPAEdit1` / `HKCSA` with `pain.001.001.02` as the default PAIN
descriptor.

The request segment contains the source account, PAIN descriptor, PAIN binary
block, and the old `orderid` to edit. The response segment
`TermUebSEPAEditRes1` / `HICSA` returns an optional new `orderid` and optional
`orderidold`, exposed through hbci4java's `GVRTermUebEdit`.

Like `GVTermUebSEPA`, hbci4java persists request-side lowlevel parameters under
`termueb_{orderid}` when the bank returns a non-empty new order id. Its SEPA
request data is still generated from the same `UebSEPA` PAIN generator family.

## Decision

Port `TermUebSEPAEdit` as the next scheduled-transfer slice:

- expose hbci4java-like constraints for `TermUebSEPAEdit1`, including source
  account aliases, required `orderid`, `_sepadescriptor`, `_sepapain`, SEPA
  dummy parameters, required `date`, `sepaid`, `pmtinfid`, `endtoendid`, and
  `purposecode`;
- render `TermUebSEPAEdit1` as `HKCSA`, preserving the segment order
  account, descriptor, PAIN binary block, and old order id;
- use the ADR 0177 `pain.001.001.02` single-transfer generator when
  `_sepapain` is absent;
- add a minimal typed `GvrTermUebEdit` result containing `order_id` and
  `order_id_old`;
- persist a `termueb_{orderid}` request snapshot when the response returns a
  non-empty new order id.

Do not port `TermUebSEPADel`, `TermUebSEPAList`, `TermMultiUebSEPA`, newer
PAIN.001 versions, or BPD-based PAIN version negotiation in this slice.

## Consequences

The Rust port can now replay-test changing scheduled SEPA transfers and carry
forward the bank's replacement order id for later scheduled-transfer operations.

The implementation deliberately reuses the `TermUebSEPA` PAIN generation and
snapshot shape, even though the helper names are still broader scheduled-order
candidates rather than final idiomatic Rust names.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVTermUebSEPAEdit.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRTermUebEdit.java`
- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0179-term-ueb-sepa-job.md`
