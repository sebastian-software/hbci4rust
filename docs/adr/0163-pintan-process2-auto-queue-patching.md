# ADR 0163: PinTAN Process-2 Auto Queue Patching

## Status

Accepted

## Context

hbci4java patches queued jobs before sending them. For TAN-protected jobs in two-step PinTAN mode,
it checks the BPD PinTAN need-TAN table, then inserts HKTAN jobs according to the selected TAN
process variant. Process variant 2 sends the original task and a first HKTAN (`process=4`) in the
same message. Process variant 1 is different: it sends the first HKTAN (`process=1`) in a separate
message before the original task.

The Rust port currently has a single-message queue and explicit HKTAN builders. It can safely
automate the process-2 same-message case now, but process-1 automation requires multi-message
execution first.

## Decision

Add an explicit queue helper:

- `HbciHandler::try_add_to_queue_with_initial_tan_job(&mut self, job: HbciJob) -> HbciResult<()>`;
- verify the original job before modifying the queue;
- only consider jobs with a known original-near HBCI code;
- if the current TAN method is one-step (`999`), queue only the original job;
- if BPD `pin_tan_info_for_segment_code(<job code>)` is not `J`, queue only the original job;
- if BPD TAN process is exactly `1`, return `Unsupported` because process-1 requires
  multi-message execution;
- otherwise queue the original job followed by the process-2 step-1 HKTAN (`process=4`) using the
  existing dispatcher rules for TAN media;
- keep `add_to_queue` and `try_add_to_queue` behavior unchanged.

## Consequences

This starts moving queue behavior toward hbci4java without changing existing callers. Process-2
tasks can now be queued with their task-adjacent HKTAN automatically, while process-1 automatic
execution remains explicitly blocked until the message runner can send multiple messages per
logical queue.
