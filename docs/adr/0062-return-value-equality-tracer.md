# ADR 0062: Return Value Equality Tracer

## Status

Accepted

## Context

hbci4java's `HBCIRetVal.equals(...)` does not compare every public field.

The upstream implementation compares:

- `code`;
- `text`;
- `segref`;
- `deref`.

It ignores:

- `params`;
- `element`.

The Rust port initially derived `PartialEq` and `Eq` for `HbciReturnValue`,
which made equality structural and included `params`.

## Decision

Replace derived `PartialEq` for `HbciReturnValue` with a manual implementation
matching hbci4java's selected fields.

Compare:

- `code`;
- `text`;
- `segment_ref`;
- `data_ref`.

Ignore:

- `params`.

Keep `Eq` implemented because the selected fields have total equality.

Do not add hbci4java's optional `element` field in this slice.

## Consequences

`HbciReturnValue` equality is now original-near rather than pure structural
Rust equality.

Status vectors and tests that compare return values inherit this original-near
comparison.

Remaining work:

- decide whether to add the upstream `element` annotation later;
- decide whether a strict structural comparison helper is useful for tests or
  serialization audits.

## Links

- `src/gv_result/mod.rs`
- `tests/status.rs`
- Upstream: `org.kapott.hbci.status.HBCIRetVal#equals`
