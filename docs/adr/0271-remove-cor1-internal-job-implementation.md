# ADR 0271: Remove COR1 Internal Job Implementation

Date: 2026-06-07

## Status

Accepted.

## Context

ADR 0265 removed `LastCOR1SEPA` and `MultiLastCOR1SEPA` from the public v1 job
registry because the SEPA `COR1` local instrument is no longer relevant for new
SDD Core collections.

ADR 0270 then enforced the public registry boundary at queueing and rendering
time, so manually constructed unsupported jobs can no longer reach executable
handler paths.

After those two decisions, the remaining COR1 request constraints, PAIN
generation branches, renderer branches, orderhash metadata, and response
extraction paths were dead compatibility implementation. Keeping them would
make future audits harder and would preserve a misleading partial
implementation for an unsupported surface.

## Decision

Delete the internal job implementation branches for:

- `LastCOR1SEPA`
- `MultiLastCOR1SEPA`

This removes their request constraints, PAIN-generation cases, queued-job
renderers, orderhash metadata, response root helpers, result-data extraction,
and stored-request snapshot branches.

Do not remove the shared `LastSepa` typed result shape. It remains the typed
shape for supported `LastSEPA`, `LastB2BSEPA`, `MultiLastSEPA`, and
`MultiLastB2BSEPA` jobs, and the upstream result coverage normalization still
records that hbci4java's `GVRLastCOR1SEPA` has no distinct Rust payload shape.

## Consequences

- `LastSEPA`, `LastB2BSEPA`, `MultiLastSEPA`, and `MultiLastB2BSEPA` keep their
  existing SEPA direct-debit behavior.
- `LastCOR1SEPA` and `MultiLastCOR1SEPA` remain visible only as intentional
  unsupported audit gaps and regression tests.
- The internal implementation now matches the public unsupported boundary more
  closely.
- Result coverage remains unchanged.

## References

- `docs/adr/0265-remove-cor1-public-jobs.md`
- `docs/adr/0270-enforce-public-job-registry-boundary.md`
- `docs/architecture/legacy-cleanup-plan.md`
- `src/gv/mod.rs`
- `src/manager/handler.rs`
