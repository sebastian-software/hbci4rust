# ADR 0020: Incoming Default Value Validation

## Status

Accepted

## Context

hbci4java copies XML `<value>` entries into a `predefs` table while parsing a
syntax element. When a data element is parsed, `DE.parseValue` checks the actual
parsed value against the predefined value for that path.

The Rust incoming parser already resolves segments to XML definitions and
extracts typed values. Without validating XML defaults, a segment could still be
accepted with inconsistent fixed fields such as `hbciversion`.

## Decision

Validate parsed incoming segment values against the resolved `SyntaxDefinition`
`<value>` defaults after value extraction.

For each XML default, the parser builds the absolute path under the current
segment or message-context root and compares the parsed value when the path is
present. Defaults whose paths are absent are not checked, matching the idea that
optional children may not have been parsed.

## Consequences

Incoming values now fail earlier when fixed XML defaults disagree with the wire
message, for example `MsgHeadInst.hbciversion` being `220` in an HBCI 3.0
message.

This moves the Rust parser closer to hbci4java's `predefs` behavior while still
keeping optional absent children tolerant.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement`
- Upstream: `org.kapott.hbci.protocol.DE.parseValue`
