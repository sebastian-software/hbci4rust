# 0220 Port ChangePIN As PinTAN Management Job

## Status
Accepted

## Context
`GVChangePIN` in hbci4java implements the HBCI/FinTS PIN change business
transaction for PIN/TAN passports. The Java class is intentionally small:

- lowlevel job name: `ChangePIN`;
- result class: generic `HBCIJobResultImpl`;
- high-level constraint: `newpin -> newpin`, required, secret-filtered.

In FinTS 3.0 the request segment is `ChangePIN1` with code `HKPAE`, version 1.
It contains only the user segment head and the optional protocol-level `newpin`
data element. The job constructor makes `newpin` a required high-level
parameter by passing no default value.

The upstream package documentation calls out an important security behavior:
after a successful PIN change, hbci4java does not automatically switch the
passport to the new PIN. The caller must provide the new PIN explicitly for any
later message that requires it, including dialog end. The Rust port currently
caches the runtime PIN in the PinTAN passport, so this job must not mutate that
cache when the bank returns success.

For PinTAN SCA/orderhash handling, hbci4java treats `HKPAE` as one of the
management business transactions whose segment code may become the TAN reference
segment instead of a generic dialog segment.

## Decision
Port `ChangePIN` as the next original-near PinTAN job slice:

- expose frontend job name `ChangePIN`;
- add the required `newpin` constraint mapped to `ChangePIN1.newpin`;
- render `ChangePIN1` as `HKPAE` with the supplied new PIN;
- do not create a specialized Rust result data variant, matching hbci4java's
  generic `HBCIJobResultImpl`;
- return only generic job status and empty content result data unless the bank
  sends protocol status values;
- add `HKPAE` orderhash source metadata for PinTAN/TAN flows;
- preserve the current cached runtime PIN after success instead of replacing it
  with `newpin`.

## Consequences
This adds a PinTAN management job that is useful in live client workflows while
remaining a narrow, original-near port. The new PIN is present in the rendered
wire message and test fixtures, so tests must avoid treating it as normal
non-secret application state. Full secret redaction/log filtering remains a
separate follow-up because the Rust port does not yet have hbci4java's
`LogFilter` equivalent.

## References
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVChangePIN.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/package.html`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/passport/AbstractPinTanPassport.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
