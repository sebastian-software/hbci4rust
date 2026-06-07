# ADR 0203: UebBZU Job

## Status

Accepted

## Context

`GVUebBZU` submits a classic, non-SEPA domestic transfer with a German
Bundeseinheitlicher Zahlungsverkehr (BZU) code. In hbci4java it extends
`GVUeb`, but its `getLowlevelName()` still returns `Ueb`. The public Java job
name is therefore `UebBZU`, while the request segment remains `Ueb5` / `HKUEB`
version 5 for HBCI 300.

The job re-adds the normal classic transfer constraints but maps the frontend
parameter `bzudata` to the first DTAUS usage line `usage.usage`. It sets the
transaction key default to `67` instead of the normal transfer default `51`, and
then adds the remaining usage lines from `usage_2` up to the BPD `maxusage`
limit. The class validates `bzudata` before delegating to `GVUeb.setParam()`:
the value must be exactly 13 characters and must pass the original modulo check
digit algorithm.

There is no dedicated `UebBZU` request segment, response segment, or typed
result class. hbci4java attaches only the generic `HBCIJobResultImpl`.

## Decision

Port `UebBZU` as an original-near classic transfer variant.

- Keep the public job name `UebBZU`.
- Render requests through the existing `Ueb5` / `HKUEB` version 5 segment.
- Use the same national `KTV3` source and destination account shape as `Ueb`.
- Map `bzudata` to `Ueb5.usage.usage` and do not expose a normal frontend
  `usage` constraint for this job.
- Keep remaining usage constraints as `usage_2` through `usage_14`, matching the
  current static `Ueb` stopgap until BPD-dynamic `maxusage` expansion is ported.
- Use Java defaults `src.country=DE`, `dst.country=DE`, empty subnumbers, empty
  `name2`, and `key=67`.
- Validate `bzudata` in the checked Rust setter with the hbci4java length and
  check-digit algorithm before persisting lowlevel parameters.
- Keep the unchecked `set_param()` permissive, matching the current Rust
  boundary between raw setters and checked Java-near setters.
- Do not port hbci4java's configurable ignore-error escape hatch for invalid
  BZU data in this slice.
- Do not add a typed result or response content mapping. Preserve only generic
  job status and basic result data, matching hbci4java's `HBCIJobResultImpl`
  usage.
- Verify account CRC for both `src` and `dst`, matching the inherited
  `GVUeb.verifyConstraints()` behavior.

## Consequences

`UebBZU` becomes queueable and usable in PinTAN TAN order-hash flows while
remaining wire-compatible with hbci4java's unusual highlevel-job /
lowlevel-segment split. The BZU validation behavior is available through the
checked API, while fully dynamic usage-line expansion remains a later BPD-driven
validation slice.
