# ADR 0194: FestList Job

## Status

Accepted

## Context

`GVFestList` queries existing fixed-term deposits. In hbci4java the request job
uses the lowlevel name `FestList`, exposes national account fields under `my.*`,
and defaults `dummy` / `allaccounts` to `N`.

For HBCI 300 the protocol XML maps this to `FestList4` / `HKFGB` version 4 and
`FestListRes4` / `HIFGB` version 4. The response embeds a fixed-term deposit
record and reuses the `FestCond2` / `FestCondVersion` shape already introduced
for `FestCondList`.

## Decision

Port `FestList` as an original-near query job.

- Keep Java-compatible frontend parameters: `my.number`, `my.subnumber`,
  `my.blz`, `my.country`, and `dummy`.
- Render the HBCI 300 request with `FestList4`, using national account data and
  the Java default `allaccounts=N`.
- Do not expose XML-only `maxentries`, `offset`, or `kontakt` in this slice
  because hbci4java's `GVFestList` constructor does not add those constraints.
- Add `GvrFestList`, `GvrFestListEntry`, and `GvrFestListProlong` result
  structures with fields close to `GVRFestList.Entry`.
- Reuse the existing `GvrFestCond` condition shape for the embedded
  `FestCond`.
- Store dates, status codes, and account values as parsed strings and integers
  rather than adding richer Rust domain types.
- Keep `FestNew` and other fixed-term deposit mutation jobs out of this slice.

## Consequences

This extends the fixed-term deposit query surface without widening the public
API style. The parser becomes a reusable bridge between `FestCondList` and later
fixed-term deposit mutation/list jobs while preserving hbci4java's request
constraints.
