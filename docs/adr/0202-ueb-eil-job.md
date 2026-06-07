# ADR 0202: UebEil Job

## Status

Accepted

## Context

`GVUebEil` submits a classic, non-SEPA urgent domestic transfer. In hbci4java it
extends `GVUeb`, overrides the lowlevel name to `UebEil`, and re-adds the same
frontend constraints as `GVUeb`.

For HBCI 300 the protocol XML defines `UebEil1` / `HKEIL` version 1. The segment
uses `SingleInlandUser4`, so its request payload has the same national `KTV3`
source and destination accounts, recipient names, amount, transaction key data,
and repeated DTAUS usage lines as `Ueb5`. The BPD parameter segment is
`UebEilPar1` / `HIEILS`. There is no dedicated `UebEilRes*` response segment and
hbci4java attaches only the generic `HBCIJobResultImpl`.

## Decision

Port `UebEil` as an original-near classic urgent transfer submission job.

- Use `UebEil1` / `HKEIL` version 1 for HBCI 300 requests.
- Reuse the same frontend parameter shape as `Ueb`: source account,
  destination account, amount, recipient names, transaction key data, and DTAUS
  usage lines.
- Render source and destination accounts as national `KTV3` values under `My`
  and `Other`, matching `SingleInlandUser4`.
- Add static frontend constraints for `usage` through `usage_14`, matching the
  current `Ueb` and `TermUeb` stopgap until BPD-dynamic `maxusage` expansion is
  ported.
- Keep Java defaults `src.country=DE`, `dst.country=DE`, empty subnumbers,
  empty `name2`, `key=51`, and empty usage lines.
- Do not add a typed result or response content mapping. Preserve only the
  generic job status and basic result data, matching hbci4java's
  `HBCIJobResultImpl` usage.
- Do not expose optional XML `date` or `id` fields because hbci4java's
  `GVUebEil` does not add constraints for them.
- Do not implement hbci4java's transaction-key restriction validation in this
  slice; leave restriction enforcement for a later BPD-driven validation slice.
- Verify account CRC for both `src` and `dst`, matching the inherited
  `GVUeb.verifyConstraints()` behavior.

## Consequences

`UebEil` becomes available as a distinct queueable job and TAN order-hash source
while sharing the classic transfer helpers introduced for `Ueb`. `UebBZU`,
`UebForeign`, and other classic transfer variants remain separate slices because
their constraints and validation differ.
