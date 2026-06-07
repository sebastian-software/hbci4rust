# ADR 0161: PinTAN Process-2 Step-1 Job Helper

## Status

Accepted

## Context

ADR 0160 added the process-2 TAN submission helper for the second HKTAN (`process=2`) after HITAN
has returned an `orderref`. hbci4java's process variant 2 starts earlier: when a task needs a TAN,
the original task and a first HKTAN are sent in the same message. That first HKTAN has `process=4`
and an `ordersegcode`; it may carry a selected TAN medium, but hbci4java does not set the process-1
order hash, order account, challenge-class parameters, or `notlasttan=N` in this step.

The Rust port is still explicit and queue-based, so we need a focused helper for this first
process-2 HKTAN before adding automatic queue patching.

## Decision

Add handler helpers that create the first process-2 HKTAN job from an existing task:

- `HbciHandler::new_tan2step_process2_step1_job(&self, task: &HbciJob) -> HbciResult<HbciJob>`;
- `HbciHandler::new_tan2step_process2_step1_job_with_tan_media_selection(&mut self, task:
  &HbciJob) -> HbciResult<HbciJob>`;
- generated Java job name remains `TAN2Step`;
- set exact frontend parameters `process=4` and `ordersegcode=<task HBCI code>`;
- set `tanmedia` only when the passport has one or BPD-driven TAN-media selection supplies one;
- do not calculate or set `orderhash`;
- do not set order-account, challenge-class, `orderref`, or `notlasttan` in this helper.

## Consequences

This gives callers the two explicit building blocks for process variant 2: the task-adjacent
`process=4` HKTAN and the later `process=2` TAN submission HKTAN. Automatic queue patching remains a
later slice, but it can now compose these helpers instead of duplicating field rules.
