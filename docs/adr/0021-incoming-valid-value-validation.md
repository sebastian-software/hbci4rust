# ADR 0021: Incoming Valid Value Validation

## Status

Accepted

## Context

hbci4java copies XML `<valids>` entries into a syntax element table while
parsing protocol definitions. When a data element is parsed, `DE.parseValue`
checks the parsed value against the valid values for that path.

The Rust incoming parser already resolves wire segments to XML definitions,
extracts typed values, and checks XML `<value>` defaults. Without `<valids>`
checks, values such as `ProcPrep.lang` could be accepted even when the protocol
XML explicitly limits them.

## Decision

Validate parsed incoming segment values against the resolved `SyntaxDefinition`
`<valids>` entries after value extraction and default validation.

For each XML valid set, the parser builds the absolute path under the current
segment or message-context root and checks the parsed value when the path is
present. Valid sets whose paths are absent are ignored, matching the current
treatment of optional absent children.

This slice validates valid sets attached to the resolved segment definition.
Valid sets inherited from nested referenced data element group definitions are
left for a later porting step.

## Consequences

Incoming parsing now rejects values outside the protocol XML valid list, for
example a `ProcPrep.lang` value not present in the HBCI 3.0 `HKVVB` definition.

This moves the Rust parser closer to hbci4java's `valids` behavior while keeping
the current message mapping and optional-child tolerance unchanged.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement`
- Upstream: `org.kapott.hbci.protocol.DE.parseValue`
