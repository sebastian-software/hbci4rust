# ADR 0059: Status Grouping Tracer

## Status

Accepted

## Context

hbci4java's `HBCIStatus` groups `HBCIRetVal` values and exceptions.

The upstream status object can:

- report whether exceptions, errors, warnings, or successes exist;
- return grouped return values;
- compute one of `STATUS_OK`, `STATUS_UNKNOWN`, or `STATUS_ERR`;
- render exceptions first, then errors, warnings, and successes;
- render only exceptions and errors through `getErrorString()`.

The Rust port already has `HbciReturnValue` and can display individual return
values. It does not yet have a status grouping type, and Rust exceptions are not
represented like Java exceptions.

## Decision

Add:

- `HbciStatus`;
- `HbciStatusCode`.

Expose Rust-cased methods for the upstream behavior:

- `has_exceptions`;
- `has_errors`;
- `has_warnings`;
- `has_success`;
- `errors`;
- `warnings`;
- `successes`;
- `status_code`;
- `is_ok`;
- `error_string`.

Use `HbciStatusCode::Ok`, `Unknown`, and `Error` as Rust enum variants, while
preserving the original numeric constants through `STATUS_OK`,
`STATUS_UNKNOWN`, `STATUS_ERR`, and `original_code()`.

Represent exceptions as already formatted `exception_messages: Vec<String>`.
This keeps the status tracer useful without introducing a Java-like exception
hierarchy or localized exception formatting.

Implement `Display` for `HbciStatus` in upstream order:

1. exception messages;
2. errors;
3. warnings;
4. successes.

Use `\n` as the stable Rust line separator.

Do not integrate `HbciStatus` into `HbciExecStatus`, `HbciJobResult`, or the
dialog handler in this slice.

## Consequences

The port now has an original-near status grouping boundary for later handler
and result refactors.

Tests pin grouping, status-code precedence, display ordering, error-string
rendering, and the original numeric status constants.

Remaining work:

- decide how `HbciExecStatus` and `HbciJobResult` should expose grouped status;
- decide whether Rust errors should be stored as strings or richer structured
  values;
- port `HBCIMsgStatus`, `HBCIDialogStatus`, and `HBCIExecStatus` display
  behavior when the dialog model is richer.

## Links

- `src/gv_result/mod.rs`
- `src/lib.rs`
- `tests/status.rs`
- Upstream: `org.kapott.hbci.status.HBCIStatus`
- Upstream: `org.kapott.hbci.status.HBCIRetVal`
