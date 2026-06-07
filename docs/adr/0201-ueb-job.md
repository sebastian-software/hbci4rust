# ADR 0201: Ueb Job

## Status

Superseded by ADR 0268 and ADR 0275

## Context

`GVUeb` submits a classic, non-SEPA domestic transfer. It is the classic
counterpart to the already ported `UebSEPA` job, but hbci4java attaches only the
generic `HBCIJobResultImpl`; there is no dedicated `GVRUeb` result class.

For HBCI 300 the protocol XML provides `Ueb2` through `Ueb5`. Version 5 uses
`SingleInlandUser4`, which carries national `KTV3` source and destination
accounts, recipient names, amount, transaction key data, repeated DTAUS usage
lines, and optional XML fields for date and client-side id. hbci4java's
`GVUeb` does not expose `date` or `id` frontend constraints.

hbci4java creates the number of `usage`, `usage_2`, ... constraints from the
bank's `maxusage` restriction. The current Rust port still uses mostly static
job constraints.

## Decision

Port `Ueb` as an original-near classic domestic transfer submission job.

- Use `Ueb5` / `HKUEB` version 5 for HBCI 300 requests.
- Do not add a `UebRes*` response root or a typed result. Preserve only the
  generic job status and basic result data, matching hbci4java's
  `HBCIJobResultImpl` usage.
- Keep Java-compatible frontend parameters for source account, destination
  account, amount, recipient names, transaction key data, and DTAUS usage
  lines.
- Render source and destination accounts as national `KTV3` values under `My`
  and `Other`, matching `SingleInlandUser4`.
- Add static frontend constraints for `usage` through `usage_14` in this slice.
  This mirrors the `TermUeb` stopgap and leaves BPD-dynamic `maxusage`
  expansion for a later restriction-backed validation slice.
- Keep Java defaults `src.country=DE`, `dst.country=DE`, empty subnumbers,
  empty `name2`, `key=51`, and empty usage lines.
- Do not expose the optional XML `date` or `id` fields because hbci4java's
  `GVUeb` does not add constraints for them.
- Do not implement hbci4java's transaction-key restriction validation in this
  slice; preserve the `key` value and leave restriction enforcement for a later
  BPD-driven validation slice.
- Verify account CRC for both `src` and `dst`, matching hbci4java's
  `checkAccountCRC("src")` and `checkAccountCRC("dst")`.

## Consequences

This adds the classic domestic transfer submission path without introducing a
new typed result family. It also gives the TAN order-hash path a concrete
`HKUEB` source segment for classic transfers. `UebBZU`, `UebEil`,
`UebForeign`, and `Umb` remain separate job ports because hbci4java models them
with distinct constraints and, in some cases, distinct segment codes.
