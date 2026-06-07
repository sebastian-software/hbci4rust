# ADR 0241: Public PinTAN Runtime Migration Docs

## Status

Accepted

## Context

The v1 readiness matrix still calls out public API documentation as a release
hardening gap. The Rust port now has enough PinTAN runtime surface to document a
real Java-to-Rust migration path:

- explicit dialog lifecycle helpers: `init().await` and `close().await`;
- Java-named jobs through `HbciHandler::new_job(...)`;
- checked queue admission through `try_add_to_queue(...)`;
- raw single-message execution through `execute().await`;
- Java-near two-step TAN dispatch through `execute_with_tan2step().await`;
- async callbacks for PIN, TAN, and selection events;
- `ReplayCommClient` for deterministic offline tests.

Without a clear public mapping, Java users may pick `execute().await` for a
normal TAN-required workflow and miss the explicit dispatcher that currently
models hbci4java's hidden PinTAN choreography more closely.

## Decision

Document the v1 PinTAN runtime API as a migration guide rather than a broad
reference manual:

- keep `docs/reference/java-to-rust-mapping.md` as the primary Java-to-Rust
  bridge;
- add an explicit handler execution matrix that distinguishes `execute()` from
  `execute_with_tan2step()`;
- show a common balance request flow using Java job names and original
  parameter keys;
- explain that v1 callers should prefer `execute_with_tan2step().await` for
  queued business jobs that may require TAN;
- keep `execute().await` documented as the low-level, single-message primitive
  used by replay tests and explicit choreography helpers;
- link the mapping and readiness docs from the README so the repository front
  page no longer describes the runtime as only a bootstrap scaffold.

## Consequences

The docs become a better migration aid for Java callers without changing public
API behavior. The port keeps its original-near runtime split for now: Java-near
PinTAN convenience is explicit, while `execute()` remains a stable low-level
building block until a later ADR decides whether it should dispatch by default.

The documentation must stay synchronized with future handler behavior changes,
especially any later decision to make `execute()` call the TAN dispatcher.

## Links

- `docs/reference/java-to-rust-mapping.md`
- `docs/architecture/v1-readiness.md`
- ADR 0167: PinTAN TAN Process Dispatcher
- ADR 0235: Java To Rust Mapping Notes
