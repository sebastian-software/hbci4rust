# ADR 0082: Job Lowlevel Param Store Tracer

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl#setParam(String,String)` stores accepted high-level
frontend parameters in the job's internal low-level parameter map. The mapping
comes from the job constraint table, where a frontend name can point to one or
more low-level destination paths.

The Rust port already has:

- permissive frontend parameter staging via `set_param(...)`;
- checked frontend setting via `try_set_param(...)`;
- constraint resolution via `verify_constraints()`.

However, `try_set_param(...)` still only wrote the frontend parameter map, so it
did not yet reflect hbci4java's internal low-level state.

## Decision

Add an internal `lowlevel_params` map to `HbciJob` with public read-only access:

- `lowlevel_param(name)`;
- `lowlevel_params()`.

When `try_set_param(...)` succeeds, copy the value to every low-level
destination registered for the frontend name.

Also let `set_param_account(...)` use the same frontend-plus-lowlevel write path
for accepted and non-empty account fields, because hbci4java's account overload
delegates to the checked string setter after its own `acceptsParam(...)` checks.

Keep direct `set_param(...)` as frontend-only staging. That preserves ADR 0081's
explicitly permissive compatibility path while the port is still building out
the full original job lifecycle.

## Consequences

The Rust job object now exposes the first persistent equivalent of hbci4java's
`llParams` for currently ported constraints.

Tests cover that:

- direct permissive `set_param(...)` does not populate low-level state;
- `try_set_param(...)` populates the mapped low-level destination;
- `set_param_account(...)` populates accepted account low-level destinations.

Remaining work:

- use the low-level store in queued-job rendering once renderers become fully
  constraint-driven;
- have `verify_constraints()` merge defaults into persistent low-level state
  when the lifecycle is ready for mutation;
- port multi-destination constraints and indexed low-level path insertion;
- add log-filter metadata for sensitive low-level values.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0078-job-constraint-metadata-tracer.md`
- `docs/adr/0080-job-constraint-verification-tracer.md`
- `docs/adr/0081-checked-job-string-param-setter-tracer.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,String)`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#verifyConstraints`
