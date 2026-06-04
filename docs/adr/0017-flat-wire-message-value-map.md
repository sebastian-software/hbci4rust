# ADR 0017: Flat Wire Message Value Map

## Status

Accepted

## Context

Resolved wire segments can now expose flat segment-level value maps. Replay
fixtures and early incoming-message tests need a convenient way to inspect a
whole parsed wire message before full `MSG` and `SF` reconstruction is ported.

hbci4java ultimately exposes message data as path/value properties. The Rust
port should move toward that shape while avoiding premature assumptions about
message container placement.

## Decision

Add `ResolvedWireMessage::values(&ProtocolSyntax)`.

The method aggregates the flat value maps of all resolved segments into one
`BTreeMap<String, String>`.

Segment roots use the resolved XML `SEGdef` id, matching ADR 0014. If the same
segment definition appears multiple times, later occurrences receive the same
counter suffix convention already used elsewhere in the port:

- first occurrence: `RetGlob`
- second occurrence: `RetGlob_2`
- third occurrence: `RetGlob_3`

The aggregation does not validate segment sequence numbers automatically and
does not infer `MSG` or `SF` context paths.

## Consequences

Offline replay tests can now assert whole-message incoming values without
waiting for the full hbci4java message parser port.

The map is explicitly an intermediate representation. Later `MSG`/`SF` parsing
can either reuse the extracted segment values or produce more specific paths
once message container placement is implemented.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- ADR 0014: Incoming Segment Value Extraction
- ADR 0016: Wire Segment Sequence Validation
