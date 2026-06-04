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
Java-style data extraction, original-near delimiter rendering, segment
enumeration, and message-size preparation.

Rendering follows hbci4java's `toString(0)` structure:

- `MSG` and `SF` concatenate their rendered children.
- `SEG` joins children with `+` and appends `'`.
- `DEG` joins children with `:`.
- Trailing empty children are trimmed for `SEG` and for `DEG` unless the `DEG`
  is nested inside another `DEG`.
- Incomplete optional complex children may be omitted, matching hbci4java's
  handling of optional `MultipleSyntaxElements`.
- Data element values are rendered through a separate protocol datatype module,
  mirroring hbci4java's `org.kapott.hbci.datatypes.Syntax*` split.
- The first datatype slice ports render-time behavior for `AN`, `Code`, `ID`,
  `Num`, `Dig`, `Ctr`, `Cur`, and `Bin` in its hbci4java `B...` input form.
  Numeric `Bin` input (`N...`) and richer date/time/amount conversions remain
  later datatype-port slices.

Outgoing message preparation follows hbci4java's order closely:

- First set all instantiated segment sequence numbers to `0`.
- Set `MsgHead.msgsize` to zero, padded to the syntax-defined minimum size.
- Renderable segments are then enumerated from `1`.
- The final `MsgHead.msgsize` is set from the rendered message length.

Full incoming-message parsing, complete datatype validation/conversion, segment
validation, and rewrite hooks remain follow-up work.

## Consequences

Message and job porting can use the same path vocabulary as hbci4java, which
reduces translation risk for tests and golden fixtures.

The message tree is intentionally less idiomatic than a domain-specific Rust
model. That is acceptable for the first porting milestone and should be revisited
only after original-near parity tests are green.

## Links

- `src/protocol/model.rs`
- `src/protocol/datatype.rs`
- `src/protocol/message.rs`
- `tests/protocol_resources.rs`
- `tests/protocol_message.rs`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement`
- Upstream: `org.kapott.hbci.protocol.MultipleSyntaxElements`
- Upstream: `org.kapott.hbci.protocol.MSG`
- Upstream: `org.kapott.hbci.protocol.SEG`
- Upstream: `org.kapott.hbci.protocol.DEG`
- Upstream: `org.kapott.hbci.datatypes.SyntaxAN`
- Upstream: `org.kapott.hbci.datatypes.SyntaxBin`
- Upstream: `org.kapott.hbci.datatypes.SyntaxCtr`
- Upstream: `org.kapott.hbci.datatypes.SyntaxDig`
- Upstream: `org.kapott.hbci.datatypes.SyntaxID`
- Upstream: `org.kapott.hbci.datatypes.SyntaxNum`
