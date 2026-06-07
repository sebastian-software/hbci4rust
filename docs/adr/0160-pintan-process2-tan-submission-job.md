# ADR 0160: PinTAN Process-2 TAN Submission Job Helper

## Status

Accepted

## Context

The port already has a low-level `TAN2Step` job, process-1 HKTAN preparation, HITAN SCA-state
extraction, TAN callback handling, and signed PinTAN messages. hbci4java's process variant 2
(`process=4` then `process=2`) keeps the first HKTAN together with the original task, stores the
`orderref` returned by HITAN, and sends a later single HKTAN that references this `orderref`; the
actual TAN is carried in `HNSHA`, not inside the HKTAN segment.

The v1 Rust handler is still explicit and queue-based, so it should expose the next original-near
building block before attempting full automatic queue patching.

## Decision

Add a handler helper that creates the second HKTAN job for process variant 2 from the current
PinTAN SCA state:

- public Rust API: `HbciHandler::new_tan2step_process2_job() -> HbciResult<HbciJob>`;
- generated Java job name remains `TAN2Step`;
- set exact frontend parameters `process=2`, `orderref=<stored HITAN orderref>`, and `notlasttan=N`;
- require a non-empty stored `orderref` and return `InvalidArgument` otherwise;
- do not request the TAN in this helper; message signing remains responsible for requesting and
  placing TAN data in `HNSHA`;
- keep decoupled process `S` and full automatic message queue patching as later slices.

## Consequences

This keeps the port close to hbci4java's observable HKTAN fields while preserving the current
incremental API. Callers can now build the process-2 confirmation job after a process-2 HITAN
response has populated `PinTanScaState.order_ref`. Full automatic process-variant orchestration can
reuse this helper later.
