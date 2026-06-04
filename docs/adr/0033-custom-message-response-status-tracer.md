# ADR 0033: Custom Message Response Status Tracer

## Status

Accepted

## Context

The first `HbciHandler::execute` tracer can render queued `SaldoReq` jobs into a
`CustomMsg`, but it originally treated every HTTP response below 400 as a
successful job result. hbci4java does not do that. It extracts `HIRMG` and
`HIRMS` return values into status objects and evaluates jobs from global and
segment-specific return codes.

hbci4java's `HBCIRetVal` documents the relevant code classes:

- `0xxx` success;
- `3xxx` warning;
- `9xxx` error.

`HBCIStatus` treats a status as OK when it has no errors and at least one
success or warning. `HBCIJobResultImpl.isOK()` treats a job as OK when neither
global nor job status has errors and at least one of those statuses is known.

## Decision

Parse HTTP-successful handler replay responses as `CustomMsgRes` using the
existing XML-backed wire parser.

Add a public `HbciReturnValue` struct with the original-near fields needed by
the current handler status path:

- `code`;
- `segment_ref`;
- `data_ref`;
- `text`;
- `params`.

Expose parsed global return values on `HbciExecStatus.global_return_values`,
segment return values on `HbciExecStatus.segment_return_values`, and the
segment-relevant return values on each `HbciJobResult.return_values`.

Keep the existing `messages: Vec<String>` as a flattened textual view, formatted
close to hbci4java's `HBCIRetVal.toString()`.

For this tracer, queued jobs map to outgoing segment sequences by their
`CustomMsg` position: first queued job is segment `2`, second is `3`, and so on.
Segment return values are associated with jobs by matching that sequence to the
`HIRMS` return reference exposed by the current wire-value mapper.

## Consequences

Replay tests now use valid `CustomMsgRes` bodies and can distinguish:

- HTTP success plus FinTS success;
- HTTP success plus segment-level FinTS errors.

`HbciExecStatus.success` now requires HTTP success, OK global FinTS status, and
all queued job results to be OK.

This is still a tracer. It does not yet model full hbci4java
`HBCIExecStatus`/`HBCIDialogStatus`/`HBCIMsgStatus` objects, dialog init/end
status, result payload extraction, TAN status handling, or multi-segment jobs.

## Links

- `src/gv_result/mod.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.status.HBCIRetVal`
- Upstream: `org.kapott.hbci.status.HBCIStatus`
- Upstream: `org.kapott.hbci.GV_Result.HBCIJobResultImpl`
