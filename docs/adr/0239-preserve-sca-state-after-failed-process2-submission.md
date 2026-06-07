# 0239 Preserve SCA State After Failed Process-2 Submission

## Status

Accepted

## Context

ADR 0164 introduced explicit execution of the second PinTAN process-2 message:
after a first message containing the business job plus `HKTAN process=4`, the
bank returns `HITAN` state with an order reference and challenge. The second
message sends `HKTAN process=2` and carries the TAN in the PinTAN user
signature.

ADR 0164 deliberately left one failure boundary open: whether a failed
`process=2` submission should clear or keep the short-lived SCA state.

The current Rust handler clears SCA state only when the `process=2` submission
returns a successful execution status. This is useful for retries because the
stored `orderref`, challenge, and HHD-UC payload are still available after a
bank-side rejection.

The hbci4java SCA dialog path similarly treats the second SCA step as complete
only after the final response succeeds; otherwise the step-2 state remains
retryable in the dialog metadata.

## Decision

Keep the SCA state after a failed process-2 TAN submission.

For `HbciHandler::execute_tan2step_process2_submission()`:

- preserve `order_ref`, `challenge`, and `hhd_uc` when the returned
  `HbciExecStatus` is not successful;
- clear the SCA state only after a successful submission;
- return the failed `HbciExecStatus` instead of converting the bank-side FinTS
  rejection into a Rust exception.

## Consequences

Applications can inspect the failed status and decide whether to retry the
submission, prompt the user again, or restart the dialog.

The port keeps the current original-near distinction between transport/protocol
errors, which are Rust errors, and FinTS return-code rejections, which are
reported as execution status.

Replay tests cover both the successful clear path and the failed preserve path.

## References

- `docs/adr/0164-pintan-process2-tan-submission-execution.md`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport#checkSCAResponse`
