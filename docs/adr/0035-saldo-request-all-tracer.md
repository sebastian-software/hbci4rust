# ADR 0035: Saldo Request All Tracer

## Status

Accepted

## Context

`SaldoReqAll` is in the PinTAN job registry but was still rejected by
`HbciHandler::execute` because only `SaldoReq` had a low-level renderer.

hbci4java implements `GVSaldoReqAll` as a subclass of `GVSaldoReq`. It uses the
same low-level `Saldo` GV and the same `GVRSaldoReq` result class, but defaults
the `allaccounts` parameter to `J`.

The Java docs describe the `my` account parameter as optional. If it is absent,
hbci4java can use UPD/passport account data. That UPD fallback is not ported
yet.

## Decision

Render `SaldoReqAll` through the existing `Saldo7`/`HKSAL` tracer:

- default `dummyall` / `allaccounts` to `J`;
- allow missing account fields, producing an empty `KTV` group before
  `allaccounts`;
- still copy optional `my.iban`, `my.bic`, `dummyall`, and `maxentries` when
  supplied.

Reuse `HbciJobResultData::SaldoReq(GvrSaldoReq)` for `SaldoReqAll`, matching the
upstream `GVRSaldoReq` result class reuse.

For the current tracer, a single queued `SaldoReqAll` collects all
`CustomMsgRes.GVRes[_n].SaldoRes7` entries from the response into one
`GvrSaldoReq`.

## Consequences

Replay tests now cover a `SaldoReqAll` request without account parameters and a
multi-account `HISAL` response.

This lifts the `SaldoReqAll with multiple accounts from one queued job`
limitation recorded in ADR 0034 for the simple single-queued-job case.

Mixed messages that combine `SaldoReqAll` with other queued GVs still need a
proper response-correlation layer before they can be modeled with hbci4java
parity.

UPD/passport account fallback is still not ported.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- ADR 0034: Saldo Request Result Tracer
- Upstream: `org.kapott.hbci.GV.GVSaldoReqAll`
- Upstream: `org.kapott.hbci.GV.GVSaldoReq`
- Upstream: `org.kapott.hbci.GV_Result.GVRSaldoReq`
