# ADR 0090: SaldoReqAll Account Constraints

## Status

Accepted

## Context

ADR 0035 introduced a pragmatic `SaldoReqAll` renderer that can send an
all-accounts balance request without explicit account fields. That kept the
early replay tracer usable while UPD/passport fallback behavior was still thin.

hbci4java's `GVSaldoReqAll` is more specific. Its constructor adds:

- `maxentries` -> `maxentries`;
- `dummyall` -> `allaccounts` with default `J`;
- SEPA account constraints for `my.bic` and `my.iban`;
- national account constraints for `my.country`, `my.blz`, `my.number`, and
  `my.subnumber` when national account data is supported.

It also calls `checkAccountCRC("my")` during `verifyConstraints()`.

The Rust port now has constraint verification, low-level parameter persistence,
and the first async IBAN account-check path. Keeping `SaldoReqAll` with only
`dummyall` and `maxentries` means the checked queue path cannot behave like the
Java job.

## Decision

Expand the Rust `SaldoReqAll` constraint metadata to include the same account
fields that the current `Saldo7` tracer already supports:

- `my.bic` -> `Saldo7.KTV.bic`;
- `my.iban` -> `Saldo7.KTV.iban`;
- `my.country` -> `Saldo7.KTV.KIK.country` with default `DE`;
- `my.blz` -> `Saldo7.KTV.KIK.blz`;
- `my.number` -> `Saldo7.KTV.number`;
- `my.subnumber` -> `Saldo7.KTV.subnumber` with default empty string;
- `dummyall` -> `Saldo7.allaccounts` with default `J`;
- `maxentries` -> `Saldo7.maxentries` with default empty string.

Do not add `my.curr` for the current `Saldo7` request tracer. hbci4java's
`GVSaldoReqAll` has a `curr` constraint in its national branch, but the
`HKSAL7` XML request segment used by this port does not expose a request
`curr` data element.

Keep the permissive `HbciHandler::add_to_queue(...)` rendering behavior from
ADR 0035: an explicit all-accounts request without account data can still be
rendered. The stricter account requirements apply to checked queue admission.

## Consequences

`SaldoReqAll` now participates in account parameter overloads, checked
constraint verification, and the async IBAN callback check.

This moves the checked path closer to `GVSaldoReqAll.verifyConstraints()` while
preserving the early offline renderer compatibility path.

Remaining work:

- derive `SaldoReqAll` constraints from BPD/segment version data rather than a
  hard-coded Saldo7 approximation;
- revisit `my.curr` when lower Saldo segment versions or BPD-driven version
  selection enter scope;
- decide when the permissive all-accounts rendering path should be narrowed.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0035-saldo-request-all-tracer.md`
- `docs/adr/0089-async-account-check-queue-admission.md`
- Upstream: `org.kapott.hbci.GV.GVSaldoReqAll`
- Upstream: `org.kapott.hbci.GV.GVSaldoReq`
