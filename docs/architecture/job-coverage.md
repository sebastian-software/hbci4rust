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
rust=55
missing=Donation,Last,LastCOR1SEPA,MultiLast,MultiLastCOR1SEPA,MultiUeb,StornoLast,Template,Ueb,UebBZU,UebEil,UebGar,Umb
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

`MultiLast` and `MultiUeb` were intentionally removed from the public registry
in ADR 0266 because they are DTAUS bulk jobs over the old national payment
rails. The modern SEPA bulk jobs remain in scope.

`Last` and `StornoLast` were intentionally removed from the public registry in
ADR 0267 because they are classic national direct-debit jobs over the old
payment rail. The modern SEPA Core and B2B direct-debit jobs remain in scope.

`Donation`, `Ueb`, `UebBZU`, `UebEil`, `UebGar`, and `Umb` were intentionally
removed from the public registry in ADR 0268 because they are classic national
domestic credit-transfer or account-transfer jobs. The modern SEPA transfer,
instant-transfer, bulk-transfer, scheduled-transfer, and SEPA account-transfer
jobs remain in scope.

## How To Recheck

```sh
scripts/fetch-upstream.sh
scripts/audit-job-coverage.sh
```

The audit exits successfully only when:

- the only missing upstream `GV*.java` names are `Donation`, `Last`,
  `LastCOR1SEPA`, `MultiLast`, `MultiLastCOR1SEPA`, `MultiUeb`, `StornoLast`,
  `Template`, `Ueb`, `UebBZU`, `UebEil`, `UebGar`, and `Umb`;
- the Rust registry has no job names without a matching upstream `GV*.java`
  class.

It is intentionally not a CI gate yet because the upstream reference checkout is
not vendored and CI remains offline-only.

## References

- `docs/adr/0232-custom-msg-job-and-template-boundary.md`
- `docs/adr/0233-gv-job-coverage-audit.md`
- `docs/adr/0252-unsupported-v1-surface-reference.md`
- `docs/adr/0265-remove-cor1-public-jobs.md`
- `docs/adr/0266-remove-dtaus-bulk-public-jobs.md`
- `docs/adr/0267-remove-classic-direct-debit-public-jobs.md`
- `docs/adr/0268-remove-classic-domestic-transfer-public-jobs.md`
- `docs/reference/unsupported-surfaces.md`
- `scripts/audit-job-coverage.sh`
