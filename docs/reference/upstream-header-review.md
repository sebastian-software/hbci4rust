# Upstream Header Review

Snapshot date: 2026-06-07.

This page records the current upstream license/header recheck for the scoped v1
PinTAN/HBCI-Plus port. It is release evidence for the pinned hbci4java baseline,
not legal advice and not a permanent substitute for publication review.

## Baseline

- Upstream repository: `https://github.com/hbci4j/hbci4java`
- Tag: `hbci4j-core-4.1.11`
- Commit: `3b7ce667c73724daa1c836ed7333ed090c21a831`
- Local reference: `target/reference/hbci4java/`

## Project-Level License Evidence

The upstream repository carries project-level LGPL 2.1 evidence:

- `target/reference/hbci4java/readme.md` declares LGPL 2.1 and notes that the
  project was GPLv2 until 2016;
- `target/reference/hbci4java/LICENSE` contains the LGPL 2.1 license text;
- `target/reference/hbci4java/src/main/resources/COPYING` also contains the
  LGPL 2.1 license text.

This supports ADR 0002's treatment of the upstream repository as LGPL 2.1 at
project level.

## Header Scan

The current source-header scan focused on the upstream core Java package:

```sh
rg --files target/reference/hbci4java/src/main/java/org/kapott/hbci -g '*.java' | wc -l
rg -l "GNU Lesser General Public" target/reference/hbci4java/src/main/java/org/kapott/hbci -g '*.java' | wc -l
rg --files-without-match "GNU Lesser General Public" target/reference/hbci4java/src/main/java/org/kapott/hbci -g '*.java' | wc -l
```

Current result:

```text
checked_java_files=399
with_classic_lgpl_header=325
without_classic_lgpl_header=74
```

The files without the exact classic header include several newer SEPA
parser/generator classes, concurrent helper classes, selected newer job/result
classes, an example, and some v1-irrelevant chipcard/RSA classes. This is
consistent with ADR 0002's "project-level LGPL with individual header
inconsistencies" assumption.

## Copied Artifacts In This Port

The current port contains copied upstream protocol resources and offline
fixtures in these paths:

```sh
rg --files resources/protocol tests/fixtures/hbci4java | sort
```

There are 22 files in those local paths:

- 20 copied upstream artifacts;
- 2 local attribution README files.

The copied upstream artifacts are:

- protocol resources: `hbci-201.xml`, `hbci-210.xml`, `hbci-220.xml`,
  `hbci-300.xml`, `hbci-plus.xml`, and `hbci.dtd`;
- bank-info fixture: `test-bank-info.properties`;
- security-mechanism fixtures: `challengedata.xml`, `TestQRCode-001.txt`,
  `TestMatrixCode-001.txt`, and `TestMatrixCode-002.txt`;
- CAMT fixtures:
  `test-camt-parse-05200102.xml`, `test-camt-parse-05200108.xml`,
  `test-camt-parse-5200108-invalid-saldo.xml`,
  `test-camt-parse-5200108-missing-date.xml`,
  `test-camt-parse-invalid.xml`, `test-camt-parse-none.xml`, and
  `test-camt-ruecklastschrift.xml`;
- SWIFT/MT940 fixtures: `test-mt940-001.sta` and `test-mt940-002.sta`.

The protocol XML/DTD resources, text fixtures, XML fixtures, properties
fixture, and binary matrix-code payloads do not carry useful individual
file-level text headers in the checked upstream baseline. They remain covered by
the upstream project-level license evidence and the local attribution files.

## Local Attribution

Local attribution is present in:

- `NOTICE`;
- `LICENSE`;
- `docs/adr/0002-license-and-attribution.md`;
- `resources/protocol/README.md`;
- `tests/fixtures/hbci4java/README.md`.

The current crate license metadata remains `LGPL-2.1-or-later`.

## Release Decision

For the current pinned baseline, keep the v1 release posture:

- treat hbci4java as LGPL 2.1 at repository level;
- keep `hbci4rust` as `LGPL-2.1-or-later`;
- keep copied upstream resources and fixtures because they are required for
  original-near offline parity;
- keep directory-level attribution for copied non-source artifacts;
- do not add copied Java headers into Rust files unless a future slice copies a
  substantial Java source body directly.

Rerun this review before publishing if:

- the upstream baseline changes;
- copied upstream artifacts change;
- generated upstream-derived sources are checked in;
- the crate package contents change materially;
- legal review asks for per-file notices beyond the current attribution model.

## References

- `docs/adr/0002-license-and-attribution.md`
- `docs/adr/0253-upstream-header-recheck.md`
- `docs/reference/packaging.md`
- `docs/architecture/release-checklist.md`
- `NOTICE`
- `LICENSE`
- `resources/protocol/README.md`
- `tests/fixtures/hbci4java/README.md`
