# ADR 0013: Wire To Syntax Resolution

## Status

Accepted

## Context

ADR 0011 introduced a tokenizer for raw FinTS wire messages. ADR 0012 introduced
segment definition lookup by `SegHead.code` and `SegHead.version`.

hbci4java uses the next incoming segment header to decide whether a referenced
`SEG` definition can be applied while parsing `MSG` and `SF` structures. In
particular, `SF.extractSegId` reads the next segment code and version from the
wire string, and `SF.getRefSegId` reads the expected values from the XML syntax
definition.

## Decision

Add a separate resolution step between wire tokenization and full message-tree
parsing.

`WireMessage::resolve_segments(&ProtocolSyntax)` resolves each wire segment to a
parsed `SyntaxDefinition` by matching:

- incoming segment code from the first header component,
- incoming segment version from the third header component,
- XML `SEGdef` defaults `SegHead.code` and `SegHead.version`.

The resolved model borrows both the original `WireSegment` and the matching
`SyntaxDefinition`. It does not yet copy field values into `SyntaxElement`
paths, run datatype parsing, validate segment sequence numbers, or apply
hbci4java's special `Params`/`...S` handling.

Unknown segment code/version pairs and incomplete headers are protocol errors in
this isolated resolution step.

## Consequences

The incoming parser now has a testable bridge from raw FinTS segments to the
original XML protocol definitions.

Keeping resolution separate makes later value-to-path parsing easier to test:
failures can be attributed to delimiter parsing, segment lookup, or field
mapping independently.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.SF.extractSegId`
- Upstream: `org.kapott.hbci.protocol.SF.getRefSegId`
- Upstream: `org.kapott.hbci.protocol.MultipleSEGs`
