# ADR 0065: Known Returncode Auth Fail Tracer

## Status

Accepted

## Context

hbci4java has `org.kapott.hbci.dialog.KnownReturncode` for small lists of
well-known FinTS return codes.

`HBCIMsgStatus#getInvalidPINCode()` uses
`KnownReturncode.LIST_AUTH_FAIL` to detect the return codes interpreted as
"PIN wrong" or authentication failure:

- `9340`;
- `9930`;
- `9931`;
- `9942`.

The Rust port currently has a flat `HbciExecStatus` instead of explicit
`HBCIMsgStatus` and `HBCIDialogStatus` types.

## Decision

Add a Rust `KnownReturncode` enum in `src/dialog/mod.rs`, matching the upstream
dialog package location.

Port the upstream enum variants that exist in the pinned hbci4java reference and
add the original auth-failure list as `KnownReturncode::LIST_AUTH_FAIL`.

Add helper methods matching the upstream shape:

- `code()`;
- `is(...)`;
- `find(...)`;
- `contains(...)`.

Add `HbciExecStatus::invalid_pin_code()` and `HbciExecStatus::is_invalid_pin()`
for the current flat status model.

Scan global errors before segment errors, matching
`HBCIMsgStatus#getInvalidPINCode()`.

Do not yet introduce a full `HBCIMsgStatus` type. That requires a broader status
hierarchy decision.

## Consequences

The PinTAN runtime can recognize the upstream authentication-failure return
codes without embedding raw string lists in handler logic.

The current helper returns an `HbciReturnValue` reference, like hbci4java returns
the matching `HBCIRetVal`.

Tests pin the auth-failure list, empty-code behavior, non-auth error behavior,
and global-before-segment scan order.

Remaining work:

- move this behavior onto an explicit `HbciMsgStatus` type if the Java status
  hierarchy is introduced;
- port additional known-returncode search helpers only when callers need them;
- decide how callback-facing PinTAN errors should expose the matched return
  value.

## Links

- `src/dialog/mod.rs`
- `src/gv_result/mod.rs`
- `tests/status.rs`
- Upstream: `org.kapott.hbci.dialog.KnownReturncode`
- Upstream: `org.kapott.hbci.status.HBCIMsgStatus#getInvalidPINCode`
