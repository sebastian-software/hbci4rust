# ADR 0175: Standing Order SEPA Edit Job

## Status

Accepted

## Context

`GVDauerSEPAEdit` is the hbci4java job for changing an existing SEPA standing
order. It is another `AbstractSEPAGV` subclass and uses `DauerSEPAEdit1` /
`DauerSEPAEditRes1` with `pain.001.001.02` as the default PAIN descriptor.

Compared with `DauerSEPANew`, the outgoing FinTS segment also carries the
existing `orderid` and an optional `date`. The result exposes the new bank-side
order id and may expose the previous id as `orderidold`.

ADR 0174 intentionally did not port `AbstractSEPAGV.createSEPAFromParams()`.
That boundary still applies to edit jobs.

## Decision

Port `DauerSEPAEdit` as the next original-near runtime slice:

- expose hbci4java-like constraints for `DauerSEPAEdit1`, including source
  account aliases, `_sepadescriptor`, `_sepapain`, `orderid`, optional `date`,
  SEPA dummy parameters, and `DauerDetails.*` schedule fields;
- render `DauerSEPAEdit1` from caller-provided `_sepapain`, preserving the
  default `pain.001.001.02` descriptor;
- parse `DauerSEPAEditRes1.orderid` and `orderidold` into a typed
  `GvrDauerEdit` result;
- when the bank returns a non-empty new `orderid`, store a `dauer_{orderid}`
  snapshot in the passport from the request-side lowlevel parameters, matching
  the hbci4java follow-up cache shape used by `GVDauerSEPAEdit.extractResults`.

Do not generate PAIN XML from SEPA dummy parameters in this slice. Do not port
delete-job behavior in this slice.

## Consequences

Callers can exercise the PinTAN runtime path for changing SEPA standing orders
when they already have a PAIN document, and follow-up operations can use the
new `dauer_{orderid}` snapshot.

The old order id is preserved in the typed result, but the persistent snapshot
is keyed by the new order id just like hbci4java.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVDauerSEPAEdit.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRDauerEdit.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/AbstractSEPAGV.java`
- `docs/adr/0174-standing-order-sepa-new-job.md`
