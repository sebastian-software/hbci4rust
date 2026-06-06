# ADR 0092: Job Value Parameter Overload

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl` has structured `setParam(...)` overloads. The
`Value` overload maps a money value object to ordinary frontend parameters:

- `<base>.value`;
- `<base>.curr`.

The Java implementation checks `acceptsParam(...)` for both derived names. It
always writes the amount when the value parameter is accepted, and writes the
currency only when it is accepted and non-empty.

The Rust port already has:

- `Value { value, curr }`;
- `HbciJob::set_param_account(...)`;
- frontend-to-lowlevel constraint persistence.

ADR 0079 left the `Value` overload for later because no value-parameter job had
entered the rendered runtime.

## Decision

Add `HbciJob::set_param_value(base, value)`.

The helper:

- derives `<base>.value` and `<base>.curr`;
- checks `accepts_param(...)` before setting either field;
- ignores empty amount and currency strings;
- writes both frontend and low-level parameters through the existing constraint
  mapping path.

Use `Value.value` directly as the Rust money amount string. Do not add a new
normalization step in this slice; amount normalization remains a separate data
model concern.

Do not port indexed `Value` setting in this slice. Indexed behavior needs a job
with indexed value constraints or a separate focused tracer.

## Consequences

Future payment and limit jobs can be parameterized with Rust `Value` structures
using the same frontend field names as hbci4java.

Tests use a synthetic constraint table to prove amount/currency mapping without
claiming that a concrete payment GV has been rendered yet.

Remaining work:

- port indexed `setParam(String, Integer, Value)`;
- decide whether Rust `Value` should expose an explicit canonical amount helper;
- apply this overload to concrete payment jobs as they enter scope.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0079-job-account-param-overload-tracer.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Value)`
- Upstream: `org.kapott.hbci.structures.Value`
