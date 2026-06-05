# ADR 0047: Account By Number Fallback Tracer

## Status

Accepted

## Context

hbci4java exposes `HBCIPassport#getAccount(String number)` as a convenience for
building a usable `Konto` when callers know only the account number.

The upstream implementation:

- creates a new `Konto`, whose constructor defaults currency to `EUR`;
- sets the requested account number;
- calls `fillAccountInfo(...)` to enrich the account from UPD accounts;
- if no matching UPD account filled the bank code, falls back to passport
  country, bank code, and customer ID;
- uses the effective customer ID as both `Konto.customerid` and `Konto.name` in
  the fallback case.

The Rust port already has `accounts()`, `first_account()`, and
`fill_account_info(...)`, but did not yet expose this Java convenience boundary.

## Decision

Add `PinTanPassport::account_by_number(...) -> Konto` as the Rust-cased port of
hbci4java `getAccount(String number)`.

Initialize the account with:

- `number` set from the caller input;
- `curr = EUR`, matching Java `new Konto()`.

Then call `fill_account_info(...)`, preserving the existing Java-near matching
behavior including leading-zero normalization for account numbers, subnumbers,
and IBANs.

If the account still has no bank code after filling, populate fallback data from
the passport:

- `blz` from `PinTanPassportData.blz` when non-empty;
- `country` from `PinTanPassportData.country` when non-empty;
- `customer_id` and `name` from the effective passport customer ID when
  non-empty.

Add `PinTanPassport::customer_id()` to centralize the hbci4java
`getCustomerId()` fallback rule: a non-empty stored customer ID wins; otherwise
the user ID is used.

Keep the method name Rust-cased (`account_by_number`) instead of `get_account`,
following the existing public API style in this crate.

## Consequences

Callers can now build a minimally usable account from a number without manually
duplicating passport fallback logic.

The fallback still does not synthesize BIC, IBAN, account type, limits, or
allowed GVs when no UPD account matches; this mirrors hbci4java's fallback
boundary.

Replay/bootstrap tests cover:

- cached-account enrichment with leading-zero-tolerant account-number matching;
- fallback to passport identity when no cached account matches.

Remaining work:

- decide whether job renderers should use `account_by_number(...)` when only
  `my.number` is provided;
- add tests for empty stored customer ID falling back to user ID;
- decide whether `Konto::default()` itself should mirror Java `new Konto()` by
  defaulting currency to `EUR`, or whether that default should stay local to
  `account_by_number(...)` for now.

## Links

- `src/passport/pintan.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.passport.HBCIPassport#getAccount`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getAccount`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getCustomerId`
- Upstream: `org.kapott.hbci.structures.Konto#Konto()`
