# ADR 0097: Job Date Parameter Overloads

## Status

Accepted

## Context

hbci4java's date job-parameter overloads are thin wrappers:

- `HBCIJobImpl#setParam(String, Date)`;
- `HBCIJobImpl#setParam(String, Integer, Date)`.

Both convert the Java `Date` through `HBCIUtils.date2StringISO(...)`, producing
`YYYY-MM-DD`, and then delegate to the string setter. The Rust port already uses
the same external date string shape for protocol datatype parsing: incoming
`Date` values are exposed as `YYYY-MM-DD`, and outgoing `Date` data elements are
rendered from `YYYY-MM-DD` to HBCI wire `YYYYMMDD`.

The port does not yet depend on a date/time crate, and ADR 0076 deliberately
kept job-result date helpers deterministic and string-based.

## Decision

Add checked date job-parameter helpers:

- `HbciJob::try_set_param_date(name, yyyy_mm_dd)`;
- `HbciJob::try_set_indexed_param_date(name, index, yyyy_mm_dd)`.

The helpers:

- accept ISO date strings in the same `YYYY-MM-DD` shape produced by
  hbci4java's `HBCIUtils.date2StringISO(...)`;
- validate the date through the protocol datatype boundary, including leap-day
  checks;
- normalize surrounding whitespace away;
- delegate to the existing checked string setter or checked indexed string
  setter after validation.

Do not introduce a public Rust date type or a date/time dependency in this
slice. A typed date API can be considered later once more upstream date-bearing
jobs and result structures are ported.

Do not add permissive `set_param_date(...)` in this slice. Unlike Java's `Date`
argument, a Rust string can be malformed; the checked `try_*` surface makes that
failure explicit while preserving the original low-level string shape.

## Consequences

Date-bearing jobs can now avoid hand-written ISO string validation while still
staying close to hbci4java's low-level values.

The indexed date helper closes the remaining indexed structured setter noted in
ADR 0087, ADR 0094, ADR 0095, and ADR 0096.

Remaining work:

- apply date helpers to concrete jobs such as statement/payment range requests
  as their constraints enter the port;
- decide whether to add a typed date API after more original behavior is
  covered by tests;
- port segment validation after low-level propagation.

## Links

- `src/gv/mod.rs`
- `src/protocol/datatype.rs`
- `tests/bootstrap.rs`
- `docs/adr/0076-job-result-result-data-tracer.md`
- `docs/adr/0087-indexed-job-param-setter-tracer.md`
- `docs/adr/0093-job-integer-param-overload.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Date)`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Integer,Date)`
- Upstream: `org.kapott.hbci.manager.HBCIUtils#date2StringISO`
