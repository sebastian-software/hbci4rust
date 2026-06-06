# ADR 0063: Return Value Element Tracer

## Status

Accepted

## Context

hbci4java's `HBCIRetVal` exposes an optional `element` field in addition to the
return code, text, segment reference, data reference, and parameters.

The upstream display form renders `element` only inside the segment-reference
parentheses:

- return code and text are always rendered as `<code>:<text>`;
- parameters are rendered as ` p:<param>`;
- `segref` opens the reference suffix;
- `deref` is rendered only inside that suffix;
- `element` is rendered only inside that suffix.

ADR 0058 and ADR 0062 intentionally deferred this field while introducing
display and original-near equality.

## Decision

Add `HbciReturnValue::element` as `Option<String>`.

Mark it with `serde(default)` so previously serialized Rust-native values remain
readable.

Render `element` in `Display` only when `segment_ref` is present, matching the
upstream shape:

- `3020:Hinweis (4: GVRes.SaldoRes7)`;
- `3020:Hinweis (4:2: GVRes.SaldoRes7=value)`.

Keep `HbciReturnValue` equality aligned with hbci4java's `equals(...)` by
continuing to compare only:

- `code`;
- `text`;
- `segment_ref`;
- `data_ref`.

Do not reconstruct `element` in the response parser in this slice. The parser
currently extracts status fields from the flattened response map and does not
carry the low-level resolved element path/value annotation yet.

## Consequences

The Rust return-value shape now includes the same diagnostic annotation field as
hbci4java.

Existing serialized return values keep deserializing because the new field has a
default.

Tests pin the original display rule that `element` is invisible without
`segment_ref`, and pin the original equality rule that `element` is ignored.

Remaining work:

- decide whether the parser should reconstruct `element` from resolved message
  value metadata;
- decide whether a strict structural comparison helper is useful for tests or
  serialization audits.

## Links

- `src/gv_result/mod.rs`
- `tests/status.rs`
- `docs/adr/0058-return-value-display-tracer.md`
- `docs/adr/0062-return-value-equality-tracer.md`
- Upstream: `org.kapott.hbci.status.HBCIRetVal#toString`
- Upstream: `org.kapott.hbci.status.HBCIRetVal#equals`
