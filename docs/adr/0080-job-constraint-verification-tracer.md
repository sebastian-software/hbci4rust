# ADR 0080: Job Constraint Verification Tracer

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl.verifyConstraints()` walks the job constraint table
before a queued job segment is rendered. For each high-level frontend parameter
it resolves the mapped low-level destination, uses a non-empty caller-provided
value when present, otherwise applies the configured default, and reports an
invalid job when neither value nor default exists.

The Java method also mutates the internal low-level parameter store, handles
indexed constraints, validates that the concrete segment can be created, and GV
subclasses such as `GVSaldoReq` perform account CRC callback checks after the
base verification.

The Rust port currently has frontend job parameters and public constraint
metadata, but it does not yet maintain a separate persistent low-level parameter
map.

## Decision

Add `HbciJob::verify_constraints()`.

For this tracer the method:

- returns a resolved low-level parameter map instead of mutating hidden state;
- uses the existing `HbciJobConstraint.destination_name` values;
- applies non-empty default values when the frontend parameter is absent or
  empty;
- omits empty defaults from the returned map;
- returns `HbciErrorKind::InvalidArgument` for missing required frontend
  parameters.

Keep queued-job rendering behavior unchanged in this slice. The Saldo renderer
still follows ADR 0036's passport-account fallback because that was a deliberate
runtime tracer decision and is not the same thing as Java's internal low-level
parameter store.

## Consequences

The Rust port now has an explicit original-near entry point corresponding to
`HBCIJobImpl.verifyConstraints()` for the part that can be verified with the
current job model.

Tests cover Saldo frontend-to-lowlevel resolution, default application,
empty-default omission, and missing-required-parameter errors.

Remaining work:

- introduce a persistent low-level parameter store once message rendering moves
  fully through constraints;
- wire verification into queued-job execution at the same lifecycle point as
  hbci4java's `HBCIDialog`;
- port indexed constraints and multi-destination mappings;
- port segment creation validation;
- port `GVSaldoReq.checkAccountCRC("my")` with async callback semantics.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0078-job-constraint-metadata-tracer.md`
- `docs/adr/0079-job-account-param-overload-tracer.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#verifyConstraints`
- Upstream: `org.kapott.hbci.GV.GVSaldoReq#verifyConstraints`
