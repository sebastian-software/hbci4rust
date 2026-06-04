# ADR 0025: Direct Segment Syntax Function Mapping

## Status

Accepted

## Context

ADR 0024 made present optional syntax functions visible by detecting when the
next wire segment belongs to an optional `SF`. That avoided misleading later
errors, but still rejected real BPD/UPD content before any values could be
extracted.

hbci4java parses `SF` definitions by walking their child `SEG` and nested `SF`
references. It peeks at the next segment code and version before trying an
optional child, so absent optional syntax branches do not consume unrelated
segments.

## Decision

Add a first incoming `SF` mapper for syntax functions whose content can be
matched by direct or nested `SEG` children.

When `values_for_message` reaches an `SF` child, it now:

- checks whether the next resolved segment matches any segment reachable from
  the referenced `SFdef`;
- skips the `SF` if it is optional and absent;
- collects matching `SEG` children under the Java-near path of the `SF`, for
  example `DialogInitRes.BPD.BPA...`;
- reuses the existing segment value extraction, datatype parsing, defaults,
  valid-value checks, and incoming validation options.

## Consequences

Minimal BPD content such as `HIBPA` can now be parsed through
`DialogInitRes.BPD.BPA` instead of failing as unsupported.

This is not the final hbci4java `SF` port. The special parsing behavior for
`Params`, `GVRes`, repeated syntax functions, and broader BPD/UPD fixtures still
needs additional parity slices and golden tests.

## Links

- ADR 0024: Present Optional Syntax Functions
- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.SF`
- Upstream: `org.kapott.hbci.protocol.MultipleSFs`
