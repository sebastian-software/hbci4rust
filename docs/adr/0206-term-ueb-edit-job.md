# ADR 0206: TermUebEdit Job

## Status

Accepted

## Context

`GVTermUebEdit` changes an existing classic, non-SEPA scheduled transfer. In
hbci4java it exposes the same classic transfer fields as `GVTermUeb`, adds
`orderid -> id`, returns `GVRTermUebEdit`, and verifies source and destination
account CRC values.

For HBCI 300 the protocol XML defines `TermUebEdit4` / `HKTUA` version 4. The
segment uses `SingleInlandUser4`, so it contains national `KTV3` source and
destination accounts, recipient names, amount, transaction key data, repeated
DTAUS usage lines, execution date, and the `id` of the order being changed. The
response segment is `TermUebEditRes4` / `HITUA` version 4 and contains a new
`orderid` plus optional `orderidold`.

When hbci4java receives `orderid`, it preloads lowlevel parameters from passport
persistent data under `termueb_<orderid>` unless the caller has already set a
specific lowlevel value. After a successful change, it stores the submitted
lowlevel request data under `termueb_<new_orderid>`, excluding keys ending in
`.id`.

## Decision

Port `TermUebEdit` as an original-near classic scheduled-transfer edit job.

- Use `TermUebEdit4` / `HKTUA` version 4 for HBCI 300 requests and
  `TermUebEditRes4` / `HITUA` version 4 for responses.
- Keep Java-compatible frontend parameters for source account, destination
  account, amount, recipient names, execution date, transaction key data, DTAUS
  usage lines, and `orderid`.
- Render source and destination accounts as national `KTV3` values under `My`
  and `Other`, matching `SingleInlandUser4`.
- Add static frontend constraints for `usage` through `usage_14`, matching the
  current classic transfer stopgap until BPD-dynamic `maxusage` expansion is
  ported.
- Keep Java defaults `src.country=DE`, `dst.country=DE`, empty subnumbers,
  empty destination account number, empty `name2`, `key=51`, and empty usage
  lines.
- Preload stored `termueb_<orderid>` snapshot values into the `TermUebEdit4`
  lowlevel segment before rendering, without overwriting explicitly supplied
  lowlevel values.
- Reject checked rendering when the referenced `termueb_<orderid>` snapshot is
  missing. Do not port hbci4java's configurable ignore-error escape hatch in
  this slice.
- Reuse the existing `GvrTermUebEdit` result structure and parse `orderid` and
  `orderidold`.
- Store a new `termueb_<orderid>` passport snapshot after successful responses,
  excluding the old `.id` value, matching hbci4java's
  `!key.endsWith(".id")` persistence rule.
- Verify account CRC for both `src` and `dst`.

## Consequences

The Rust port can now change classic scheduled transfers that were previously
submitted or listed through the port, while preserving hbci4java's stateful
`termueb_<orderid>` workflow. Direct edits without a local snapshot remain a
future compatibility decision because hbci4java's behavior depends on a global
ignore-error setting that is not yet modeled.
