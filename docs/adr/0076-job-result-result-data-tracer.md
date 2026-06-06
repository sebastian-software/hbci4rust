# ADR 0076: Job Result Result Data Tracer

## Status

Accepted

## Context

hbci4java's `HBCIJobResultImpl` keeps raw result data in a `Properties` object.
That object stores:

- `content.*` keys for low-level response data;
- `basic.dialogid`;
- `basic.msgnum`;
- `basic.segnum`.

The original class also exposes:

- `storeResult(key, value)`, ignoring `null` values;
- `getDialogId()`;
- `getMsgNum()`;
- `getSegNum()`;
- `getJobId()`, formatted as `yyyyMMdd/dialogid/msgnum/segnum`;
- `toString()`, rendering sorted `key = value` lines.

ADR 0075 deliberately deferred this raw result-data layer while adding
global/job status storage.

## Decision

Add `result_data: BTreeMap<String, String>` to `HbciJobResult`.

Use `BTreeMap` rather than `Properties`/`HashMap` so display order is stable and
matches hbci4java's sorted `toString()` behavior.

Add original-near helpers:

- `store_result(...)`, ignoring `None` values;
- `dialog_id()`;
- `msg_num()`;
- `seg_num()`;
- `job_id_for_date(...)`;
- `Display` for sorted `key = value` lines.

Populate the current handler-created job results with:

- `basic.dialogid`;
- `basic.msgnum`;
- `basic.segnum`.

Do not add a live `job_id()` method yet. The Java method depends on the current
local date; the Rust port currently has no date/time dependency, and a
deterministic `job_id_for_date(...)` keeps the original string shape testable
without adding a dependency for this tracer slice.

## Consequences

Rust job results now carry the original basic execution metadata needed to
identify the dialog, message, and segment that produced the result.

Direct replay execution gets `basic.dialogid = 0`, `basic.msgnum = 1`, and the
queued segment number, matching the current request reference used by the
handler.

The sorted `Display` implementation can later include `content.*` values once
job-specific raw result storage is ported.

Remaining work:

- populate `content.*` raw result values from parsed job responses;
- decide whether to add a date crate and a live `job_id()` helper;
- attach richer parent-job/passport references only if needed for v1 behavior.

## Links

- `src/gv_result/mod.rs`
- `src/manager/handler.rs`
- `tests/status.rs`
- `tests/bootstrap.rs`
- `docs/adr/0075-job-result-status-tracer.md`
- Upstream: `org.kapott.hbci.GV_Result.HBCIJobResult`
- Upstream: `org.kapott.hbci.GV_Result.HBCIJobResultImpl`
