# ADR 0087: Indexed Job Param Setter Tracer

## Status

Accepted

## Context

hbci4java supports indexed high-level job parameters via
`HBCIJobImpl#setParam(String,Integer,String)`. When the frontend parameter was
registered as indexed, the Java code inserts the index into the low-level
destination path with `insertIndex(...)` before writing the value to `llParams`.

The Rust port already stored `HbciJobConstraint::indexed`, but no setter used
that metadata yet.

## Decision

Add `HbciJob::try_set_indexed_param(name, index, value)`.

The checked indexed setter:

- rejects unknown frontend names;
- rejects empty values;
- rejects indexed calls for frontend names without indexed constraints;
- writes to low-level destinations only;
- uses a Rust implementation of hbci4java's `insertIndex(...)` shape for
  three- and four-component low-level paths.

Do not store a plain frontend parameter value for indexed calls. Repeated values
need indexed low-level paths, and a single frontend map entry would lose the
index information.

Do not port indexed constraint verification in this slice. `verifyConstraints()`
has a separate `insertIndex(destination, 0)` fallback in hbci4java that should be
ported with focused tests.

## Consequences

The existing `indexed` constraint flag is now behaviorally meaningful for the
checked setter path.

Tests use a synthetic indexed job constraint because no currently rendered
PinTAN tracer job exposes indexed parameters yet.

Remaining work:

- port indexed account/value/date helpers;
- port `verifyConstraints()` indexed lookup for index `0`;
- derive indexed constraints from the upstream job/protocol metadata instead of
  synthetic tests;
- add real PinTAN job tracers with repeated parameter groups.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0078-job-constraint-metadata-tracer.md`
- `docs/adr/0081-checked-job-string-param-setter-tracer.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Integer,String)`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#insertIndex`
