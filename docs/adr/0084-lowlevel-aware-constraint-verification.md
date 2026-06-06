# ADR 0084: Lowlevel Aware Constraint Verification

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl.verifyConstraints()` checks each constraint by reading
the low-level destination first via `getLowlevelParam(destination)`. Only when no
non-empty low-level value exists does the Java method fall back to the
constraint default. Frontend values have normally already been copied into
`llParams` by checked `setParam(...)` calls.

ADR 0082 added `HbciJob` low-level state, and ADR 0083 made the Saldo renderer
prefer that state. `HbciJob::verify_constraints()` still only looked at the
frontend parameter map, which meant a job with valid low-level state could be
reported as missing required frontend data.

## Decision

Make `HbciJob::verify_constraints()` resolve each constraint in this order:

1. non-empty low-level destination value;
2. non-empty frontend value, as a temporary compatibility bridge;
3. configured default value;
4. missing-required-parameter error.

Keep the frontend fallback for now because direct `set_param(...)` remains a
documented permissive staging API while the port is still migrating toward a
fully low-level job lifecycle.

Do not mutate `lowlevel_params` in this slice. The method still returns a
resolved map, matching ADR 0080's non-mutating tracer boundary.

## Consequences

Constraint verification now aligns with hbci4java's primary source of truth:
low-level job parameters.

Tests cover both low-level-only required parameters and low-level precedence
over conflicting frontend values.

Remaining work:

- turn `verify_constraints()` into a lifecycle operation that writes defaults
  back into `lowlevel_params`;
- integrate verification into queued-job execution at the same point as
  `HBCIDialog.addTask(...)`;
- port indexed constraint lookup with `insertIndex(destination, 0)`;
- remove the frontend fallback once all supported setup paths populate
  low-level state.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0080-job-constraint-verification-tracer.md`
- `docs/adr/0082-job-lowlevel-param-store-tracer.md`
- `docs/adr/0083-saldo-renderer-lowlevel-param-source.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#verifyConstraints`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#getLowlevelParam`
