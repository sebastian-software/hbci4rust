# ADR 0208: DauerEdit Job

## Status

Accepted

## Context

`GVDauerEdit` changes an existing classic, non-SEPA standing order. It uses the
lowlevel job name `DauerEdit`, returns `GVRDauerEdit`, and exposes the same
national account, amount, recipient, usage, and `DauerDetails` schedule fields
as `GVDauerNew`, plus `orderid` and an optional scheduled change `date`.

For HBCI 300 the protocol XML defines `DauerEdit5` / `HKDAN` version 5. The
request segment contains national `KTV3` source and destination accounts,
recipient names, amount, transaction key data, optional repeated DTAUS usage
lines, optional `date`, optional `orderid`, and `DauerDetails`. The response
segment is `DauerEditRes5` / `HIDAN` version 5 and contains `orderid` plus
optional `orderidold`.

When hbci4java receives `orderid`, it tries to preload lowlevel parameters from
passport persistent data under `dauer_<orderid>`. Existing caller-supplied
lowlevel values are not overwritten. It skips `date` and `Aussetzung.*` snapshot
entries while preloading. If no snapshot exists, hbci4java continues and normal
constraint verification decides whether enough explicit data was supplied. After
a successful edit response, hbci4java stores the submitted lowlevel request data
under `dauer_<new_orderid>`, excluding keys ending in `.orderid`.

## Decision

Port `DauerEdit` as an original-near classic standing-order edit job.

- Use `DauerEdit5` / `HKDAN` version 5 for HBCI 300 requests and
  `DauerEditRes5` / `HIDAN` version 5 for responses.
- Keep Java-compatible frontend parameters for source account, destination
  account, amount, recipient names, transaction key data, DTAUS usage lines,
  `DauerDetails` schedule fields, optional `date`, and `orderid`.
- Render source and destination accounts as national `KTV3` values under `My`
  and `Other`.
- Keep Java defaults `src.country=DE`, `dst.country=DE`, empty subnumbers,
  empty `name2`, optional empty `date`, optional empty `lastdate`, `key=52`, and
  empty usage lines.
- Add static frontend constraints for `usage` through `usage_14`, matching the
  current classic-job stopgap until BPD-dynamic `maxusage` expansion is ported.
- Preload stored `dauer_<orderid>` snapshot values into `DauerEdit5` before
  constraint verification without overwriting explicit lowlevel values, while
  skipping `date` and `Aussetzung.*` entries.
- Do not reject a missing `dauer_<orderid>` snapshot by itself; rely on normal
  constraint verification, matching hbci4java's non-fatal preload behavior.
- Reuse the existing `GvrDauerEdit` result structure and parse `orderid` and
  `orderidold`.
- Store a new `dauer_<orderid>` passport snapshot after successful responses,
  excluding the old `.orderid` value.
- Verify account CRC for both `src` and `dst`.
- Defer hbci4java's BPD-dependent `setParam()` validation for `date`,
  `timeunit`, `turnus`, `execday`, and `key`; protocol XML validation still
  rejects structurally invalid rendered values.

## Consequences

The Rust port can now change classic standing orders using the same
`dauer_<orderid>` persistent-data workflow seeded by `DauerNew` and `DauerList`.
Callers may still submit a fully explicit edit without a local snapshot, while
partial edits depend on the local snapshot being present.

