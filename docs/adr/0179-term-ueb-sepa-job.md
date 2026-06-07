# ADR 0179: Scheduled SEPA Transfer Job

## Status

Accepted

## Context

`GVTermUebSEPA` is hbci4java's high-level job for submitting a scheduled SEPA
credit transfer. It is an `AbstractSEPAGV` subclass and uses
`TermUebSEPA1` / `HKCSE` with `pain.001.001.02` as the default PAIN descriptor.

The job stores the scheduled execution date inside the PAIN document as the
SEPA parameter `date`. The protocol segment itself contains only the source
account, PAIN descriptor, and PAIN binary block. The response segment
`TermUebSEPARes1` / `HICSE` contains an optional `orderid`, exposed through
hbci4java's `GVRTermUeb`.

When an order id is returned, hbci4java persists the request lowlevel
parameters under `termueb_{orderid}` so later edit/delete jobs can reuse them.

## Decision

Port `TermUebSEPA` as the next original-near PinTAN job slice:

- expose hbci4java-like constraints for `TermUebSEPA1`, including source
  account aliases, `_sepadescriptor`, `_sepapain`, SEPA dummy parameters,
  required `date`, `sepaid`, `pmtinfid`, `endtoendid`, and `purposecode`;
- render `TermUebSEPA1` as `HKCSE` from source account, PAIN descriptor, and
  generated or caller-provided `_sepapain`;
- use the ADR 0177 `pain.001.001.02` single-transfer generator, with `date`
  mapped into the PAIN execution date;
- add a minimal typed `GvrTermUeb` result containing the returned order id;
- persist a `termueb_{orderid}` snapshot from request-side lowlevel parameters
  when the response returns a non-empty order id.

Do not port `TermUebSEPAList`, `TermUebSEPAEdit`, `TermUebSEPADel`,
`TermMultiUebSEPA`, newer PAIN.001 versions, or BPD-based PAIN version
negotiation in this slice.

## Consequences

The Rust port can now replay-test scheduled SEPA transfer submission and carry
forward the bank-provided order id for later scheduled-transfer slices.

The request rendering reuses the same compact PAIN generation path as
`UebSEPA` and standing order jobs, keeping the port package-near without
introducing a second XML generation mechanism.

The persisted snapshot format is intentionally request-lowlevel shaped and may
be extended when edit/delete scheduled-transfer jobs are ported.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVTermUebSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRTermUeb.java`
- `target/reference/hbci4java/src/test/java/org/kapott/hbci4java/sepa/TestGVTermUebSEPA.java`
- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0177-pain001-transfer-generator.md`
- `docs/adr/0178-ueb-sepa-job.md`
