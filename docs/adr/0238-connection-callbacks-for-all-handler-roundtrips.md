# 0238 Emit Connection Callbacks For All Handler Roundtrips

## Status

Accepted

## Context

The async callback API includes original hbci4java-style connection lifecycle
reasons:

- `NeedConnection`;
- `CloseConnection`.

`HbciHandler::init()` already emits those callbacks around its FinTS transport
request. `HbciHandler::execute()` and `HbciHandler::close()` still send through
`CommClient` directly.

That asymmetry is visible for optional live-bank tests and for applications that
use callback events to display or monitor connection state. It also makes the
Rust handler less original-near than necessary because all three handler
operations perform the same kind of FinTS roundtrip.

## Decision

Route `init()`, `execute()`, and `close()` through a shared helper that emits:

1. `CallbackReason::NeedConnection` before sending the `CommRequest`;
2. `CallbackReason::CloseConnection` after a successful `CommClient::send`.

Keep the current error boundary: when `CommClient::send` itself fails, no
`CloseConnection` event is emitted. HTTP error responses still count as received
responses, so `CloseConnection` is emitted before the handler raises its HTTP
status error.

## Consequences

Connection lifecycle callbacks now cover dialog init, custom message execution,
and dialog end consistently.

Replay tests can assert the lifecycle without using a real network connection.

The callback API stays async-first and does not introduce Java's mutable
`StringBuffer` response channel.

## References

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0004-async-tokio-architecture.md`
- `docs/adr/0069-dialog-init-institute-message-callback-tracer.md`
- `docs/adr/0236-optional-live-bank-test-hooks.md`
