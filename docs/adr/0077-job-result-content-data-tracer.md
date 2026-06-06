# ADR 0077: Job Result Content Data Tracer

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl.extractPlaintextResults(...)` copies every parsed
response value under a job response segment into `HBCIJobResultImpl.resultData`.
The generated keys use:

- `content.*` for the first response segment;
- `content_2.*`, `content_3.*`, ... for later response segments.

This happens before GV-specific `extractResults(...)` overrides add richer
typed result data.

ADR 0076 added `HbciJobResult::result_data` and populated `basic.*`, but did
not copy `content.*` values.

## Decision

Populate `content.*` result data for the currently ported Saldo result paths:

- `SaldoReq` copies the matching `CustomMsgRes.GVRes[_N].SaldoRes7.*` values
  into `content.*`;
- `SaldoReqAll` copies all `CustomMsgRes.GVRes[_N].SaldoRes7.*` values into
  `content`, `content_2`, ... in response order.

Use the same counter shape as hbci4java's
`HBCIUtilsInternal.withCounter("content", idx)`.

The copied values come from the Rust parser's normalized value map, not from the
raw wire string. For example, a wire amount `123,45` is present in
`result_data` as `123.45`, matching the crate's current datatype parsing
boundary.

Keep this as a plaintext/result-data tracer. Do not port GV-specific
`extractResults(...)` overrides in this slice.

## Consequences

`HbciJobResult::result_data` now contains both `basic.*` execution metadata and
`content.*` response values for Saldo replay results.

`SaldoReqAll` now demonstrates repeated content sections via `content_2.*`.

The implementation is intentionally scoped to currently ported PinTAN Saldo
jobs. Additional jobs need their own response-root mapping or a more general
registry-backed extraction layer.

Remaining work:

- generalize response-root lookup from the job registry/protocol definitions;
- port GV-specific `extractResults(...)` overrides where they add typed or
  renamed result fields;
- decide whether any callers require raw wire-value preservation alongside
  parser-normalized values.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0076-job-result-result-data-tracer.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#extractPlaintextResults`
- Upstream: `org.kapott.hbci.manager.HBCIUtilsInternal#withCounter`
