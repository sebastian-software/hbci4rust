# ADR 0151: PinTAN Signature Range Collection

## Status

Accepted

## Context

hbci4java's `Sig.collectHashData(...)` builds the byte/string range that is
passed into the passport `hash(...)` and then `sign(...)`. In the common
single-passport `range = 1` case this range contains:

- the current `SigHeadUser` (`HNSHK`);
- all message payload segments between signature head and signature tail;
- no `MsgHead` (`HNHBK`);
- no `SigTailUser` (`HNSHA`);
- no `MsgTail` (`HNHBS`).

For PinTAN, `HBCIPassportPinTan.hash(...)` returns the collected data unchanged,
and `sign(...)` may inspect that data for one-step TAN decisions before returning
`UserSig` bytes. The Rust port can render a deterministic PinTAN signature shell
but does not yet expose the collected signed range.

## Decision

Add a narrow manager helper:

- expose `collect_pintan_signature_range(...)`;
- accept an `HbciMessage`, a `SigHead` path, and a `SigTail` path;
- find both paths in the top-level message child order;
- render and concatenate top-level elements from the head up to, but excluding,
  the tail;
- reject missing paths or a tail that appears before the head.

This helper intentionally covers the v1 single-PinTAN-passport shape first. It
does not implement hbci4java's nested multi-passport shell ranges.

Do not call the PinTAN passport `sign(...)`, inspect one-step TAN-required job
codes, or wire handler signing in this slice.

## Consequences

The Rust port now has the same observable message range that hbci4java hands to
the PinTAN passport immediately before creating `UserSig` bytes. This gives the
future handler signer a concrete input for one-step TAN detection and replay
fixtures.

Remaining work:

- use the collected range in the PinTAN signing path;
- port one-step segment-code TAN-required detection;
- support multi-passport/nested signature shells if they ever enter scope;
- integrate full signing into `DialogInit`, `DialogEnd`, and `CustomMsg`.

## Links

- `src/manager/signature.rs`
- `src/protocol/message.rs`
- Upstream: `org.kapott.hbci.security.Sig.collectHashData`
- Upstream: `org.kapott.hbci.passport.HBCIPassportPinTan.hash`
