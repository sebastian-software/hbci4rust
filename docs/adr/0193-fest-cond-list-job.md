# ADR 0193: FestCondList Job

## Status

Accepted

## Context

`GVFestCondList` is a compact hbci4java job for querying fixed-term deposit
conditions. The Java job declares the lowlevel name `FestCondList`, defaults
`curr` to `EUR`, accepts an optional `maxentries`, and extracts repeated
`FestCond` response groups into `GVRFestCondList`.

The protocol XML maps this job to `FestCondList3` / `HKFGK` version 3 for HBCI
300 and `FestCondListRes3` / `HIFGK` version 3 for responses.

`GVFestList` consumes the same condition structure but has broader account and
prolongation result data. It is better handled as a separate porting slice.

## Decision

Port `FestCondList` as the next original-near PinTAN-compatible handler job.

- Add Java-compatible constraints for `curr` and `maxentries`.
- Render `FestCondList3` requests as `HKFGK`, preserving the Java default
  currency `EUR` and omitting empty optional fields.
- Add `GvrFestCondList` and `GvrFestCond` result data structures with names
  close to `GVRFestCondList.Cond`.
- Store condition dates, version date/time, identifiers, names, amounts, and
  interest method as parsed strings and integers instead of introducing richer
  domain types in this slice.
- Convert `zinsmethode` letters `A` through `F` to the original Java constants
  `0` through `5`; preserve unknown or later values by returning `None`.
- Convert `zinssatz` from protocol `Wrt` text to the Java observable
  thousand-scaled integer.
- Defer `FestList` and reuse of this condition parser by `FestList` until a
  dedicated job/result slice.

## Consequences

The port gains another package-near job without changing the public string job
API. The result structure remains deliberately simple and testable, while
leaving a clear reuse path for the later `FestList` implementation.
