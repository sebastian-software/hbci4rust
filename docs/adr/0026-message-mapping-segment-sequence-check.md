# ADR 0026: Message Mapping Segment Sequence Check

## Status

Accepted

## Context

hbci4java's incoming `MSG` constructor parses the message and then calls
`checkSegSeq(1)` by default. Callers can opt out with `DONT_CHECK_SEQ`.

The Rust port already had an explicit `validate_segment_sequence()` helper on
resolved wire messages, but `values_for_message` did not use it. That meant a
Rust `MSGdef` mapping could accept a message whose segment numbers would be
rejected by hbci4java's default incoming `MSG` parsing.

## Decision

Run segment sequence validation by default when mapping values through a
specific `MSGdef`.

`ResolvedWireMessage::values_for_message(...)` now checks that resolved segment
sequence numbers start at 1 and increase by 1. The lower-level flat
`values(...)` method remains an intermediate inspection view and does not
validate sequence numbers automatically.

`IncomingValidation` now also carries the sequence-check flag. Strict validation
checks both valid values and segment sequence numbers. Callers can opt out with
`with_segment_sequence_check(false)`, preserving hbci4java's
`DONT_CHECK_SEQ`-style escape hatch.

## Consequences

Message-level incoming parsing is closer to hbci4java defaults and catches bad
wire messages earlier.

Replay fixtures and rewrite ports that intentionally parse out-of-sequence
messages can still disable this check without disabling XML default validation.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.MSG`
- Upstream: `org.kapott.hbci.protocol.SEG.checkSegSeq`
