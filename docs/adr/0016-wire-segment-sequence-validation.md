# ADR 0016: Wire Segment Sequence Validation

## Status

Accepted

## Context

hbci4java validates parsed message segment sequence numbers after constructing an
incoming `MSG`. `SEG.checkSegSeq` reads `SegHead.seq`, compares it with the
expected number, and returns the next expected value.

The Rust port already parses wire segment headers and resolves them to XML
`SEGdef`s. The next parser-hardening step is to expose the same sequence check
for incoming wire messages.

## Decision

Add explicit sequence-validation methods to `ResolvedWireMessage`.

- `validate_segment_sequence()` checks that segment sequence numbers start at
  `1` and increase by one.
- `check_segment_sequence(start_value)` mirrors hbci4java's return shape by
  returning the next expected value after the last segment.
- `ResolvedWireSegment::sequence_number()` parses the header sequence component
  as `usize`.

Sequence validation is not run automatically during wire parsing or segment
resolution. Callers can inspect malformed or partially parsed messages before
choosing to validate them.

## Consequences

Incoming replay fixtures can now assert hbci4java-like sequence behavior without
requiring full `MSG`/`SF` reconstruction.

Keeping the check explicit preserves diagnostic flexibility and keeps the earlier
parser stages independently testable.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.MSG.initData`
- Upstream: `org.kapott.hbci.protocol.SEG.checkSegSeq`
