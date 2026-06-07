# ADR 0270: Enforce Public Job Registry Boundary

Date: 2026-06-07

## Status

Accepted.

## Context

ADRs 0265 through 0269 removed multiple legacy jobs from the public PinTAN v1
registry. `HbciHandler::new_job(...)` correctly rejects those names.

The Rust API still exposes `HbciJob::new(...)` for original-near tests,
serialization, and migration tooling. Without an additional guard, callers
could manually construct an unsupported job and pass it through queueing or
rendering paths that still contained temporary legacy implementation code.

That would weaken the "unsupported or out-of-scope" claim: the job would be
absent from discovery but not truly outside the executable v1 surface.

## Decision

Treat `PINTAN_JOB_NAMES` as the enforced public job boundary, not just as a
factory allow-list.

The handler now rejects any job name outside `PINTAN_JOB_NAMES` when:

- `try_add_to_queue(...)` is called;
- `try_add_to_queue_with_initial_tan_job(...)` is called;
- `try_add_to_queue_with_account_checks(...)` is called;
- queued jobs are rendered for execution.

The render-time guard is necessary because `add_to_queue(...)` remains an
infallible compatibility helper for tests and original-near examples. It must
not become a bypass for removed legacy jobs.

## Consequences

- Manually constructed unsupported jobs such as
  `HbciJob::new("LastCOR1SEPA")` now fail with the same
  `unsupported or out-of-scope job: ...` error as `HbciHandler::new_job(...)`.
- Temporary internal helpers for removed jobs can be deleted in later
  mechanical cleanup slices without changing public behavior.
- Modern jobs in `PINTAN_JOB_NAMES` remain unaffected.
- `HbciJob::new(...)` remains available, but it no longer defines public
  support by itself.

## References

- `docs/adr/0252-unsupported-v1-surface-reference.md`
- `docs/adr/0258-baseline-and-scope-change-guard.md`
- `docs/adr/0265-remove-cor1-public-jobs.md`
- `docs/adr/0266-remove-dtaus-bulk-public-jobs.md`
- `docs/adr/0267-remove-classic-direct-debit-public-jobs.md`
- `docs/adr/0268-remove-classic-domestic-transfer-public-jobs.md`
- `docs/adr/0269-remove-classic-scheduled-standing-public-jobs.md`
- `src/gv/mod.rs`
- `src/manager/handler.rs`
