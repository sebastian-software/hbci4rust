# GV Job Coverage

This note records the current coverage of hbci4java high-level `GV*` job
classes by the Rust static PinTAN registry.

## Scope

The comparison uses:

- upstream baseline fetched with `scripts/fetch-upstream.sh`;
- upstream directory
  `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV`;
- Rust registry `PINTAN_JOB_NAMES` in `src/gv/mod.rs`.

The audit compares only direct `GV*.java` files in that directory. It does not
count abstract helper classes, SEPA generator/parser classes, result classes, or
manager/kernel lowlevel infrastructure.

## Current Snapshot

Last checked: 2026-06-07.

```text
upstream=68
rust=65
missing=LastCOR1SEPA,MultiLastCOR1SEPA,Template
extra=<none>
```

`Template` is intentionally not in the Rust v1 registry. hbci4java uses
`GVTemplate` for `HBCIHandler.newLowlevelJob(gvname)`, which creates arbitrary
lowlevel jobs dynamically. ADR 0232 keeps that fallback out of the v1 public API
while the Rust port remains a static, original-near PinTAN job surface.

`LastCOR1SEPA` and `MultiLastCOR1SEPA` were intentionally removed from the
public registry in ADR 0265 because EPC guidance says `COR1` is no longer
relevant for new SDD Core collections from 20 November 2016. The modern CORE
and B2B SEPA direct-debit jobs remain in scope.

## How To Recheck

```sh
scripts/fetch-upstream.sh
scripts/audit-job-coverage.sh
```

The audit exits successfully only when:

- the only missing upstream `GV*.java` names are `LastCOR1SEPA`,
  `MultiLastCOR1SEPA`, and `Template`;
- the Rust registry has no job names without a matching upstream `GV*.java`
  class.

It is intentionally not a CI gate yet because the upstream reference checkout is
not vendored and CI remains offline-only.

## References

- `docs/adr/0232-custom-msg-job-and-template-boundary.md`
- `docs/adr/0233-gv-job-coverage-audit.md`
- `docs/adr/0252-unsupported-v1-surface-reference.md`
- `docs/adr/0265-remove-cor1-public-jobs.md`
- `docs/reference/unsupported-surfaces.md`
- `scripts/audit-job-coverage.sh`
