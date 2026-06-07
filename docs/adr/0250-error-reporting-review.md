# ADR 0250: Error Reporting Review

## Status

Accepted

## Context

ADR 0246 left a v1 release checklist item open for reviewing user-facing
error/reporting behavior. The current port has two distinct reporting surfaces:

- `HbciResult<T>` / `HbciError` for local execution failures such as callback,
  configuration, network, protocol, storage, invalid argument, and unsupported
  surface errors;
- `HbciExecStatus`, `HbciDialogStatus`, `HbciMsgStatus`, `HbciStatus`, and
  `HbciReturnValue` for FinTS bank return values, warnings, success messages,
  and bank-side business errors.

hbci4java also separates thrown exceptions from status/result return values. The
Rust port should keep that distinction visible while documenting how callers
should inspect the returned status after a successful transport/protocol call.

## Decision

Keep the v1 public API split:

- use `HbciError` only for failures that prevent producing a meaningful
  execution status;
- represent bank-side FinTS return codes as status data even when they describe
  failed jobs or rejected authorization;
- keep Java-near display and `error_string()` behavior on status types;
- keep `KnownReturncode` search helpers exposed through status objects for
  migration-critical cases such as invalid PIN/authentication failure and SCA
  metadata return values.

Add an error-reporting reference page that explains:

- when to handle `Result::Err`;
- when to inspect `HbciExecStatus::success` and `HbciExecStatus::is_ok()`;
- how global, segment, dialog, and job statuses differ;
- how to find known return codes and invalid PIN/authentication failures;
- why some Java-near asymmetries remain, such as `HbciMsgStatus::is_ok()` using
  the global status while `error_string()` still includes segment errors.

Add a public-API smoke test that constructs the status types through crate-root
exports and verifies the documented inspection shape.

## Consequences

The release checklist can treat user-facing error/reporting review as covered
for the current v1 hardening slice.

Applications get a documented migration rule instead of guessing whether a
bank-side rejection should be expected as `Err` or as `Ok(HbciExecStatus)`.

This does not introduce an idiomatic Rust error facade. The port remains
original-near, and later rustification can consider a richer typed diagnostic
layer only after v1 parity is stable.

## Links

- `docs/reference/error-reporting.md`
- `docs/reference/public-api.md`
- `docs/reference/java-to-rust-mapping.md`
- `docs/architecture/release-checklist.md`
- `tests/status.rs`
- `tests/public_api.rs`
- ADR 0246: V1 Release Checklist
