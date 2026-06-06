# ADR 0066: Known Returncode Status Search Tracer

## Status

Accepted

## Context

hbci4java's `KnownReturncode` has search helpers that find matching
`HBCIRetVal` values:

- `searchReturnValues(HBCIRetVal[])`;
- `searchReturnValues(HBCIMsgStatus)`;
- `searchReturnValue(HBCIRetVal[])`.

The Rust port currently has:

- `KnownReturncode` in `src/dialog/mod.rs`;
- `HbciStatus` for one list of return values;
- a flat `HbciExecStatus` containing global and segment return values.

Putting return-value search methods directly on `KnownReturncode` would make the
`dialog` module depend on `gv_result`, while `gv_result` already depends on
`KnownReturncode` for invalid-PIN detection.

## Decision

Add known-returncode search helpers on the status types:

- `HbciStatus::return_values_for_code(...)`;
- `HbciStatus::return_value_for_code(...)`;
- `HbciExecStatus::return_values_for_code(...)`;
- `HbciExecStatus::return_value_for_code(...)`.

Keep `KnownReturncode` code-oriented for now.

Search `HbciExecStatus` in the original message-status order:

1. global return values;
2. segment return values.

Keep `HbciExecStatus::invalid_pin_code()` using the auth-failure code list and
the original error-only search rule.

Do not introduce `HbciMsgStatus` in this slice.

## Consequences

Callers can now perform original-near returncode lookup without manually
iterating over raw vectors.

The module boundary stays simple until the Java status hierarchy is ported more
fully.

Tests pin multi-hit search, first-hit search, missing-code behavior, and
global-before-segment ordering.

Remaining work:

- move these helpers onto an explicit `HbciMsgStatus` type if that type is
  introduced;
- decide whether `KnownReturncode` should expose additional search helpers once
  the module hierarchy can support it cleanly;
- extend known-returncode use for SCA/TAN flows such as `3920` when the runtime
  needs it.

## Links

- `src/dialog/mod.rs`
- `src/gv_result/mod.rs`
- `tests/status.rs`
- `docs/adr/0065-known-returncode-auth-fail-tracer.md`
- Upstream: `org.kapott.hbci.dialog.KnownReturncode#searchReturnValues`
- Upstream: `org.kapott.hbci.dialog.KnownReturncode#searchReturnValue`
