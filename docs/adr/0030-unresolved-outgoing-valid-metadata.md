# ADR 0030: Unresolved Outgoing Valid Metadata

## Status

Accepted

## Context

hbci4java stores XML `<valids>` entries on outgoing message tree data elements
when the referenced path can be found. If a `<valids>` path cannot be resolved,
`SyntaxElement.storeValidValueInDE(...)` returns `false` and the constructor
continues.

The upstream `hbci-300.xml` contains at least one such mismatch:
`TANListListRes1` declares `<valids path="zustand">`, but the segment has no
`zustand` data element. It has `liststatus` instead. A strict Rust outgoing
message-tree builder therefore rejected `CustomMsgRes`, even though hbci4java can
construct that message definition.

## Decision

For outgoing message tree construction, unresolved `<valids>` metadata is
ignored.

Resolved `<valids>` entries are still stored on the addressed data element.
Unresolved `<value>` defaults remain strict, because hbci4java throws when a
definition default cannot be propagated.

## Consequences

Large original message definitions such as `CustomMsgRes` can be constructed
even when the XML table contains stale or mismatched valid-value metadata.

This decision only affects outgoing message-tree metadata storage. Incoming
wire validation still checks resolved `<valids>` through the parsed syntax
definitions and naturally skips paths that are absent in the extracted values.

## Links

- `src/protocol/message.rs`
- `tests/protocol_message.rs`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement.storeValidValueInDE`
