# ADR 0086: Checked Queue Admission Tracer

## Status

Accepted

## Context

hbci4java verifies a job when it is added to the dialog task list:
`HBCIDialog.addTask(...)` calls `job.verifyConstraints()` before the task's
low-level parameters are copied into the outgoing message.

The Rust port already has a permissive `HbciHandler::add_to_queue(...)` from
early async/runtime tracers. Calling `verify_constraints()` unconditionally from
that path would currently break documented tracer behavior such as ADR 0036's
passport account fallback, because the Saldo constraint table is still a small
hard-coded approximation and not yet fully BPD/UPD-aware.

## Decision

Add `HbciHandler::try_add_to_queue(job) -> HbciResult<()>`.

The checked queue admission:

- takes ownership of the job;
- calls `HbciJob::verify_constraints()` before queueing;
- queues the job only when verification succeeds;
- preserves any low-level defaults that verification persisted into the job.

Keep `add_to_queue(...)` permissive for now. It remains the compatibility path
for earlier handler tracers and for cases where runtime fallback behavior has
not yet been represented in constraints.

## Consequences

Callers can now opt into a lifecycle point that mirrors hbci4java's
`HBCIDialog.addTask(...)` without destabilizing existing replay tests.

Tests cover successful verified queue admission and rejection of a job with
missing required data before it reaches the queue.

Remaining work:

- make `add_to_queue(...)` checked or deprecate it once constraints and runtime
  fallbacks are fully aligned;
- call verification from `execute()` after applying any documented passport/UPD
  fallbacks;
- port BPD/UPD-driven constraint generation so Saldo verification does not rely
  on hard-coded Saldo7 assumptions;
- decide how failed verification should interact with queued jobs in
  multi-job batches.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0036-passport-account-fallback-tracer.md`
- `docs/adr/0085-constraint-verification-default-persistence.md`
- Upstream: `org.kapott.hbci.manager.HBCIDialog#addTask`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#verifyConstraints`
