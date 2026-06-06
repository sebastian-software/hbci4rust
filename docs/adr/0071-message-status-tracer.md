# ADR 0071: Message Status Tracer

## Status

Accepted

## Context

hbci4java has `HBCIMsgStatus` for exactly one message exchange. It contains:

- `globStatus`;
- `segStatus`.

It has some intentionally asymmetric behavior:

- `isOK()` checks only `globStatus.getStatusCode() == STATUS_OK`;
- `hasExceptions()` checks only global exceptions;
- `getErrorString()` combines global and segment error strings;
- `toString()` combines global and segment displays;
- invalid-PIN detection searches global errors first, then segment errors.

ADR 0064 previously added HBCI-message-like display and error-string behavior
directly on the current flat `HbciExecStatus`, while deferring an explicit Rust
message-status type.

## Decision

Add `HbciMsgStatus` with Rust-cased public fields:

- `global_status`;
- `segment_status`.

Add original-near helpers:

- `new()`;
- `from_statuses(...)`;
- `has_exceptions()`;
- `is_ok()`;
- `error_string()`;
- `Display`;
- `is_invalid_pin()`;
- `invalid_pin_code()`;
- known-returncode search helpers.

Add `HbciExecStatus::message_status()` to expose the current flat execution
result as a message-status view.

Keep `HbciExecStatus::success` and job success computation unchanged.

## Consequences

The Rust port now has an explicit first-level equivalent for hbci4java's
`HBCIMsgStatus`.

The asymmetric `is_ok()` rule is pinned in tests: a segment error does not make
`HbciMsgStatus::is_ok()` false if the global status is OK.

`HbciExecStatus` can keep serving as the current handler result while exposing a
more original-near message-status aggregate.

Remaining work:

- use `HbciMsgStatus` in parser/handler internals where it reduces duplication;
- introduce `HbciDialogStatus` when dialog init, job messages, and dialog end
  need an original-near aggregate;
- introduce a true multi-customer execution status only if v1 needs that
  hbci4java layer.

## Links

- `src/gv_result/mod.rs`
- `tests/status.rs`
- `docs/adr/0064-exec-status-message-display-tracer.md`
- Upstream: `org.kapott.hbci.status.HBCIMsgStatus`
