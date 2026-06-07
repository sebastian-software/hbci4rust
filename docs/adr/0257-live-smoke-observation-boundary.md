# ADR 0257: Live Smoke Observation Boundary

## Status

Accepted

## Context

ADR 0236 added ignored, environment-gated live PinTAN dialog smoke hooks. The v1
release checklist also requires manual live-bank observations, if any, to be
recorded without credentials and converted into deterministic replay fixtures or
explicit limitations before they influence acceptance.

The current v1 evidence set is offline: protocol tests, copied fixtures,
deterministic `ReplayCommClient` paths, parser/generator goldens and
limitations, and public docs. No manual live-bank observation is currently used
as release evidence.

## Decision

Record the live-smoke observation boundary in `docs/reference/live-bank-tests.md`.

For the current v1 release evidence:

- no manual live-bank observations are recorded;
- no additional bank-specific SCA variants from live smoke testing are part of
  the acceptance evidence;
- the ignored live hook remains available for manual experiments, but CI and v1
  acceptance remain offline-only;
- any future live observation must be anonymized, must not include credentials,
  PINs, TANs, or personal account data, and must become either deterministic
  replay coverage or an explicit limitation before it changes acceptance.

## Consequences

The release checklist can mark the live-observation handling items as covered
for the current evidence set: there are no live observations to convert, and the
recording/conversion rule is explicit.

This does not claim that live-bank testing has validated v1. It only says live
observations are not hidden inputs to the release decision.

If future manual live runs reveal bank-specific SCA or TAN behavior, add replay
fixtures or a limitation entry and record a new ADR when the public semantics or
release acceptance bar changes.

## Links

- `docs/reference/live-bank-tests.md`
- `docs/reference/malformed-bank-responses.md`
- `docs/architecture/release-checklist.md`
- `docs/architecture/v1-readiness.md`
- `tests/live_bank.rs`
- ADR 0007: Offline Test Strategy
- ADR 0236: Optional Live Bank Test Hooks
- ADR 0246: V1 Release Checklist
