# ADR 0207: DauerNew Job

## Status

Accepted

## Context

`GVDauerNew` creates a classic, non-SEPA standing order. In hbci4java it uses
the lowlevel job name `DauerNew`, returns `GVRDauerNew`, exposes national source
and destination accounts, amount, recipient names, DTAUS usage lines, and the
standing-order schedule fields under `DauerDetails`.

For HBCI 300 the protocol XML defines `DauerNew5` / `HKDAE` version 5. The
segment uses national `KTV3` source and destination accounts, recipient names,
amount, transaction key data, optional repeated DTAUS usage lines, and
`DauerDetails` with `firstdate`, `timeunit`, `turnus`, `execday`, and optional
`lastdate`. The response segment is `DauerNewRes5` / `HIDAE` version 5 and
contains an optional `orderid`.

hbci4java persists all lowlevel request parameters under `dauer_<orderid>` when
the response contains an order id. `DauerEdit` and `DauerDel` later use that
snapshot to prefill their request data. `GVDauerNew.setParam()` also performs
BPD-dependent validation for time unit, turnus, execution day, and transaction
key. The current Rust port already defers dynamic `maxusage` and several
BPD-restriction checks in classic transfer jobs.

## Decision

Port `DauerNew` as an original-near classic standing-order creation job.

- Use `DauerNew5` / `HKDAE` version 5 for HBCI 300 requests and
  `DauerNewRes5` / `HIDAE` version 5 for responses.
- Keep Java-compatible frontend parameters for source account, destination
  account, amount, recipient names, transaction key data, DTAUS usage lines, and
  `DauerDetails` schedule fields.
- Render source and destination accounts as national `KTV3` values under `My`
  and `Other`.
- Keep Java defaults `src.country=DE`, `dst.country=DE`, empty subnumbers,
  empty `name2`, optional empty `lastdate`, `key=52`, and empty usage lines.
- Add static frontend constraints for `usage` through `usage_14`, matching the
  current classic-job stopgap until BPD-dynamic `maxusage` expansion is ported.
- Reuse the existing `GvrDauerNew` result structure and parse `orderid`.
- Store a `dauer_<orderid>` passport snapshot after successful responses, using
  the submitted lowlevel request data plus resolved account fallback values.
- Verify account CRC for both `src` and `dst`.
- Defer hbci4java's BPD-dependent `setParam()` validation for `timeunit`,
  `turnus`, `execday`, and `key`; protocol XML validation still rejects
  structurally invalid rendered values.

## Consequences

The Rust port can now submit classic standing-order creation requests and seed
the same `dauer_<orderid>` persistent-data workflow that later classic
standing-order edit and delete jobs need. Some bank-specific restrictions remain
future work until BPD parameter handling is made dynamic across classic jobs.

