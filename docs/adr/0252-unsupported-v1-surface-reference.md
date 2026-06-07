# ADR 0252: Unsupported V1 Surface Reference

## Status

Accepted

## Context

The v1 port is intentionally scoped to FinTS PinTAN / HBCI-Plus. Several
hbci4java surfaces remain out of scope for v1 even though the port keeps job
names, parameter keys, protocol resources, and tests close to the original.

The current source-surface audits record two intentional gaps:

- `GVTemplate`, because it is hbci4java's dynamic `newLowlevelJob(...)`
  fallback for arbitrary lowlevel segment names;
- `WPStammData`, because upstream documents it as requiring the lowlevel
  `WPStammList` path rather than a normal high-level job.

Other exclusions, such as chipcard, PCSC, CTAPI, DDV, RDH, RAH, RSA key-file
live support, Java passport import/export, and live-bank acceptance criteria,
are already recorded across ADRs, README text, release planning, and public API
notes. The release checklist still requires the public unsupported surface to
match the audit exclusions before v1 can be declared ready.

## Decision

Add a single public reference page:

```text
docs/reference/unsupported-surfaces.md
```

The page will group the v1 unsupported surface by category:

- security media and passport formats;
- dynamic lowlevel job creation, including `GVTemplate` and
  `newLowlevelJob(...)`;
- typed result exclusions, including `WPStammData` and `WPStammList`;
- runtime and testing boundaries, including offline-only CI and optional
  env-gated live smoke tests;
- release and packaging boundaries that must still be closed before a v1
  release candidate.

Update the public API and migration references to point to this page. Update the
release checklist evidence and mark the checklist item for public unsupported
surface documentation as covered once the page matches the job and result audit
exclusions.

Any future widening of these unsupported surfaces requires a new ADR before code
or public API changes rely on it.

## Consequences

The v1 scope becomes easier to audit from one public document instead of being
spread across README, ADRs, and coverage notes.

The port remains original-near within the PinTAN/HBCI-Plus scope while avoiding
accidental promises for chipcard, key-file live support, Java passport
compatibility, arbitrary lowlevel jobs, or live-bank acceptance.

The release checklist still cannot be fully completed by this decision alone.
Final release-candidate gates, upstream header checks, risky parser/generator
golden artifacts, and live observation handling remain separate checklist
items.

## Links

- `docs/reference/unsupported-surfaces.md`
- `docs/architecture/release-checklist.md`
- `docs/architecture/job-coverage.md`
- `docs/architecture/result-coverage.md`
- `docs/reference/public-api.md`
- `docs/reference/java-to-rust-mapping.md`
- ADR 0003: V1 PinTAN Scope
- ADR 0232: CustomMsg Job And Template Boundary
- ADR 0233: GV Job Coverage Audit
- ADR 0234: GV Result Coverage Audit
- ADR 0236: Optional Live Bank Test Hooks
- ADR 0246: V1 Release Checklist
