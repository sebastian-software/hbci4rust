# ADR 0041: Response Message Reference Validation Tracer

## Status

Accepted

## Context

The handler can now render and replay a minimal lifecycle:

1. `DialogInit`
2. `CustomMsg`
3. `DialogEnd`

FinTS response message heads include `MsgRef`, which points back to the request
message being answered. The protocol mapper already exposes values like
`CustomMsgRes.MsgHead.MsgRef.dialogid` and `CustomMsgRes.MsgHead.MsgRef.msgnum`,
but the handler accepted responses without checking them.

hbci4java relies on the kernel/dialog machinery to keep request and response
messages connected. For the Rust tracer we need that invariant explicitly in the
handler while staying near the original message paths.

## Decision

Introduce an internal `MessageReference` containing the outgoing request
`dialog_id` and `msgnum`.

Each handler request builds the expected reference from the same values used to
render the outgoing message:

- `DialogInit`: `0:1`;
- `CustomMsg`: current dialog id and current message number;
- `DialogEnd`: current dialog id and current message number.

After parsing `DialogInitRes`, `CustomMsgRes`, or `DialogEndRes`, validate:

- `{MessageName}.MsgHead.MsgRef.dialogid`;
- `{MessageName}.MsgHead.MsgRef.msgnum`.

If the response reference does not match the request reference, return a
protocol error before mutating successful response state.

## Consequences

Replay tests now reject responses that answer the wrong dialog id or message
number. This makes the handler less forgiving but much closer to a real FinTS
dialog.

The response parser code now shares a small `parse_response_values(...)` helper
for UTF-8 decoding, wire parsing, syntax resolution, and message mapping.

Remaining work:

- validate response `MsgHead.dialogid` itself for non-init responses;
- validate response `MsgHead.MsgRef` against all multi-message/SCA continuation
  flows once those exist;
- decide whether message numbers should advance on responses that fail protocol
  validation;
- include response-reference details in richer status objects instead of only
  protocol errors.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `tests/protocol_wire.rs`
- `resources/protocol/hbci-300.xml`
- Upstream: `org.kapott.hbci.manager.HBCIKernelImpl`
- Upstream: `org.kapott.hbci.dialog.AbstractRawHBCIDialog`
