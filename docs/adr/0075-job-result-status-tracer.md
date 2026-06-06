# ADR 0075: Job Result Status Tracer

## Status

Accepted

## Context

hbci4java's `HBCIJobResultImpl` stores two status objects:

- `globStatus`, for the whole message that carried the job;
- `jobStatus`, for the segment/status values belonging to this job.

It also exposes:

- `getRetNumber()`, counting only job/segment return values;
- `getRetVal(int)`, returning a job/segment return value by index;
- `isOK()`, using the combined global/job status rule:
  - neither global nor job status may be `STATUS_ERR`;
  - at least one of global or job status must be known/OK rather than both
    unknown.

The Rust port already had `HbciJobResult::job_status()` and a transitional
`is_ok_with_global_status(...)`, but individual job results did not store their
own global status values.

## Decision

Add `global_return_values` to `HbciJobResult`.

Add original-near helpers:

- `ret_number()`;
- `ret_value(...)`;
- `global_status()`;
- `job_status()`;
- `is_ok()`.

Keep `is_ok_with_global_status(...)` as a transitional helper for code paths
that still compute the global status externally.

Populate `global_return_values` in `HbciHandler::execute()` for each generated
job result.

Do not port `resultData`, `basic.dialogid`, `basic.msgnum`, `basic.segnum`,
`getJobId()`, parent-job references, or passport backreferences yet.

## Consequences

Individual Rust job results can now evaluate success using the same status rule
as hbci4java's `HBCIJobResultImpl`.

`ret_number()` and `ret_value(...)` intentionally operate only on job/segment
return values, matching the Java interface documentation.

The handler currently clones global return values into each job result. This is
acceptable for the original-near tracer stage and can be revisited once result
ownership is redesigned.

Remaining work:

- port raw `resultData`/`Properties`-like result storage if later jobs need it;
- add dialog/message/segment identifiers once request metadata is stored per
  queued job;
- decide whether `success` should become a computed compatibility field or stay
  stored in the public result structure.

## Links

- `src/gv_result/mod.rs`
- `src/manager/handler.rs`
- `tests/status.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.GV_Result.HBCIJobResult`
- Upstream: `org.kapott.hbci.GV_Result.HBCIJobResultImpl`
