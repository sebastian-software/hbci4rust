# ADR 0034: Saldo Request Result Tracer

## Status

Accepted

## Context

`HbciHandler::execute` can now render queued `SaldoReq` jobs and evaluate
`CustomMsgRes` return statuses. The next original-near step is to expose the
actual `HISAL` response payload in a shape that resembles hbci4java's
`GVRSaldoReq`.

Upstream `GVSaldoReq.extractResults(...)` builds a `GVRSaldoReq.Info` entry from
the low-level `SaldoRes` paths:

- `KTV` into `Konto`;
- `booked` into the mandatory booked `Saldo`;
- optional `pending` into the unbooked/pending `Saldo`;
- optional `kredit`, `available`, and `used` into `Value` fields.

For debit balances, hbci4java prefixes the parsed amount with `-` before
creating a `Value`.

## Decision

Add first typed result structs in `src/gv_result/mod.rs`:

- `HbciJobResultData::SaldoReq(GvrSaldoReq)`;
- `GvrSaldoReq`;
- `GvrSaldoReqInfo`;
- `Konto`;
- `Saldo`;
- `Value`.

Keep values string-backed for this tracer. The protocol datatype parser has
already normalized wire values such as dates and decimal amounts, and we do not
yet want to commit to a public decimal/date-time type before more upstream
result classes are ported.

Map `SaldoReq` responses by queue order: the first queued `SaldoReq` reads
`CustomMsgRes.GVRes.SaldoRes7`, the second reads
`CustomMsgRes.GVRes_2.SaldoRes7`, and so on.

Keep the struct fields close to hbci4java, with one Rust naming adjustment:
`Konto.type` becomes `Konto.account_type` because `type` is a Rust keyword.
Serde still serializes it as `type`.

## Consequences

Replay tests can now assert real balance payloads, including debit sign handling
and optional `pending`, `kredit`, `available`, and `used` fields.

The tracer does not yet cover:

- `SaldoReqAll` with multiple accounts from one queued job;
- BPD-driven segment-version/result-version selection;
- `Konto` enrichment from passport UPD data;
- Java-like `Value` cent-integer or public date-time APIs.

Those should be ported once more `GV_Result` classes reveal a stable pattern.

## Links

- `src/gv_result/mod.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.GV.GVSaldoReq`
- Upstream: `org.kapott.hbci.GV_Result.GVRSaldoReq`
- Upstream: `org.kapott.hbci.structures.Konto`
- Upstream: `org.kapott.hbci.structures.Saldo`
- Upstream: `org.kapott.hbci.structures.Value`
