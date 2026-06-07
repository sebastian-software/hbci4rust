# ADR 0226: Donation Job Alias

## Status

Accepted

## Context

`GVDonation` in hbci4java extends `GVUeb` but returns the same lowlevel name,
`Ueb`. It therefore uses the same FinTS 3.0 request segment as `GVUeb`:
`Ueb5` / `HKUEB` version 5.

Unlike the generic `GVUeb`, the donation job exposes a narrower public
parameter set:

- source account fields `src.number`, `src.subnumber`, `src.blz`, and
  `src.country`;
- destination account fields `dst.number`, `dst.subnumber`, `dst.blz`, and
  `dst.country`;
- amount fields `btg.value` and `btg.curr`;
- recipient fields `name` and `name2`;
- `spenderid -> usage.usage`;
- `plz_street -> usage.usage_2`;
- `name_ort -> usage.usage_3`;
- `key -> key`, defaulting to `"69"`.

It inherits `GVUeb` verification behavior, so both source and destination
account CRC checks apply. hbci4java uses the generic `HBCIJobResultImpl` result
shape for the underlying transfer job.

## Decision

Port `Donation` as an original-near public job alias over the existing classic
domestic transfer renderer.

- Add `Donation` to the PinTAN job registry.
- Map its constraints to the existing `Ueb5` paths.
- Preserve the donation-specific frontend names for the first three usage
  lines instead of accepting all generic `usage_*` lines.
- Render through `Ueb5` / `HKUEB` with default transaction key `"69"`.
- Reuse the generic `Ueb` status/result behavior; do not add a typed result
  family or a separate response parser.
- Add the job to the PinTAN order-hash mapping with segment code `HKUEB`.

## Consequences

Callers can create `new_job("Donation")` like in hbci4java while the wire
message remains the same as a classic domestic transfer. The public Rust API
stays close to hbci4java's donation-specific parameter names and does not
silently widen the job to generic `Ueb` usage lines.

## References

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVDonation.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVUeb.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
