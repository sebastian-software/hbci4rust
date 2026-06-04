# ADR 0018: Direct Message Value Mapping

## Status

Accepted

## Context

The incoming parser can tokenize wire messages, resolve segments to XML
`SEGdef`s, validate sequence numbers, and expose flat segment/message value maps.

hbci4java parses incoming `MSG` structures by walking the XML `MSGdef` children.
That context matters because some segment codes and versions are shared by
multiple definitions, for example user and institute variants of message header
segments.

## Decision

Add `ResolvedWireMessage::values_for_message(&ProtocolSyntax, message_name)`.

The first slice supports `MSGdef`s whose expanded children can be matched as
direct `SEG` references. It walks the `MSGdef` in order, matches incoming
segments by the expected segment definition's `SegHead.code` and
`SegHead.version`, and prefixes extracted values with the message path.

This deliberately uses the expected `SEGdef` from the message context while
extracting values. That keeps shared code/version pairs such as `HNHBK:3`
context-sensitive instead of relying only on the earlier code/version
resolution.

Nested `SF` reconstruction is not ported in this slice. A message definition
that requires or reaches an `SF` child is rejected with an unsupported protocol
error.

## Consequences

Simple response messages such as `DialogEndRes` can now produce Java-like
message paths, for example `DialogEndRes.RetGlob.RetVal.text`.

The parser is still intentionally narrower than hbci4java's full `MSG`/`SF`
parser. Future slices can add syntax-function parsing, optional SF skipping, and
the BPD/UPD/GVRes container rules on top of this message-order walker.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.MSG`
- Upstream: `org.kapott.hbci.protocol.MultipleSEGs`
- Upstream: `org.kapott.hbci.protocol.SF`
