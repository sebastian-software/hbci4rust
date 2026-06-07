# ADR 0144: PinTAN UserSig Encoding

## Status

Accepted

## Context

hbci4java's PinTAN passport does not create a cryptographic signature. Instead,
`HBCIPassportPinTan.sign(...)` returns the result of
`UserSig.encode(pin, tan)`. The generic `Sig` layer then decodes those bytes and
propagates the values into `SigTail.UserSig.pin` and, when non-empty,
`SigTail.UserSig.tan`.

The upstream helper is deliberately small:

- `null` PIN/TAN values become empty strings;
- the first byte stores the PIN length;
- the PIN bytes follow;
- the remaining bytes are the TAN;
- bytes are encoded with `Comm.ENCODING` (`ISO-8859-1` in the FinTS wire path);
- decode always returns two strings or raises an error for missing/invalid
  input.

The Rust port can already ask for a TAN from stored SCA state, but it has no
original-near representation of this `UserSig` byte boundary.

## Decision

Port the Java helper as a small `passport::UserSig` type:

- keep the upstream class name because it is already Rust-style UpperCamelCase
  and directly names the protocol concept;
- expose `UserSig::encode(Option<&str>, Option<&str>) -> HbciResult<Vec<u8>>`;
- expose `UserSig::decode(Option<&[u8]>) -> HbciResult<UserSig>`;
- store decoded PIN and TAN as owned strings with `pin()` and `tan()` accessors;
- encode and decode ISO-8859-1 explicitly, rejecting characters outside that
  byte range;
- reject PIN byte lengths above 255 instead of silently wrapping the length
  byte.

Do not wire this helper into full `HNSHK`/`HNSHA` rendering in this slice. The
message-signature layer still needs its own original-near port.

## Consequences

The PinTAN signing byte contract is now testable independently from the larger
message-signature machinery. Later code can combine
`request_tan_for_sca(...)`, PIN collection, and `UserSig::encode(...)` before
filling `SigTail.UserSig`.

Remaining work:

- collect PIN through `NeedPtPin` and cache it with the same lifecycle as
  hbci4java;
- render PinTAN `HNSHK`/`HNSHA` user signature segments;
- skip `SigTail.UserSig.tan` when the decoded TAN is empty;
- clear consumed SCA/TAN state after signing.

## Links

- `src/passport/user_sig.rs`
- `src/passport/mod.rs`
- Upstream: `org.kapott.hbci.passport.UserSig`
- Upstream: `org.kapott.hbci.security.Sig`
