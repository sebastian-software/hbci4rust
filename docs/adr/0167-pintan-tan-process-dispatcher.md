# ADR 0167: PinTAN TAN Process Dispatcher

## Status

Accepted

## Context

The Rust port now has explicit runtime helpers for both relevant two-step PinTAN process variants:

- `execute_with_tan2step_process1(job)` sends a process-1 HKTAN message before the original job;
- `execute_with_tan2step_process2()` sends a same-message process-2 step-1 request and then the
  process-2 TAN submission message when HITAN returned an order reference.

hbci4java chooses between those variants inside `AbstractPinTanPassport.patchMessagesFor2StepMethods(...)`
from the selected BPD security mechanism. Callers of the Rust port should not have to duplicate
that process selection forever. At the same time, replacing `execute()` directly is still premature
because multi-job process-1 queues, hidden HKTAN result linking, and decoupled process `S` are not
ported yet.

## Decision

Add an explicit dispatcher:

- `HbciHandler::execute_with_tan2step(&mut self) -> HbciResult<HbciExecStatus>`;
- keep `execute()` unchanged;
- for one-step TAN methods, delegate to `execute()`;
- for two-step `process=1`, support the Java-near single queued business-job case by removing the
  job from the queue and delegating to `execute_with_tan2step_process1(job)`;
- for two-step process variants other than exact `1`, support the single queued business-job case
  by inserting the first process-2 HKTAN and then delegating to `execute_with_tan2step_process2()`;
- for queues that do not contain TAN-required jobs, delegate to `execute()`;
- reject multi-job process-1 queues containing TAN-required jobs until inserted-message queue
  modeling is ported;
- keep existing explicit helpers public and tested.

## Consequences

This gives the port a first Java-near execution entry point while preserving the current explicit
building blocks. A caller can queue a single TAN-required job and ask the handler to execute it with
the selected PinTAN process, instead of manually choosing process 1 or process 2.

Open follow-up work:

- generalize process-1 inserted-message queue modeling beyond one queued business job;
- decide whether `execute()` should eventually call this dispatcher by default;
- hide or link HKTAN helper results like hbci4java if upstream parity demands it;
- add the full decoupled process `S` polling loop; ADR 0242 covers the status
  request helper and PIN-only signing boundary.
