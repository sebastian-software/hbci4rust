# ADR 0093: Job Integer Parameter Overload

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl#setParam(String,int)` is a thin wrapper:

- convert the integer with `Integer.toString(...)`;
- delegate to `setParam(String,String)`.

In the Rust port, ADR 0081 deliberately split the string setter surface:

- `HbciJob::set_param(...)` remains permissive for early tracers;
- `HbciJob::try_set_param(...)` performs Java-like constraint checks and writes
  matching low-level parameters.

The port already has structured overloads for `Konto` and `Value`.

## Decision

Add integer convenience methods:

- `HbciJob::set_param_int(name, value)`;
- `HbciJob::try_set_param_int(name, value)`.

`set_param_int(...)` mirrors the current permissive Rust string setter and only
stores the frontend value.

`try_set_param_int(...)` mirrors the checked Rust string setter and therefore
validates the frontend name, rejects impossible empty values by construction,
and writes mapped low-level parameters on success.

Use Rust `i32`, matching Java's signed `int`.

Do not add datatype/range validation in this slice. Protocol datatype
validation remains part of message rendering and segment validation.

## Consequences

Callers can use typed integer values without hand-written string conversion.

Tests pin both paths: permissive frontend storage and checked low-level
propagation/rejection.

Remaining work:

- port date and indexed overloads;
- decide when the permissive string and integer setters should become checked
  or be deprecated;
- add datatype/range validation once job segment validation is ported.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0081-checked-job-string-param-setter-tracer.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,int)`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,String)`
