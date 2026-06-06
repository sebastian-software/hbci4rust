# ADR 0089: Async Account Check Queue Admission

## Status

Accepted

## Context

hbci4java validates account data during job admission. `HBCIDialog.addTask(...)`
calls `job.verifyConstraints()`, and account-bearing jobs such as `GVSaldoReq`
then run `checkAccountCRC("my")`.

The Rust port already has:

- synchronous `HbciJob::verify_constraints()`;
- synchronous `HbciHandler::try_add_to_queue(...)`;
- async callback events;
- `CallbackReason::HaveCrcError` and `CallbackReason::HaveIbanError`;
- `Konto::check_iban()` for the IBAN checksum part.

National German BLZ/account-number CRC validation is larger than this tracer:
hbci4java uses `blz.properties` and `AccountCRCAlgs` for many algorithms. That
table and its sourcing need a separate decision.

## Decision

Add an async checked queue admission method:

- `HbciHandler::try_add_to_queue_with_account_checks(job).await`.

The method:

- calls `HbciJob::verify_constraints()` before queueing;
- runs the currently ported account checks afterwards;
- queues the job only when both steps complete;
- uses the globally configured async callback, matching the existing runtime
  callback model.

For this slice, the account check covers only IBAN validation for jobs whose
constraint table exposes `my.iban`, currently `SaldoReq`.

When an IBAN fails validation and a callback is configured, emit:

- `CallbackReason::HaveIbanError`;
- `CallbackDataType::Text`;
- `current_value = Some(invalid_iban)`.

If the callback returns a changed value, re-run the IBAN check. If the callback
returns no value or the same value, accept the unchanged invalid value, matching
hbci4java's `StringBuffer` behavior. If the value is corrected, write it back to
both frontend and low-level job parameters.

Keep `HbciHandler::try_add_to_queue(...)` synchronous and constraint-only for
now. It remains useful for existing tests and for callers that do not want async
callback interaction at queue time.

Do not emit `HaveCrcError` for national BLZ/account-number pairs until the
national CRC table strategy is ported.

## Consequences

The port now has the first async bridge from hbci4java's job-admission account
checks to Rust callbacks.

`SaldoReq` can correct an invalid IBAN before the job reaches the queue. The
actual outgoing renderer then sees the corrected low-level value.

Remaining work:

- port the national BLZ/account-number CRC data and algorithms;
- apply the same account-check helper to further account-bearing jobs as their
  constraint tables enter the Rust port;
- decide when the permissive `add_to_queue(...)` should be deprecated or made
  checked.

## Links

- `src/gv/mod.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0088-account-crc-callback-reasons.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#checkAccountCRC`
- Upstream: `org.kapott.hbci.GV.GVSaldoReq#verifyConstraints`
- Upstream: `org.kapott.hbci.manager.AccountCRCAlgs`
