# ADR 0022: Recursive Incoming Definition Metadata Validation

## Status

Accepted

## Context

hbci4java builds nested `SyntaxElement` objects from referenced `SEGdef`,
`DEGdef`, and `SFdef` definitions. Each syntax element carries the XML
metadata declared on its own definition, including `<value>` defaults and
`<valids>` lists.

The Rust incoming parser already validates metadata attached to the resolved
segment definition. ADR 0021 intentionally deferred valid sets from referenced
data element group definitions. That left a gap for values such as
`CompMethod1.SuppCompMethods.func`, where the allowed values are declared on
the `SuppCompMethods` `DEGdef`, not on the surrounding `CompMethod1` segment.

## Decision

Validate definition metadata recursively while parsing incoming referenced data
element groups.

After a `DEG` field or nested `DEG` component has been consumed, the parser now
validates the referenced `SyntaxDefinition` under the current absolute path:

- XML `<value>` defaults are checked against present parsed values.
- XML `<valids>` lists are checked against present parsed values.
- Absent optional paths remain ignored.

The segment-level validation remains in place and uses the same helper.

## Consequences

Incoming parsing is closer to hbci4java's nested `SyntaxElement` behavior and
can reject invalid values declared by referenced `DEGdef` metadata.

This makes real protocol fixtures such as `HIKPV`/`SuppCompMethods` stricter
without changing the wire tokenizer, field cursor, or message-level mapping
shape.

## Links

- ADR 0021: Incoming Valid Value Validation
- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement`
- Upstream: `org.kapott.hbci.protocol.DE.parseValue`
