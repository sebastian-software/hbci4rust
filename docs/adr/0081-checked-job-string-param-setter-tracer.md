# ADR 0081: Checked Job String Param Setter Tracer

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl#setParam(String,String)` validates high-level job
parameters against the job constraint table. It reports invalid user data when
the frontend parameter is not accepted by the job, when the value is empty, or
when an indexed call is used for a non-indexed parameter.

The Rust port started with a permissive `HbciJob::set_param(...)` so early
handler and replay tracers could stage Java-named parameters before the
constraint table existed. ADR 0078 added constraint metadata and ADR 0080 added
constraint verification, but changing `set_param(...)` to return a `Result`
would be a broad public API break at this point.

## Decision

Add `HbciJob::try_set_param(name, value) -> HbciResult<()>`.

For this tracer the checked setter:

- rejects frontend names that are not present in `HbciJob::constraints()`;
- rejects empty values;
- stores the value through the existing frontend parameter map on success;
- uses `HbciErrorKind::InvalidArgument` for invalid job data.

Keep `set_param(...)` permissive for now. It remains useful for early tracer
tests, low-level experiments, and preserving already documented behavior until
queued-job rendering is fully constraint-driven.

Do not port indexed string parameters in this slice.

## Consequences

New code can opt into hbci4java-like checked string parameter behavior without
destabilizing existing handler tracers.

The port now has separate stepping stones for:

- permissive frontend parameter staging: `set_param(...)`;
- checked Java-like frontend setting: `try_set_param(...)`;
- low-level resolution and default application: `verify_constraints()`.

Remaining work:

- decide when `set_param(...)` should become checked or be deprecated;
- port indexed `setParam(String,Integer,String)` semantics;
- route successful checked parameters into a persistent low-level parameter
  store once that store exists;
- add log-filter handling for sensitive parameter values.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0078-job-constraint-metadata-tracer.md`
- `docs/adr/0080-job-constraint-verification-tracer.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,String)`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Integer,String)`
