# ADR 0050: Konto Equality Tracer

## Status

Accepted

## Context

hbci4java's `Konto.equals(...)` does not compare every public field on
`Konto`. It compares only:

- `blz`;
- `country`;
- `number`;
- `subnumber`;
- `curr`;
- `customerid`;
- `name`;
- `name2`;
- `type`;
- `bic`;
- `iban`.

It does not compare `acctype`, `limit`, `allowedGVs`, or `creditorid`.

The Rust port initially derived `PartialEq`/`Eq` for `Konto`, which compared all
Rust fields structurally. That was tidy Rust, but not original-near behavior.
Recent tracers added `limit` and `allowed_gvs`, making the difference more
visible.

## Decision

Replace derived `PartialEq` for `Konto` with a manual implementation matching
hbci4java `Konto.equals(...)`.

Keep `Eq` implemented for `Konto` because the selected comparison fields are
all equality-stable optional strings.

Compare Rust `Konto.account_type` as the port of Java `Konto.type`.

Ignore these fields in `Konto` equality for now:

- `acctype`;
- `limit`;
- `allowed_gvs`.

Do not add a separate strict structural equality helper in this slice.

## Consequences

Rust `Konto` equality is now original-near rather than pure structural equality.
This can be surprising, but it better preserves hbci4java behavior for account
matching and account collections.

Tests cover ignored fields (`acctype`, `limit`, `allowed_gvs`) and one compared
field (`iban`).

Remaining work:

- decide whether a future `Konto::strict_eq(...)` helper is useful for storage
  roundtrip tests or diagnostics;
- revisit equality if `creditorid` is added to the Rust `Konto` model;
- document the equality semantics in public API docs once those docs are added.

## Links

- `src/gv_result/mod.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.structures.Konto#equals`
- ADR 0045: Account Allowed GV Cache Tracer
- ADR 0046: Account Limit Cache Tracer
