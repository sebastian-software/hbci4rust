# GV Result Coverage

This note records the current coverage of hbci4java `GVR*` result classes by
Rust typed `HbciJobResultData` variants.

## Scope

The comparison uses:

- upstream baseline fetched with `scripts/fetch-upstream.sh`;
- upstream directory
  `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result`;
- Rust enum `HbciJobResultData` in `src/gv_result/mod.rs`.

The audit compares direct `GVR*.java` files. It does not count the generic
`HBCIJobResult`/`HBCIJobResultImpl` base classes or package documentation.

## Normalization

Several upstream result classes intentionally share one Rust result shape:

- `GVRDauerLastList` maps to `DauerList`.
- `GVRDauerLastNew` maps to `DauerNew`.
- `GVRLastSEPA`, `GVRLastCOR1SEPA`, and `GVRLastB2BSEPA` map to `LastSepa`.
- `GVRInstUebSEPA`, `GVRTANList`, and `GVRTANMediaList` map to Rust-cased
  variants `InstUebSepa`, `TanList`, and `TanMediaList`.

This keeps the Rust port close to observable result behavior without adding
separate enum variants whose payloads would be identical in v1.

## Current Snapshot

Last checked: 2026-06-07.

```text
upstream_raw=28
upstream_normalized=24
rust=23
missing=WPStammData
extra=<none>
```

`WPStammData` is intentionally not represented as a v1 typed result. The
hbci4java class comments state that it cannot yet be used through a normal
high-level job and requires lowlevel `WPStammList`; lowlevel jobs remain outside
v1 per ADR 0232.

## How To Recheck

```sh
scripts/fetch-upstream.sh
scripts/audit-result-coverage.sh
```

The audit exits successfully only when:

- the only missing normalized upstream result shape is `WPStammData`;
- the Rust result enum has no variants without a matching normalized upstream
  `GVR*.java` class.

It is intentionally not a CI gate yet because the upstream reference checkout is
not vendored and CI remains offline-only.

## References

- `docs/adr/0232-custom-msg-job-and-template-boundary.md`
- `docs/adr/0234-gv-result-coverage-audit.md`
- `scripts/audit-result-coverage.sh`
