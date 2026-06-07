# ADR 0166: PinTAN Process-1 Auto Execution Helper

## Status

Accepted

## Context

hbci4java's `AbstractPinTanPassport.patchMessagesFor2StepMethods(...)` handles process variant 1
by inserting a separate HKTAN message before the original business message:

1. render and hash the original order segment;
2. send `TAN2Step` with `process=1`, `ordersegcode`, `orderhash`, optional order account, TAN
   medium, and challenge parameters;
3. import the HITAN/SCA challenge from the response;
4. send the original business message signed with the collected TAN in `HNSHA`.

The Rust port already has the individual pieces: process-1 HKTAN preparation, HITAN/SCA state
import, SCA-aware PinTAN signing, single-message `execute()`, and local execution-status merging
from the process-2 helper. Automatic queue patching still rejects process variant 1 because a
single queue cannot represent "send HKTAN first, then send the original task".

## Decision

Add an explicit process-1 execution helper:

- `HbciHandler::execute_with_tan2step_process1(&mut self, job: HbciJob) -> HbciResult<HbciExecStatus>`;
- require the handler queue to be empty before starting;
- require the current TAN method to be two-step and the current BPD TAN process to be exactly `1`;
- verify the original job before sending anything;
- build and verify the process-1 HKTAN with existing original-near helpers;
- send the HKTAN message through the normal signed `CustomMsg` path;
- only send the original job when the HKTAN status succeeded;
- rely on the existing SCA-aware signature renderer to collect the TAN for the original job;
- clear short-lived SCA state after the original message succeeds;
- merge both message statuses by appending job results, messages, and return values.

Keep `try_add_to_queue_with_initial_tan_job(...)` rejecting process variant 1 for now, because it is
a queue-admission helper and cannot preserve the needed message ordering by itself.

## Consequences

This moves the Rust port closer to hbci4java's multi-message PinTAN runtime without changing the
existing queue semantics or `execute()` default behavior. Process variant 1 is now testable
offline as a full two-message flow.

Open follow-up work:

- integrate process-1 and process-2 helpers into a Java-near default dialog runner;
- decide whether helper HKTAN job results should stay visible or be linked/hidden like hbci4java's
  task references;
- support multi-job process-1 queues once the original queue model can represent inserted messages;
- port decoupled process `S` status polling separately.
