# 0237 Track V1 Readiness With Evidence Matrix

## Status

Accepted

## Context

The port now has broad original-near coverage for the v1 PinTAN/HBCI-Plus
surface. The static job registry and typed result coverage audits show only
intentional v1 exclusions, while replay and offline tests cover much of the
runtime behavior.

A single percentage is useful for conversation, but it is too imprecise to guide
the remaining port work. It can hide the difference between:

- source-surface coverage, such as ported `GV*` and `GVR*` classes;
- deterministic offline parity, such as generated SEPA/CAMT/SWIFT/message
  artifacts;
- runtime confidence, such as replayed PinTAN dialogs and optional live smoke
  hooks;
- release hardening, such as public API docs and explicit unsupported
  boundaries.

## Decision

Track v1 readiness with an architecture-level evidence matrix.

The matrix must record:

- the current evidence source for each v1 phase or capability;
- the current state of that evidence;
- the remaining work that prevents calling v1 complete;
- the commands or documents that can be used to re-check the evidence.

Keep percentage estimates secondary and explicitly scoped to the v1
PinTAN/HBCI-Plus port, not to the full hbci4java repository including
chipcard/key-file media.

## Consequences

Progress discussions become anchored in inspectable facts instead of fuzzy
overall estimates.

The matrix can evolve as implementation proceeds, but changes that alter the
acceptance model or scope must get their own ADR.

The project still remains implementation-led: the matrix is not a replacement
for porting missing behavior, adding replay fixtures, or hardening the public
API.

## References

- `docs/architecture/porting-plan.md`
- `docs/architecture/job-coverage.md`
- `docs/architecture/result-coverage.md`
- `docs/reference/live-bank-tests.md`
- `docs/adr/0003-v1-pintan-scope.md`
- `docs/adr/0233-gv-job-coverage-audit.md`
- `docs/adr/0234-gv-result-coverage-audit.md`
