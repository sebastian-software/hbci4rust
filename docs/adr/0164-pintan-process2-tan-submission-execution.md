# ADR 0164: PinTAN Process-2 TAN Submission Execution

## Status

Accepted

## Context

Process variant 2 for two-step PinTAN is a two-message flow. The first message contains the
original task and a first HKTAN with `process=4`. The bank then returns HITAN state, including an
order reference and usually a TAN challenge. The second message contains only HKTAN with
`process=2`, references the stored order reference, and carries the TAN in the PinTAN user
signature.

The Rust port now has:

- automatic process-2 initial HKTAN queueing for TAN-protected tasks;
- HITAN/SCA state import into the PinTAN passport;
- a helper that builds the process-2 TAN submission job from the stored order reference;
- SCA-aware user-signature rendering that can ask the callback for the TAN.

Changing `HbciHandler::execute()` to loop automatically would alter existing single-message tests
and make result aggregation semantics larger than this slice. hbci4java's full dialog runner should
eventually become the default behavior, but the next safe step is an explicit runtime helper for the
second process-2 message.

## Decision

Add an explicit handler method:

- `HbciHandler::execute_tan2step_process2_submission(&mut self) -> HbciResult<HbciExecStatus>`;
- require the normal queue to be empty before creating the TAN submission message;
- create the queued `TAN2Step` job with existing `new_tan2step_process2_job()`;
- verify and queue that HKTAN job through the checked queue path;
- execute it through the normal signed `CustomMsg` path, so the TAN is collected by the existing
  SCA callback helper and encoded in `HNSHA`;
- clear the short-lived SCA state only after the submission message succeeds;
- keep `execute()` single-message for now.

## Consequences

This gives callers and replay tests an original-near way to complete the second half of process
variant 2 without changing the existing queue semantics. It also establishes the internal building
block for a later fully automatic PinTAN dialog loop that can combine the first business result and
the second HKTAN confirmation result.

Open follow-up work:

- define result aggregation for automatic multi-message execution;
- failed `process=2` submissions keep SCA state per ADR 0239;
- transport errors during `process=2` submission keep queued retry state per
  ADR 0240;
- support process variant 1 automatic execution once the multi-message runner exists;
- cover decoupled process `S` status polling separately.
