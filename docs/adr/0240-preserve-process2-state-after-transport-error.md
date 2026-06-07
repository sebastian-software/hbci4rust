# 0240 Preserve Process-2 State After Transport Error

## Status

Accepted

## Context

ADR 0239 records that bank-side FinTS rejection of the second PinTAN process-2
message keeps the short-lived SCA state so applications can retry or restart the
flow deliberately.

There is a separate failure boundary: the second message may fail before any
FinTS response is received, for example because HTTPS transport fails. In that
case the handler cannot know whether the bank accepted, rejected, or never saw
the message.

The Rust handler already treats transport errors as `HbciErrorKind::Network`
rather than `HbciExecStatus`. ADR 0039 also keeps the dialog message number from
advancing when a transport send fails because no response was received.

## Decision

When `HbciHandler::execute_tan2step_process2_submission()` hits a transport
error while sending the second process-2 message:

- return the transport error as `HbciErrorKind::Network`;
- keep the queued `TAN2Step process=2` job in the handler queue;
- keep the short-lived SCA state (`order_ref`, challenge, and HHD-UC payload);
- do not advance the dialog message number.

Do not convert the transport error into an execution status. Execution status is
reserved for responses that were actually received and parsed.

## Consequences

Applications can decide whether to retry the exact pending message, close and
restart the dialog, or ask the user to re-authorize.

The handler avoids silently discarding the order reference or TAN challenge
after a no-response failure.

This does not prove that replaying the message is always accepted by a real
bank. Optional live-bank observations and future bank-specific replay fixtures
should refine that guidance.

## References

- `docs/adr/0039-dialog-context-message-number-tracer.md`
- `docs/adr/0164-pintan-process2-tan-submission-execution.md`
- `docs/adr/0239-preserve-sca-state-after-failed-process2-submission.md`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
