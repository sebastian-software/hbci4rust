# ADR 0061: Handler Status Helper Use Tracer

## Status

Accepted

## Context

ADR 0059 introduced `HbciStatus` as the Rust port of hbci4java's
`HBCIStatus` grouping behavior. ADR 0060 added computed helpers for viewing
existing execution and job result return values as `HbciStatus`.

`HbciHandler::execute(...)` still used local boolean helper functions that
duplicated parts of that status logic:

- `status_is_ok(...)`;
- `status_pair_is_ok(...)`.

Keeping duplicate rules makes later status refactors riskier because the
handler can drift away from the original-near status model.

## Decision

Use `HbciStatus` in the handler's success computation.

In `ParsedResponseStatus`:

- compute global status with `HbciStatus::from_return_values(...)`;
- let `global_is_ok()` delegate to `HbciStatus::is_ok()`.

For queued job results:

- build the `HbciJobResult` with its segment return values;
- compute `success` through
  `HbciJobResult::is_ok_with_global_status(...)`.

For dialog end validation:

- use `HbciStatus::from_return_values(...).is_ok()`.

Remove the local duplicate status helper functions from the handler.

Do not store `HbciStatus` directly in `HbciExecStatus` or `HbciJobResult` yet.
Do not change parsing, return-value extraction, or the public result shape in
this slice.

## Consequences

The PinTAN handler now uses the same original-near status grouping boundary as
the public result helpers.

Existing replay tests continue to cover the behavior while the implementation
has less duplicate status logic.

Remaining work:

- decide whether execution and job results should store grouped status values;
- decide how HTTP/network errors should map into `HbciStatus` exception
  messages;
- port richer `HBCIMsgStatus` and `HBCIExecStatus` behavior when the dialog
  model grows.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.GV_Result.HBCIJobResultImpl#isOK`
- Upstream: `org.kapott.hbci.status.HBCIStatus`
- ADR 0059: Status Grouping Tracer
- ADR 0060: Computed Status Helpers Tracer
