# ADR 0060: Computed Status Helpers Tracer

## Status

Accepted

## Context

hbci4java's `HBCIJobResultImpl` stores separate global and job-local
`HBCIStatus` objects. Its `isOK()` rule is:

- global status is not `STATUS_ERR`;
- job status is not `STATUS_ERR`;
- at least one of global or job status is not `STATUS_UNKNOWN`.

The Rust port already stores return values on:

- `HbciExecStatus.global_return_values`;
- `HbciExecStatus.segment_return_values`;
- `HbciJobResult.return_values`.

ADR 0059 introduced `HbciStatus`, but did not connect it to existing execution
or job result values.

## Decision

Add computed helpers instead of changing stored data:

- `HbciExecStatus::global_status()`;
- `HbciExecStatus::segment_status()`;
- `HbciJobResult::job_status()`;
- `HbciJobResult::is_ok_with_global_status(...)`;
- `HbciStatus::from_return_values(...)`.

Keep the helpers clone-based for now. The stored public structs remain
unchanged, so this slice does not force a handler refactor or serialization
change.

Implement `is_ok_with_global_status(...)` with the hbci4java `HBCIJobResultImpl`
rule: neither global nor job status is `Error`, and at least one side is not
`Unknown`.

Do not replace `HbciExecStatus.success` or `HbciJobResult.success` in this
slice. Do not wire `HbciStatus` into the dialog handler yet.

## Consequences

Callers and tests can now view existing return value vectors through the
original-near status grouping model.

The next handler refactor can compare stored boolean success fields against
computed status values without changing all call sites at once.

Remaining work:

- decide whether `HbciExecStatus` should store `HbciStatus` directly;
- decide whether job results should carry both global and job status like
  hbci4java;
- revisit serialization compatibility before changing public result structs;
- add display support for complete execution status once grouping is wired.

## Links

- `src/gv_result/mod.rs`
- `tests/status.rs`
- Upstream: `org.kapott.hbci.GV_Result.HBCIJobResultImpl#isOK`
- ADR 0059: Status Grouping Tracer
