# ADR 0019: Absent Optional Syntax Functions

## Status

Accepted

## Context

ADR 0018 added direct `MSGdef` value mapping for messages that can be matched
through direct `SEG` children. Many response message definitions also contain
optional `SF` children such as `BPD` and `UPD`.

Those syntax functions are not always present on the wire. Rejecting a message
definition merely because it declares an optional `SF` would make simple
responses like a minimal `DialogInitRes` fail even when no syntax-function
segments are present.

## Decision

When direct message mapping reaches an optional `SF` child (`minnum="0"`), skip
that child if the parser has not consumed any syntax-function segments.

Required `SF` children and actually present syntax-function content remain out
of scope for this slice and continue to be rejected or left as trailing
unconsumed segments.

## Consequences

Message definitions with absent optional BPD/UPD-style containers can be matched
by the direct `SEG` walker.

Full syntax-function reconstruction is still a later porting step. The parser
does not yet implement hbci4java's `SF`, `MultipleSFs`, `BPD`, `UPD`, `GVRes`, or
`Params` behavior.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- ADR 0018: Direct Message Value Mapping
- Upstream: `org.kapott.hbci.protocol.SF`
- Upstream: `org.kapott.hbci.protocol.MultipleSFs`
