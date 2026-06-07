# ADR 0263: Guard Legacy Cleanup

## Status

Accepted

## Context

ADR 0262 changed the publication stance from "PinTAN first" to a non-legacy
FinTS PinTAN/HBCI-Plus port. That creates a new maintenance question: some
classic hbci4java jobs are already ported, but they are compatibility-carried
legacy surface rather than strategic modern API.

Removing or feature-gating those jobs can still break modern paths accidentally
because the port is original-near and shared helpers live in the same modules:

- `src/gv/mod.rs` holds both the public registry and the constraint/rendering
  helpers;
- classic and SEPA variants share result shapes, persistent-data conventions,
  and tests in `tests/bootstrap.rs`;
- the release checklist currently proves source-surface coverage against the
  broader original-near registry.

The project needs a cleanup plan that allows deleting or hiding legacy-carried
jobs without weakening modern FinTS, PinTAN/SCA, SEPA, CAMT, MT940, passport,
callback, or replay behavior.

## Decision

Add a guarded cleanup path before removing any legacy-carried job code.

The guard has three parts:

- a documented cleanup plan at `docs/architecture/legacy-cleanup-plan.md`;
- a source-controlled registry partition audit at
  `scripts/audit-modern-scope.sh`;
- release-candidate runner coverage for that audit.

The audit must classify every current `PINTAN_JOB_NAMES` entry as either:

- modern v1 surface; or
- compatibility-carried legacy surface.

It must fail when a registry entry is unclassified, classified twice, or
classified but no longer present in the registry.

Actual removal, hiding, or feature-gating of compatibility-carried jobs remains
out of this ADR. Each cleanup category needs its own ADR or implementation slice
that proves the affected surface is limited to compatibility-carried jobs.

## Consequences

Future cleanup commits get a small mechanical proof that they are touching the
legacy-carried surface deliberately.

The release runner becomes slightly stricter, but still offline-only.

The original-near coverage claim remains honest while the project prepares a
smaller non-legacy public surface.

## Links

- `docs/architecture/legacy-cleanup-plan.md`
- `docs/reference/modern-scope-audit.md`
- `scripts/audit-modern-scope.sh`
- `scripts/run-release-candidate-checks.sh`
- ADR 0262: Non-Legacy Publication Scope
