# ADR 0150: PinTAN Deterministic Signature Shell

## Status

Accepted

## Context

hbci4java's `Sig.signIt(...)` applies the PinTAN user signature in a fixed
order:

1. create `SigHeadUser` and `SigTailUser` segments;
2. fill `SigHead` metadata;
3. copy the signature check reference into `SigTail`;
4. collect the signed message range;
5. call the PinTAN passport `sign(...)`;
6. decode the returned `UserSig` bytes into `SigTail.UserSig.pin/tan`.

The Rust port now has separate helpers for steps 2, 3, and 6. Calling those
pieces manually in tests and future handler code would make the intended
hbci4java order easy to obscure or accidentally reorder.

## Decision

Add a deterministic PinTAN signature-shell helper:

- expose `apply_pintan_signature_shell(...)`;
- accept a mutable `HbciMessage`, the `SigHead` path, the `SigTail` path, a
  prepared `PinTanSigHead`, and raw PinTAN `UserSig` bytes;
- apply `SigHead` values first;
- copy `SigTail.seccheckref` from the applied head;
- apply decoded `UserSig` values to the tail last.

The helper deliberately assumes the caller already chose deterministic
`seccheckref`, `secref`, timestamp, and signature bytes. This keeps replay tests
and later handler integration explicit while still preserving hbci4java's
observable segment-fill sequence.

Do not generate random references, read the current clock, collect hash data,
call the callback system, persist signature counters, or automatically sign
handler-rendered messages in this slice.

## Consequences

The Rust port can now render a complete observable PinTAN `HNSHK`/`HNSHA`
request shell from one original-near boundary. Future handler signing can call
this helper after it has generated the non-deterministic inputs and obtained the
PinTAN `UserSig` payload.

Remaining work:

- provide deterministic or injectable generation for check references and
  timestamps;
- collect hbci4java-like signed message ranges;
- call `sign_pintan_user_sig_for_sca(...)` from the actual handler render path;
- integrate full signing into `DialogInit`, `DialogEnd`, and `CustomMsg`.

## Links

- `src/manager/signature.rs`
- `src/protocol/message.rs`
- `src/passport/user_sig.rs`
- Upstream: `org.kapott.hbci.security.Sig.signIt`
