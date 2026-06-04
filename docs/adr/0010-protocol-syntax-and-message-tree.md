# ADR 0010: Protocol Syntax And Message Tree

## Status

Accepted

## Context

hbci4java builds `MSG`, `SEG`, `DEG`, `DE`, and `SF` runtime objects from the
original `hbci-*.xml` protocol tables. Those XML files use internal DTD
entities such as `MsgSigHeadUser`, `GVP2`, and `SecClassValids` to share segment
fragments and valid-value lists.

The Rust port needs a foundation for message generation and parsing that stays
close to the original before later idiomatic Rust cleanup.

## Decision

Keep the original `hbci-*.xml` resources as the source of truth for protocol
syntax.

Parse internal DTD entity declarations from those XML resources and expand entity
references during syntax loading. The parsed model also keeps entities
addressable for tests and diagnostics.

Build an original-near message tree from the expanded syntax definitions:

- Rust type names are cased for Rust, but message element paths keep hbci4java
  names such as `DialogInit.MsgHead.SegHead.code`.
- Child occurrences use hbci4java counter suffixes, where the first occurrence
  has no suffix and later occurrences use `_2`, `_3`, and so on.
- Elements with `minnum="0"` are still instantiated once, matching hbci4java's
  early tree-building behavior; later validation/rendering decides whether they
  are emitted.
- Definition-level `<value>` and `<valids>` entries are applied to the message
  tree during construction.

The first implementation provides tree construction, path lookup, value setting,
and Java-style data extraction. Delimiter rendering, parsing, validation, segment
enumeration, and rewrite hooks remain follow-up work.

## Consequences

Message and job porting can use the same path vocabulary as hbci4java, which
reduces translation risk for tests and golden fixtures.

The message tree is intentionally less idiomatic than a domain-specific Rust
model. That is acceptable for the first porting milestone and should be revisited
only after original-near parity tests are green.

## Links

- `src/protocol/model.rs`
- `src/protocol/message.rs`
- `tests/protocol_resources.rs`
- `tests/protocol_message.rs`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement`
- Upstream: `org.kapott.hbci.protocol.MultipleSyntaxElements`
- Upstream: `org.kapott.hbci.protocol.MSG`
