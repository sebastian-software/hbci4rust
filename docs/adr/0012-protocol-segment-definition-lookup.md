# ADR 0012: Protocol Segment Definition Lookup

## Status

Accepted

## Context

Incoming FinTS wire segments expose their segment code and version in the first
field, for example `HIRMG:2:2`. To parse those segments against the original XML
syntax, the Rust port needs a way to resolve that code/version pair to a
`SEGdef`.

The original `hbci-*.xml` files already declare the segment code and version as
default values on each segment definition, usually through:

- `SegHead.code`
- `SegHead.version`

## Decision

Resolve segment definitions directly from the parsed XML value metadata.

`ProtocolSyntax` now exposes:

- `segment_definition(code, version)` for exact code/version lookup.
- `segment_definitions_by_code(code)` for version discovery and diagnostics.

`SyntaxDefinition` exposes:

- `default_value(path)` for original XML `<value>` entries.
- `segment_code()` and `segment_version()` for `SEGdef` constants.

The first implementation scans the parsed definitions instead of building a
separate generated index.

## Consequences

The incoming parser can stay close to the original syntax tables without adding
a second source of truth for segment identifiers.

Linear scanning is acceptable for the first porting milestone because syntax is
loaded offline and the definition set is small. If profiling later shows this is
hot, an internal index can be added behind the same public lookup methods.

## Links

- `src/protocol/model.rs`
- `tests/protocol_resources.rs`
- `resources/protocol/hbci-*.xml`
