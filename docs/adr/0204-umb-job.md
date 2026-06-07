# ADR 0204: Umb Job

## Status

Superseded by ADR 0268 and ADR 0275

## Context

`GVUmb` submits a classic, non-SEPA domestic account transfer
(`Umbuchung`). In hbci4java it extends `GVUeb`, overrides the lowlevel name to
`Umb`, and re-adds the same frontend constraints as `GVUeb`.

For HBCI 300 the protocol XML defines `Umb1` / `HKUMB` version 1 for HBCI 2.2
and `Umb2` / `HKUMB` version 2 for HBCI 3.0. `Umb2` uses
`SingleInlandUser4`, so its request payload has the same national `KTV3`
source and destination accounts, recipient names, amount, transaction key data,
and repeated DTAUS usage lines as `Ueb5`. The BPD parameter segment is
`UmbPar2` / `HIUMBS`.

There is no dedicated `GVRUmb` result class in the pinned upstream tree and no
dedicated `UmbRes*` response segment. hbci4java therefore uses only the generic
job result status shape.

## Decision

Port `Umb` as an original-near classic domestic account-transfer submission job.

- Use `Umb2` / `HKUMB` version 2 for HBCI 300 requests.
- Reuse the same frontend parameter shape as `Ueb`: source account,
  destination account, amount, recipient names, transaction key data, and DTAUS
  usage lines.
- Render source and destination accounts as national `KTV3` values under `My`
  and `Other`, matching `SingleInlandUser4`.
- Add static frontend constraints for `usage` through `usage_14`, matching the
  current `Ueb`, `UebEil`, and `TermUeb` stopgap until BPD-dynamic `maxusage`
  expansion is ported.
- Keep Java defaults `src.country=DE`, `dst.country=DE`, empty subnumbers,
  empty `name2`, `key=51`, and empty usage lines.
- Do not add a typed result or response content mapping. Preserve only generic
  job status and basic result data, matching hbci4java's generic result usage.
- Do not implement hbci4java's transaction-key restriction validation in this
  slice; leave restriction enforcement for a later BPD-driven validation slice.
- Verify account CRC for both `src` and `dst`, matching the inherited
  `GVUeb.verifyConstraints()` behavior.

## Consequences

`Umb` becomes available as a queueable classic account-transfer job and TAN
order-hash source while sharing the classic transfer renderer used by `Ueb`,
`UebBZU`, and `UebEil`. `UebForeign` and other remaining classic or investment
jobs stay separate slices because their parameter and result shapes differ.
