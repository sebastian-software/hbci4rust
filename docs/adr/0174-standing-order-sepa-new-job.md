# ADR 0174: Standing Order SEPA New Job

## Status

Accepted

## Context

`GVDauerSEPANew` is the hbci4java job for creating a new SEPA standing order.
It is an `AbstractSEPAGV` subclass and uses `DauerSEPANew1` /
`DauerSEPANewRes1` with `pain.001.001.02` as the default PAIN descriptor.

The upstream job has two distinct responsibilities:

- render the FinTS segment fields for source account, PAIN descriptor, raw
  PAIN XML, and standing-order schedule details;
- generate `_sepapain` from SEPA frontend parameters during
  `verifyConstraints()`.

The Rust port already has `DauerSEPAList`, a PAIN.001 result parser, and
Rust-native `dauer_{orderid}` persistent-data snapshots. It does not yet have
SEPA PAIN generators.

## Decision

Port `DauerSEPANew` in a first original-near runtime slice:

- expose hbci4java-like constraints for `DauerSEPANew1`, including the source
  account aliases, `_sepadescriptor`, `_sepapain`, SEPA dummy parameters, and
  `DauerDetails.*` schedule fields;
- render `DauerSEPANew1` from caller-provided `_sepapain`, preserving the
  default `pain.001.001.02` descriptor;
- parse `DauerSEPANewRes1.orderid` into a typed `GvrDauerNew` result;
- when the bank returns a non-empty `orderid`, store a `dauer_{orderid}`
  snapshot in the passport from the request-side lowlevel parameters, matching
  the hbci4java follow-up cache shape as closely as the Rust job model allows.

Do not generate PAIN XML from SEPA dummy parameters in this slice. PAIN.001
generation remains a later SEPA generator slice and must get its own ADR before
implementation.

## Consequences

Callers can exercise the PinTAN runtime path for creating SEPA standing orders
when they already have a PAIN document, and the returned order id feeds the same
Rust-native persistent-data map used by `DauerSEPAList`.

This keeps the implementation close to the upstream segment/result behavior
without hiding the missing `AbstractSEPAGV.createSEPAFromParams()` parity.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVDauerSEPANew.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/AbstractSEPAGV.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRDauerNew.java`
- `docs/adr/0173-standing-order-persistent-data.md`
