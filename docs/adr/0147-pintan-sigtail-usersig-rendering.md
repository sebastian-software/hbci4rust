# ADR 0147: PinTAN SigTail UserSig Rendering

## Status

Accepted

## Context

hbci4java's generic `Sig` layer handles the result returned by
`HBCIPassportPinTan.sign(...)` specially when `passport.needUserSig()` is true:

- decode the raw signature bytes with `UserSig.decode(...)`;
- propagate the decoded PIN to `SigTail.UserSig.pin`;
- propagate `SigTail.UserSig.tan` only when the decoded TAN is non-empty;
- otherwise leave the cryptographic `SigTail.sig` field unset.

The Rust port can now produce PinTAN `UserSig` bytes for the SCA path, and the
protocol message model can already render `HNSHA` (`SigTailUser`) segments, but
there is no small bridge that applies decoded `UserSig` values to a message
tail.

## Decision

Add a narrow manager helper:

- expose `apply_pintan_user_sig_to_sig_tail(...)`;
- accept a mutable `HbciMessage`, a signature-tail path such as
  `DialogEnd.SigTail`, and raw UserSig bytes;
- decode with the already ported `UserSig`;
- set `<sigTail>.UserSig.pin` unconditionally;
- set `<sigTail>.UserSig.tan` only when the decoded TAN is non-empty;
- leave `<sigTail>.sig` untouched.

Keep this helper outside `protocol` so the lower-level message model does not
depend on PinTAN passport concepts.

Do not fill `SigHead`, collect hash data, enumerate multiple passports, or wire
automatic signing into handler rendering in this slice.

## Consequences

The Rust port can now render the observable PinTAN `HNSHA` user-signature data
shape that hbci4java emits after decoding `UserSig`. This is the first concrete
piece of the future full `Sig` layer.

Remaining work:

- port `Sig.fillSigHead(...)` for PinTAN security metadata;
- collect sign/hash data around the signed message range;
- integrate PinTAN signing into `DialogInit`, `DialogEnd`, and `CustomMsg`
  rendering;
- support multiple signature tails if the original multi-passport machinery is
  later brought into scope.

## Links

- `src/manager/signature.rs`
- `src/protocol/message.rs`
- `src/passport/user_sig.rs`
- Upstream: `org.kapott.hbci.security.Sig`
- Upstream: `org.kapott.hbci.passport.UserSig`
