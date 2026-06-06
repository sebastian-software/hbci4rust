# ADR 0085: Constraint Verification Default Persistence

## Status

Accepted

## Context

ADR 0084 made `HbciJob::verify_constraints()` low-level aware but kept the
method non-mutating. hbci4java's `HBCIJobImpl.verifyConstraints()` is
state-changing: when a constraint has no low-level value but resolves to a
non-empty default, it calls `setLowlevelParam(destination, content)`.

That behavior matters because `HBCIDialog` later renders jobs from
`getLowlevelParams()` rather than from a separately returned map.

## Decision

Make `HbciJob::verify_constraints()` take `&mut self`.

For every resolved non-empty constraint value, insert it into
`lowlevel_params` when the destination is not already present. The method still
returns the resolved low-level map for current tests and callers, but the
persistent job state now also receives default values such as
`Saldo7.allaccounts`.

Keep existing low-level values authoritative and do not overwrite them.

Keep the temporary frontend fallback from ADR 0084. If a permissive frontend
parameter is the only available value, verification also moves that value into
the low-level store because the destination was missing.

## Consequences

Constraint verification is closer to hbci4java's lifecycle: successful
verification prepares persistent low-level job state for rendering.

Tests now assert that default values resolved by verification are also available
through `HbciJob::lowlevel_param(...)`.

Remaining work:

- call `verify_constraints()` from queued-job execution at the same lifecycle
  point as hbci4java's `HBCIDialog.addTask(...)`;
- decide how to handle partial low-level mutations if verification later fails;
- port indexed constraint lookup and insertion;
- port segment validation after low-level propagation.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0080-job-constraint-verification-tracer.md`
- `docs/adr/0084-lowlevel-aware-constraint-verification.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#verifyConstraints`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setLowlevelParam`
