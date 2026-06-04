# ADR 0011: Protocol Wire Parsing Layer

## Status

Accepted

## Context

Incoming FinTS/HBCI messages need to be split into segments, data-element
groups, and data elements before the parsed values can be matched against the
`hbci-*.xml` protocol syntax.

hbci4java performs the delimiter scan in
`org.kapott.hbci.datatypes.SyntaxDE.findNextDelim`. That logic treats `'`, `+`,
and `:` as delimiters, supports `?` as a quote marker for the next character,
and skips `@len@payload` binary blocks so delimiters inside the payload are not
interpreted as syntax separators.

## Decision

Add a separate `protocol::wire` parsing layer for raw FinTS wire strings.

The first slice parses a UTF-8 `&str` into:

- `WireMessage`: ordered FinTS segments.
- `WireSegment`: ordered fields, with convenience accessors for segment code,
  sequence, and version from the segment header.
- `WireField`: ordered components, preserving empty fields and empty
  components.

The parser follows the upstream delimiter behavior closely:

- Unquoted `'` ends a segment.
- Unquoted `+` starts the next field.
- Unquoted `:` starts the next component.
- `?` quotes the next character and is removed from the stored value.
- `@len@payload` is preserved as one token fragment while delimiters inside the
  declared payload are skipped.

This layer does not yet map incoming values to `SyntaxElement`, does not run
datatype-specific parsing, and does not validate segment order or required
fields. Those are follow-up port slices on top of the tested wire tokenizer.

## Consequences

The incoming-message parser can be built in the same order as hbci4java:
delimiter handling first, then syntax-tree matching, then datatype conversion.

The current public parser accepts UTF-8 strings, so binary payloads whose declared
byte boundary does not land on a valid UTF-8 boundary are rejected. A later
byte-level parser can relax this without changing the higher-level wire model.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.datatypes.SyntaxDE.findNextDelim`
