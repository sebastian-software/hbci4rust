# ADR 0055: Konto Default Currency Tracer

## Status

Accepted

## Context

hbci4java's `org.kapott.hbci.structures.Konto` no-argument constructor sets
`curr` to `EUR`.

The Rust port originally derived `Default` for `Konto`, which left all optional
fields unset, including the currency. Earlier slices worked around that at
specific call sites such as `PinTanPassport::account_by_number(...)`.

ADR 0047 and ADR 0051 both left open whether `Konto::default()` itself should
mirror Java's `new Konto()` constructor.

## Decision

Replace derived `Default` for `Konto` with a manual implementation.

Set:

- `curr = Some("EUR")`;
- all other optional account fields to `None`;
- `limit` to `None`;
- `allowed_gvs` to an empty vector.

Keep existing struct literals that parse protocol values explicit. They should
still preserve absent protocol data instead of inheriting default currency.

## Consequences

`Konto::default()` now behaves like hbci4java's no-argument constructor for the
currency field.

Call sites that need a blank account can still override `curr` explicitly.

The display of a default `Konto` is now ` (EUR)`, matching the existing
original-near display implementation plus the constructor default.

Remaining work:

- add Java golden tests for additional `Konto` constructor shapes if they become
  public Rust helpers;
- decide whether account parsers should ever fill missing protocol currency
  from `Konto::default()` or continue preserving absence.

## Links

- `src/gv_result/mod.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.structures.Konto#Konto()`
- ADR 0047: Account By Number Fallback Tracer
- ADR 0051: Konto Display Tracer
