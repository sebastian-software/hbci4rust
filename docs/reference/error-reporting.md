# Error Reporting

Snapshot date: 2026-06-07.

This page explains how v1 callers should read errors and status reports in the
original-near PinTAN/HBCI-Plus API.

## Rule Of Thumb

`HbciResult<T>` is for local execution failures. A returned `HbciError` means
the library could not complete the requested operation well enough to produce
the normal result value.

Examples:

- callback input was missing or rejected;
- configuration was invalid;
- transport failed;
- protocol parsing or validation failed;
- passport storage failed;
- caller input was invalid;
- the requested hbci4java surface is outside v1 scope.

Bank-side FinTS return values are not automatically converted into
`HbciError`. When a bank rejects a job, asks for stronger authorization, reports
an invalid PIN, or returns warnings, inspect the returned status object.

## Main Status Types

The public status surface mirrors hbci4java concepts:

- `HbciExecStatus` is the result of handler execution.
- `HbciDialogStatus` groups init, business-message, and dialog-end statuses.
- `HbciMsgStatus` groups global and segment status for one FinTS message.
- `HbciStatus` groups return values and exception messages for one status
  level.
- `HbciReturnValue` represents one bank return code, text, references, params,
  and optional source element.

Application code should first handle `Result::Err`, then inspect the returned
`HbciExecStatus`.

```rust
# use hbci4rust::{HbciExecStatus, HbciResult};
# async fn execute() -> HbciResult<HbciExecStatus> { Ok(HbciExecStatus::default()) }
# async fn run() -> HbciResult<()> {
let status = execute().await?;

if !status.success {
    let message = status.error_string();
    // Show or log message, then inspect status/job return values.
}
# Ok(())
# }
```

## `success` And `is_ok()`

`HbciExecStatus::success` is the handler's computed execution flag for the
message sequence that was run.

`HbciExecStatus::is_ok()` is original-near:

- if dialog status data is present, it evaluates every known customer dialog;
- otherwise it falls back to the flat `success` field.

`HbciMsgStatus::is_ok()` follows hbci4java's message-status rule and checks the
global status. Segment errors can still appear in `error_string()` and job
statuses.

That asymmetry is intentional. It keeps Java migration behavior visible instead
of hiding segment-level bank diagnostics behind a single Rust error.

## Return-Code Search

Known return codes are exposed through `KnownReturncode` and search helpers on
status objects:

```rust
# use hbci4rust::{HbciExecStatus, KnownReturncode};
# let status = HbciExecStatus::default();
let tan_method_values = status.return_values_for_code(KnownReturncode::W3920);
let invalid_pin = status.invalid_pin_code();
```

Use these helpers for migration-critical PinTAN cases such as:

- `3920`: available TAN methods;
- `3956`: decoupled authorization still pending;
- `9340`, `9930`, `9931`, `9942`: authentication/PIN failure family.

The underlying `HbciReturnValue` is still available so callers can log the exact
bank text and references.

## Job Status

Each `HbciJobResult` stores:

- global return values copied from the message;
- job/segment return values for the business job;
- optional typed result data;
- raw result data keys for original-near migration.

Use `HbciJobResult::is_ok()` or `is_ok_with_global_status(...)` when checking a
single job result. Use `ret_number()` and `ret_value(...)` when porting code
that iterated hbci4java job return values.

## Display And Diagnostics

`Display` and `error_string()` intentionally keep Java-near, compact text:

- `HbciReturnValue` renders as `code:text` plus params and references;
- `HbciStatus::error_string()` includes exceptions and error return values;
- `HbciMsgStatus::error_string()` includes global and segment error strings;
- `HbciExecStatus::error_string()` uses dialog/customer grouping when dialog
  data exists.

For structured handling, prefer the fields and helper methods over parsing
display text.
