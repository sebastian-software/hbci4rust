# ADR 0024: Present Optional Syntax Functions

## Status

Accepted

## Context

hbci4java parses `MSG` definitions through `SEG` and `SF` child containers. Its
`SF` parser peeks at the next wire segment code and version before trying a
candidate segment, which keeps optional syntax functions such as `BPD` and
`UPD` from consuming unrelated segments.

The Rust incoming message mapper currently supports direct `SEG` children and
skips absent optional `SF` children. Before full `SF` reconstruction is ported,
blindly skipping a present optional `SF` can turn a real BPD/UPD response into a
misleading later error, for example reporting a missing `MsgTail`.

## Decision

When direct `MSGdef` mapping reaches an optional `SF`, inspect the next resolved
wire segment. If that segment matches any direct or nested `SEG` child of the
referenced `SFdef` by segment code and version, reject the message with
`Unsupported` and name the present optional syntax function.

If no segment matches, keep treating the optional `SF` as absent and continue
with the next `MSGdef` child.

## Consequences

The parser now fails earlier and more honestly when a response contains BPD,
UPD, or another optional syntax function that has not been ported yet.

This is still not full hbci4java `SF` parsing. The actual reconstruction of
`BPD`, `UPD`, `Params`, `GVRes`, and nested syntax functions remains a later
porting slice.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.SF`
- Upstream: `org.kapott.hbci.protocol.MultipleSFs`
