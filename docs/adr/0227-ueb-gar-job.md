# ADR 0227: UebGar Job

## Status

Accepted

## Context

`GVUebGar` in hbci4java extends `GVUeb` but changes the lowlevel job name to
`UebGar`. In FinTS 3.0 this maps to request segment `UebGar1` / `HKGUB`
version 1.

The job keeps the classic domestic transfer shape:

- source account fields `src.country`, `src.blz`, `src.number`, and
  `src.subnumber`;
- destination account fields `dst.country`, `dst.blz`, `dst.number`, and
  `dst.subnumber`;
- amount fields `btg.value` and `btg.curr`;
- recipient fields `name` and `name2`;
- transaction key `key`, defaulting to `"51"`;
- additional key `addkey`, defaulting to `"100"`;
- the original `usage`, `usage_2`, and later usage-line naming pattern from
  the protocol restrictions.

It inherits `GVUeb` verification behavior, so both source and destination
account CRC checks apply. The protocol defines `UebGarRes1` / `HIGUB`, but
hbci4java still uses the generic `HBCIJobResultImpl` result shape.

## Decision

Port `UebGar` as a narrow original-near classic domestic transfer variant.

- Add `UebGar` to the PinTAN job registry.
- Map constraints to `UebGar1` and keep the hbci4java defaults `key = "51"`
  and `addkey = "100"`.
- Reuse the existing classic transfer rendering path, extending it only enough
  to render optional `addkey`.
- Add the job to the PinTAN order-hash mapping with segment code `HKGUB`.
- Keep result handling generic: no typed result type, but preserve raw
  `UebGarRes1` response content in job result data when present.

## Consequences

`new_job("UebGar")` becomes available without widening the API into a more
idiomatic abstraction. The implementation remains close to hbci4java and shares
the classic domestic transfer code path, while the distinct lowlevel segment and
additional key stay visible for tests and later parity work.

## References

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVUebGar.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVUeb.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
