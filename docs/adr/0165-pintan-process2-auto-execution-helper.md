# ADR 0165: PinTAN Process-2 Auto Execution Helper

## Status

Accepted

## Context

Process variant 2 for two-step PinTAN now has all low-level runtime parts in the Rust port:

- a queue helper can add the original task and the first HKTAN (`process=4`) to the same message;
- `execute()` sends one signed `CustomMsg` and imports HITAN/SCA state from the response;
- `execute_tan2step_process2_submission()` sends the second HKTAN (`process=2`) with the TAN in
  the PinTAN user signature.

hbci4java hides this choreography inside its dialog runner. The Rust port should move toward that
behavior, but changing `HbciHandler::execute()` directly would surprise existing explicit tests and
callers while process variant 1 and decoupled process `S` are still separate follow-ups.

## Decision

Add an explicit automatic process-2 execution helper:

- `HbciHandler::execute_with_tan2step_process2(&mut self) -> HbciResult<HbciExecStatus>`;
- first call the existing single-message `execute()`;
- only continue when the first status succeeded, the current TAN method is two-step, the current
  BPD process is not exactly `1`, and the imported SCA state contains an order reference;
- call the existing `execute_tan2step_process2_submission()` for the second message;
- merge the second status into the first status by appending job results, messages, global return
  values, segment return values, dialog statuses, and exception messages;
- compute the merged success as the logical AND of both message statuses;
- leave `execute()` unchanged.

## Consequences

This gives callers a first original-near runtime entry point for full process-2 PinTAN execution
while preserving the smaller explicit building blocks and existing tests. It also creates the merge
behavior needed by a later default dialog runner.

Open follow-up work:

- port process variant 1 automatic execution;
- decide when this helper should replace or be called by `execute()`;
- represent HKTAN helper job results in a more Java-near way if later upstream parity tests require
  them to be hidden or linked differently;
- add decoupled process `S` status polling.
