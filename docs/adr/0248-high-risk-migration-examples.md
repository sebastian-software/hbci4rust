# ADR 0248: High-Risk Migration Examples

## Status

Accepted

## Context

ADR 0246 added a v1 release checklist. Its public API section still requires
per-job migration examples for the highest-risk transfer and statement
workflows.

The port already has deep replay and constraint tests for these jobs, but they
live mostly in `tests/bootstrap.rs`. That proves behavior, not discoverability.
Java users need a compact mapping from hbci4java-style job names and parameter
keys to the Rust v1 public API.

## Decision

Add `docs/reference/migration-examples.md` for per-job examples that stay close
to hbci4java names and parameter keys.

Cover the first release-hardening slice with:

- `KUmsAll` for MT940/MT942 account statement retrieval;
- `KUmsAllCamt` for CAMT account statement retrieval;
- `UebSEPA` for a single SEPA credit transfer;
- `LastSEPA` for a single SEPA direct debit.

Keep examples offline-friendly. The docs should show job construction and queue
preparation rather than live-bank execution with credentials. Add public-only
integration tests that import from the crate root and verify the same job shapes
resolve into the expected original-near lowlevel parameters or generated SEPA
payloads.

## Consequences

The release checklist can treat per-job migration examples as covered for the
highest-risk v1 workflows while leaving broader per-job docs as future
maintenance work.

The examples remain original-near and do not introduce an idiomatic facade or
builder API.

Future examples should be added when live-bank observations or user migration
questions expose unclear job shapes.

## Links

- `docs/reference/java-to-rust-mapping.md`
- `docs/reference/public-api.md`
- `docs/architecture/release-checklist.md`
- `tests/public_api.rs`
- `tests/bootstrap.rs`
- ADR 0246: V1 Release Checklist
- ADR 0247: Public API Review Docs
