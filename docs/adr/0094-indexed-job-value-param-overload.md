# ADR 0094: Indexed Job Value Parameter Overload

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl#setParam(String,Integer,Value)` maps a structured
money value to indexed frontend parameters:

- `<base>.value`;
- `<base>.curr`.

For each derived frontend name, Java first checks `acceptsParam(...)`. Accepted
fields are delegated to the indexed string setter. Unknown fields are ignored.
Accepted but non-indexed fields fail in the indexed string setter.

The Rust port already has:

- `HbciJob::try_set_indexed_param(...)`;
- `HbciJob::set_param_value(...)`;
- `Value { value, curr }`.

ADR 0092 deliberately left indexed `Value` setting for a focused tracer.

## Decision

Add `HbciJob::try_set_indexed_param_value(base, index, value)`.

The helper:

- derives `<base>.value` and `<base>.curr`;
- ignores derived names that are not accepted by the job;
- ignores empty amount and currency strings, matching the Rust
  `set_param_value(...)` boundary;
- delegates accepted non-empty fields to `try_set_indexed_param(...)`;
- returns the same indexed-parameter errors as the string setter when an
  accepted field is not indexed.

Do not store plain frontend map entries for indexed values. Indexed parameters
are represented as indexed low-level paths, matching ADR 0087.

## Consequences

Repeated value groups can now be populated with Rust `Value` structures once
jobs with indexed value constraints enter the port.

Synthetic tests pin successful amount/currency mapping, ignored optional
fields, and non-indexed rejection.

Remaining work:

- port indexed `Konto` and date helpers;
- port `verifyConstraints()` indexed lookup for index `0`;
- apply indexed value parameters to concrete SEPA or batch jobs once their
  constraints are ported.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0087-indexed-job-param-setter-tracer.md`
- `docs/adr/0092-job-value-param-overload.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Integer,Value)`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Integer,String)`
