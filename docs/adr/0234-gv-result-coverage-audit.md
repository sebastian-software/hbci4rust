# 0234 Add A Local GV Result Coverage Audit

## Status

Accepted

## Context

ADR 0233 added a local audit for high-level `GV*.java` job coverage. The other
side of the original-near job surface is hbci4java's `GV_Result/GVR*.java`
classes.

The mapping is not one-to-one by Rust enum variant name:

- hbci4java has separate `GVRLastSEPA`, `GVRLastCOR1SEPA`, and
  `GVRLastB2BSEPA`; the Rust port stores all three as `LastSepa`.
- hbci4java has `GVRDauerLastList` and `GVRDauerLastNew`; the Rust port reuses
  the same durable-order result shapes as `DauerList` and `DauerNew`.
- Java all-caps suffixes such as `SEPA` and `TAN` use Rust-cased enum variants
  such as `InstUebSepa`, `TanList`, and `TanMediaList`.

The only upstream `GVR*.java` class that currently has no Rust result variant is
`GVRWPStammData`. The class documentation says it cannot yet be used through a
normal high-level job and requires the lowlevel `WPStammList` job. Lowlevel jobs
are outside v1 per ADR 0232.

## Decision

Add a local result coverage audit that compares:

- upstream files matching `GVR*.java` under
  `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result`;
- Rust enum variants in `HbciJobResultData`.

Normalize upstream names before comparison where the Rust port intentionally
shares one typed result shape across multiple original classes.

Document the current result as:

- upstream `GVR*.java` classes: 28;
- normalized upstream result shapes: 24;
- Rust typed result variants: 23;
- missing from Rust after normalization: `WPStammData`;
- extra Rust typed result variants: none.

Keep this audit local for the same reason as ADR 0233: the pinned upstream
checkout is stored under `target/reference/` and is not available in offline CI
by default.

## Consequences

The port can distinguish true typed-result gaps from intentional result-shape
sharing. `WPStammData` remains documented as a lowlevel-only gap rather than an
accidental omission.

Future typed result changes should update the audit normalization and the
architecture note together.

## References

- `docs/adr/0232-custom-msg-job-and-template-boundary.md`
- `docs/adr/0233-gv-job-coverage-audit.md`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result`
- `src/gv_result/mod.rs`
