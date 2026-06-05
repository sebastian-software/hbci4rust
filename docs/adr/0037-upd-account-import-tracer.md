# ADR 0037: UPD Account Import Tracer

## Status

Accepted

## Context

ADR 0036 added a Rust-native passport account cache and used it as a fallback
for `SaldoReq`. That still left account population manual.

hbci4java populates passport accounts from UPD `KInfo` segments. In
`AbstractHBCIPassport.getAccounts()`, it maps `KInfo.KTV`, `customerid`,
`acctype`, `cur`, `name1`, `name2`, `konto`, `KTV.bic`, and `KTV.iban` or direct
`iban` into `Konto`.

`HBCIUser.updateUPD(...)` also preserves previously known BIC/IBAN values when a
new UPD response omits them but the account number, BLZ, and country still
match. The upstream comment notes that some banks no longer send those values
reliably in every UPD response.

## Decision

Add `PinTanPassport::update_accounts_from_values(...)`.

The method imports accounts from an already mapped flat message-value map under
a caller-provided prefix, currently intended for `DialogInitRes.UPD`.

For this tracer it supports repeated:

- `KInfo.KTV.number`;
- `KInfo.KTV.subnumber`;
- `KInfo.KTV.KIK.country`;
- `KInfo.KTV.KIK.blz`;
- `KInfo.KTV.bic` or `KInfo.bic`;
- `KInfo.KTV.iban` or `KInfo.iban`;
- `KInfo.customerid`;
- `KInfo.acctype`;
- `KInfo.cur`;
- `KInfo.name1`;
- `KInfo.name2`;
- `KInfo.konto`.

Extend `Konto` with the original-near optional fields needed by this import:
`customer_id`, `name`, `name2`, and `acctype`.

If imported KInfo accounts omit IBAN or BIC, preserve those fields from existing
passport accounts when number, BLZ, and country match.

## Consequences

The port now has the first end-to-end bridge from XML-backed `DialogInitRes` UPD
mapping to stored PinTAN passport accounts.

Replay tests parse a real `HIUPD` segment through the protocol mapper and import
it into `PinTanPassportData.accounts`.

This is still not a full UPD engine:

- the handler does not yet run authenticated dialog initialization;
- UPA metadata and UPD version handling are not stored;
- TAN media and protected UPD keys are not imported;
- limits and allowed GVs are not mapped yet;
- no live bank persistence flow is wired to this method.

## Links

- `src/passport/pintan.rs`
- `src/gv_result/mod.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getAccounts`
- Upstream: `org.kapott.hbci.manager.HBCIUser#updateUPD`
