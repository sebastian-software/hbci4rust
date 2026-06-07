# ADR 0176: Standing Order SEPA Delete Job

## Status

Accepted

## Context

`GVDauerSEPADel` is the hbci4java job for deleting an existing SEPA standing
order. It is an `AbstractSEPAGV` subclass and uses `DauerSEPADel1` /
`HKCDL` with `pain.001.001.02` as the default PAIN descriptor.

The original protocol XML has an important asymmetry: it defines
`DauerSEPADel1` and `DauerSEPADelPar1`, but no `DauerSEPADelRes1`. The `GVRes`
syntax function contains `DauerSEPAEditRes1`, and hbci4java's
`GVDauerSEPADel` uses `GVRDauerEdit`, setting `orderid` and optional
`orderidold` just like `GVDauerSEPAEdit`.

ADR 0174 and ADR 0175 intentionally did not port
`AbstractSEPAGV.createSEPAFromParams()`. That boundary still applies here.

## Decision

Port `DauerSEPADel` as the next original-near runtime slice:

- expose hbci4java-like constraints for `DauerSEPADel1`, including source
  account aliases, `_sepadescriptor`, `_sepapain`, `orderid`, optional `date`,
  SEPA dummy parameters, and `DauerDetails.*` schedule fields;
- render `DauerSEPADel1` from caller-provided `_sepapain`, preserving the
  default `pain.001.001.02` descriptor;
- parse returned order ids through `DauerSEPAEditRes1` into the existing typed
  `GvrDauerEdit` result, because the upstream XML does not define a
  delete-specific result segment;
- when a non-empty returned `orderid` exists, store a `dauer_{orderid}`
  snapshot in the passport from the request-side lowlevel parameters, matching
  `GVDauerSEPADel.extractResults`.

Do not generate PAIN XML from SEPA dummy parameters in this slice. Do not port
the HBCI 3.0-only `DauerSEPADel2` / `DauerDetails4` variant in this slice.

## Consequences

The Rust port can render and replay-test the PinTAN runtime path for deleting
SEPA standing orders when callers already provide a PAIN document.

The shared `GvrDauerEdit` typed result is intentional and original-near, even
though the job name is delete-specific.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVDauerSEPADel.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRDauerEdit.java`
- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0175-standing-order-sepa-edit-job.md`
