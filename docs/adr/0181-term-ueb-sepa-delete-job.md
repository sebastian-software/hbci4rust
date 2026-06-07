# ADR 0181: Scheduled SEPA Transfer Delete Job

## Status

Accepted

## Context

`GVTermUebSEPADel` is hbci4java's high-level job for deleting an existing
scheduled SEPA credit transfer. It is an `AbstractSEPAGV` subclass and uses
`TermUebSEPADel1` / `HKCSL` with `pain.001.001.02` as the default PAIN
descriptor.

The request segment contains the source account, PAIN descriptor, PAIN binary
block, and the `orderid` to delete. The protocol XML defines
`TermUebSEPADel1` and `TermUebSEPADelPar1`, but no `TermUebSEPADelRes1`.
hbci4java therefore uses `HBCIJobResultImpl` directly rather than a
delete-specific typed result.

The BPD parameter segment contains `orderdata_required`, but the current
original-near runtime slices do not yet branch request rendering on BPD
restriction values.

## Decision

Port `TermUebSEPADel` as the next scheduled-transfer slice:

- expose hbci4java-like constraints for `TermUebSEPADel1`, including source
  account aliases, required `orderid`, `_sepadescriptor`, `_sepapain`, SEPA
  dummy parameters, required `date`, `sepaid`, `pmtinfid`, `endtoendid`, and
  `purposecode`;
- render `TermUebSEPADel1` as `HKCSL`, preserving the segment order account,
  descriptor, PAIN binary block, and order id;
- use the ADR 0177 `pain.001.001.02` single-transfer generator when
  `_sepapain` is absent;
- keep result handling as status-only with no new typed result and no
  delete-specific content-data root, matching the absence of a response
  segment in the upstream XML.

Do not port `TermUebSEPAList`, `TermMultiUebSEPA`, non-SEPA `TermUebDel`,
newer PAIN.001 versions, BPD-based `orderdata_required` behavior, or
BPD-based PAIN version negotiation in this slice.

## Consequences

The Rust port can now replay-test deleting scheduled SEPA transfers when the
caller provides enough data for the PAIN request or lets the existing
single-transfer PAIN generator build it.

The lack of a typed result is intentional and original-near. A successful
delete job still has normal message and job status values, but there is no
bank-returned order id to persist.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVTermUebSEPADel.java`
- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0180-term-ueb-sepa-edit-job.md`
