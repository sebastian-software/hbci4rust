# ADR 0200: TermUeb Job

## Status

Accepted

## Context

`GVTermUeb` submits a classic, non-SEPA scheduled transfer. It is the classic
counterpart to the already ported `TermUebSEPA` job and returns
hbci4java's `GVRTermUeb`.

For HBCI 300 the protocol XML provides `TermUeb2` through `TermUeb4`. Version 4
uses `SingleInlandUser4`, which carries national `KTV3` source and destination
accounts, recipient names, amount, transaction key data, repeated DTAUS usage
lines, execution date, and an optional client-side id. The response
`TermUebRes4` returns only an optional `orderid`.

hbci4java creates the number of `usage`, `usage_2`, ... constraints from the
bank's `maxusage` restriction. The current Rust port still uses mostly static
job constraints.

## Decision

Port `TermUeb` as an original-near classic scheduled-transfer submission job.

- Use `TermUeb4` / `HKTUE` version 4 for HBCI 300 requests and `TermUebRes4` /
  `HITUE` version 4 for responses.
- Keep Java-compatible frontend parameters for source account, destination
  account, amount, recipient names, execution date, transaction key data, and
  DTAUS usage lines.
- Render source and destination accounts as national `KTV3` values under `My`
  and `Other`, matching `SingleInlandUser4`.
- Add static frontend constraints for `usage` through `usage_14` in this slice.
  This keeps the common hbci4java names and avoids introducing BPD-dynamic
  constraint generation inside this job port. A later tracer can replace this
  with dynamic restriction-backed expansion.
- Keep Java defaults `src.country=DE`, `dst.country=DE`, empty subnumbers,
  empty `name2`, `key=51`, and empty usage lines.
- Do not implement hbci4java's transaction-key restriction validation in this
  slice; preserve the `key` value and leave restriction enforcement for a later
  BPD-driven validation slice.
- Reuse the existing `GvrTermUeb` result structure and parse only `orderid`.
- Store `termueb_<orderid>` passport snapshots from the submitted lowlevel
  request fields, matching hbci4java's use of `getLowlevelParams()`.
- Verify account CRC for both `src` and `dst`, matching hbci4java's
  `checkAccountCRC("src")` and `checkAccountCRC("dst")`.

## Consequences

This adds the classic scheduled-transfer submission path without changing the
public stringly job API. The static usage-line limit is deliberately pragmatic
and documented; dynamic BPD-backed constraint expansion remains a separate
rustification/backlog item.
