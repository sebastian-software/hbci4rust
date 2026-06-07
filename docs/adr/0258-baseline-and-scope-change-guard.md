# ADR 0258: Baseline And Scope Change Guard

## Status

Accepted

## Context

ADR 0001 pins the upstream hbci4java baseline to tag
`hbci4j-core-4.1.11` and commit
`3b7ce667c73724daa1c836ed7333ed090c21a831`.

ADR 0003 pins the v1 implementation scope to FinTS PinTAN / HBCI-Plus and
excludes chipcard, PCSC, CTAPI, DDV, RDH, RAH, RSA key-file live support, and
Java passport import.

The v1 release checklist keeps a standing guard: future baseline or scope
changes must be recorded in an ADR before code changes rely on them.

## Decision

Keep the current upstream baseline and v1 scope unchanged.

Any future change to either of these release boundaries requires an accepted ADR
before dependent implementation, fixture, audit, or documentation changes are
treated as release evidence.

A baseline-change ADR must record at least:

- the new upstream tag, commit, and any version metadata mismatch;
- whether `scripts/fetch-upstream.sh` or upstream metadata changed;
- expected effects on copied protocol resources and copied fixtures;
- required reruns of job/result coverage audits;
- required license/header and attribution rechecks.

A scope-change ADR must record at least:

- the newly included or excluded upstream surface;
- whether public API, migration docs, unsupported-surface docs, and tests need
  to change;
- how the new surface fits the PinTAN/HBCI-Plus v1 boundary or why v1 is being
  widened;
- deterministic fixture, replay, or explicit limitation requirements.

## Consequences

The release checklist can treat the baseline/scope guard as covered for the
current evidence set.

The guard does not freeze future work. It only prevents accidental drift where
code, tests, or docs silently depend on a different upstream baseline or a wider
scope than the one users have been told v1 supports.

## Links

- `docs/adr/0001-upstream-baseline.md`
- `docs/adr/0003-v1-pintan-scope.md`
- `docs/adr/0005-upstream-reference.md`
- `docs/reference/unsupported-surfaces.md`
- `docs/architecture/release-checklist.md`
- `docs/architecture/v1-readiness.md`
- `scripts/fetch-upstream.sh`
