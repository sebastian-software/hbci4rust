# ADR 0229: MultiUeb Job

## Status

Accepted

## Context

`GVMultiUeb` in hbci4java implements the classic domestic bulk transfer job. It
extends `AbstractMultiGV`, uses lowlevel name `SammelUeb`, and in FinTS 3.0 maps
to request segment `SammelUeb6` / `HKSUB` version 6.

The job is intentionally thin:

- `data` is the required DTAUS bulk-transfer payload and is mapped to lowlevel
  `data`;
- setting `data` in hbci4java prefixes the value with `B` for FinTS binary
  transport;
- account fields are `my.country`, `my.blz`, `my.number`, and `my.subnumber`,
  mapped to `KTV.*`;
- hbci4java verifies account CRC for `my`;
- hbci4java uses generic `HBCIJobResultImpl` result handling and no typed
  `GVR*` result.

The protocol defines no `SammelUebRes` response segment for successful
submissions; completion is status-segment driven.

## Decision

Port `MultiUeb` as an original-near legacy domestic PinTAN job.

- Add `MultiUeb` to the PinTAN job registry.
- Map constraints to `SammelUeb6`, preserving hbci4java's `data` and `my.*`
  frontend names.
- Preserve hbci4java's `data` binary behavior by storing `B...` on the lowlevel
  parameter when callers set `data`.
- Render `SammelUeb6` / `HKSUB` with account and binary data, but do not
  generate DTAUS from structured parameters.
- Add the job to the PinTAN order-hash mapping with segment code `HKSUB`.
- Keep result handling generic and do not add a typed result parser.

## Consequences

Callers can create `new_job("MultiUeb")` and supply an already serialized DTAUS
payload like hbci4java expects. This advances legacy domestic parity while
leaving DTAUS generation/parsing as a possible later compatibility slice rather
than folding it into this job port.

## References

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVMultiUeb.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/AbstractMultiGV.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
