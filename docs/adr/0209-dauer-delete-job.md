# ADR 0209: DauerDel Job

## Status

Superseded by ADR 0269 and ADR 0276

## Context

`GVDauerDel` deletes an existing classic, non-SEPA standing order. It uses the
lowlevel job name `DauerDel` and returns hbci4java's generic
`HBCIJobResultImpl`; there is no specialized `GVRDauerDel` result type.

For HBCI 300 the protocol XML defines `DauerDel4` / `HKDAL` version 4. The
request segment contains national `KTV3` source and destination accounts,
recipient names, amount, transaction key data, optional repeated DTAUS usage
lines, optional `date`, optional `orderid`, and `DauerDetails`. The HBCI 300
XML has `DauerDelPar4` / `HIDALS` parameter data, but no `DauerDelRes*`
response segment.

When hbci4java receives `orderid`, it tries to preload lowlevel parameters from
passport persistent data under `dauer_<orderid>`. It skips `date` and
`Aussetzung.*` snapshot entries. If no snapshot exists, hbci4java continues and
normal constraint verification/rendering decides whether enough explicit data
was supplied.

## Decision

Port `DauerDel` as an original-near classic standing-order delete job.

- Use `DauerDel4` / `HKDAL` version 4 for HBCI 300 requests.
- Do not parse a typed response or add a `HbciJobResultData` variant, because
  upstream has no `GV_Result` class and the protocol XML has no response
  segment for this job.
- Keep Java-compatible frontend parameters for source account, destination
  account, amount, recipient names, transaction key data, DTAUS usage lines,
  optional `date`, optional `orderid`, and `DauerDetails` schedule fields.
- Keep Java defaults: empty source number, source subnumber, destination BLZ,
  destination number, destination subnumber, amount, currency, recipient name,
  schedule fields, `name2`, `date`, `orderid`, and `lastdate`; `src.blz` remains
  the only required constraint, source and destination countries default to
  `DE`, and `key` defaults to `52`.
- Add static frontend constraints for `usage` through `usage_14`, matching the
  current classic-job stopgap until BPD-dynamic `maxusage` expansion is ported.
- Preload stored `dauer_<orderid>` snapshot values into `DauerDel4` before
  constraint verification without overwriting explicit lowlevel values, while
  skipping `date` and `Aussetzung.*` entries.
- Do not reject a missing `dauer_<orderid>` snapshot by itself; rely on normal
  constraint verification and rendering.
- Do not add extra account CRC checks: hbci4java `GVDauerDel` does not override
  `verifyConstraints()`.
- Do not mutate `dauer_<orderid>` passport persistent data after a successful
  delete in this slice, matching the absence of hbci4java `extractResults()`
  logic.
- Defer hbci4java's BPD-dependent `setParam("date")` validation for
  `cantermdel`; protocol XML validation still rejects structurally invalid
  rendered values.

## Consequences

The Rust port can now submit classic standing-order deletes using the same
`dauer_<orderid>` persistent-data snapshots created by list/new/edit jobs. The
preload step preserves explicit caller-provided lowlevel values, consistent with
the existing `DauerEdit` Rust behavior, even though hbci4java's direct
`setParam("orderid")` path copies snapshot values immediately.
