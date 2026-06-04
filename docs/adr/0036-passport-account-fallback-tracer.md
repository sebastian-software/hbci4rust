# ADR 0036: Passport Account Fallback Tracer

## Status

Accepted

## Context

hbci4java treats account parameters for many jobs as optional because the
passport can expose accounts from UPD. For `SaldoReq`, the documentation says
that omitting `my` uses the first account from UPD. Result extraction also calls
`getMainPassport().fillAccountInfo(info.konto)` so partially returned account
data can be enriched from passport accounts.

The Rust port does not yet have a full UPD fetch/update pipeline, but the
Rust-native PinTAN passport format already exists and can carry local data.

## Decision

Add an `accounts: Vec<Konto>` field to `PinTanPassportData` with
`serde(default)` so older Rust-native passport payloads remain readable.

Expose original-near account helpers on `PinTanPassport`:

- `accounts()`;
- `first_account()`;
- `fill_account_info(...)`.

For the current Saldo handler tracer:

- explicit `my.*` job parameters override passport account values;
- `SaldoReq` accepts either explicit account identity or the first passport
  account;
- `SaldoReqAll` still allows an empty account, but uses the first passport
  account when one is available;
- `SaldoRes7` result extraction calls `fill_account_info` before returning
  `GvrSaldoReqInfo`.

## Consequences

`SaldoReq` can now be queued without `my.iban` when the passport has an account.
Replay tests cover the generated `HKSAL` request and result enrichment.

This is not yet a full UPD port. Missing pieces include:

- parsing `KInfo`/UPD segments into passport accounts;
- preserving TAN media and other protected UPD keys;
- `getAccount(number)` fallback behavior for unknown account numbers;
- all hbci4java `Konto` fields such as customer id, owner names, limits, and
  allowed GVs.

The current `Konto` struct still lives in `gv_result` from the first result
tracer. A later original-near cleanup should move shared structures into a
dedicated `structures` module once more result classes use them.

## Links

- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.passport.HBCIPassport`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport`
- Upstream: `org.kapott.hbci.GV.GVSaldoReq`
