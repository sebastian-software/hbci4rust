# ADR 0091: Global Callback Test Serialization

## Status

Accepted

## Context

hbci4java exposes process-wide helper APIs around initialization and callbacks.
The Rust port currently mirrors that shape with `manager::init(...)`,
`manager::done()`, and a global callback slot used by handler methods.

Most offline tests do not touch the global callback. A few integration tests do:

- dialog initialization emits institute-message callbacks;
- async checked queue admission emits account-correction callbacks.

Rust integration tests can run concurrently. If two tests call `init(...)` and
`done()` at the same time, they can overwrite or clear the shared callback while
another test is awaiting handler work.

## Decision

Serialize only integration tests that mutate the global callback runtime.

Use a shared Tokio test mutex in `tests/bootstrap.rs` around the full
`init(...)` / handler call / `done()` lifetime. Keep other tests parallel.

Cover `HbciHandler::try_add_to_queue_with_account_checks(...)` through the
public handler API, not only through private `HbciJob` unit tests.

## Consequences

Callback-facing integration tests become deterministic without forcing the
whole suite to run serially.

The public async queue-admission path is now covered with the same callback
shape that application code uses.

Remaining work:

- revisit the global callback model once the Rust API grows per-handler or
  per-passport callback injection;
- add similar serialization for future tests that mutate global manager params.

## Links

- `tests/bootstrap.rs`
- `src/manager/mod.rs`
- `src/manager/handler.rs`
- `docs/adr/0089-async-account-check-queue-admission.md`
