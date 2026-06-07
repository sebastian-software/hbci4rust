# ADR 0246: V1 Release Checklist

## Status

Accepted

## Context

ADR 0237 introduced an evidence matrix for v1 readiness. That matrix is useful
for tracking progress, but it is not operational enough for a final release
pass. A release candidate needs a checklist that can be re-run and reviewed
without relying on memory, percentage estimates, or live bank credentials.

The project is still an original-near hbci4java port. The checklist must
therefore check parity evidence and documented v1 boundaries before packaging,
instead of treating idiomatic Rust cleanup as a release requirement.

## Decision

Add a source-controlled v1 release checklist at
`docs/architecture/release-checklist.md`.

The checklist must be evidence-oriented and offline-first. It should cover:

- scope and upstream baseline;
- cargo, test, and audit gates;
- public API and Java-to-Rust mapping docs;
- protocol resources and generated/parsing parity evidence;
- PinTAN runtime replay breadth;
- passport storage and security review;
- license, NOTICE, crate metadata, and packaging review;
- optional live-bank smoke observations as non-blocking inputs that must be
  converted into deterministic replay fixtures or explicit limitations.

Do not make live credentials, chipcard/key-file support, Java passport import,
or idiomatic Rust rewrites part of the v1 acceptance bar.

## Consequences

The v1 completion claim becomes auditable: every remaining release task must
either have passing evidence, be explicitly out of scope, or be documented as a
known limitation.

Release hardening can proceed in small original-near slices without changing
the already accepted v1 scope.

The checklist may be updated as the port matures, but changes that alter scope
or release acceptance rules need a new ADR.

## Links

- `docs/architecture/v1-readiness.md`
- `docs/architecture/porting-plan.md`
- `docs/reference/java-to-rust-mapping.md`
- `docs/reference/live-bank-tests.md`
- ADR 0003: V1 PinTAN Scope
- ADR 0237: Track V1 Readiness With Evidence Matrix
