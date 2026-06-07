# ADR 0146: PinTAN SCA UserSig Helper

## Status

Accepted

## Context

hbci4java's `HBCIPassportPinTan.sign(...)` combines three PinTAN pieces:

- ensure a PIN is available, asking callback reason `NEED_PT_PIN` if needed;
- for two-step methods, inspect stored SCA/HITAN state and ask for a TAN only
  when a challenge is present and SCA was not exempted by return code `3076`;
- return `UserSig.encode(getPIN(), tan)`.

The Rust port now has each piece as a small original-near building block:

- `HbciHandler::request_pin()` asks and caches the PIN;
- `HbciHandler::request_tan_for_sca()` asks for the SCA TAN when needed;
- `UserSig::encode(...)` implements the PinTAN user signature byte boundary.

They are not yet connected, so callers still have to reproduce the ordering
from hbci4java manually.

## Decision

Add an explicit `HbciHandler::sign_pintan_user_sig_for_sca()` helper:

- keep it async because it may invoke the configured callback;
- ask/cache PIN first, matching `HBCIPassportPinTan.sign(...)`;
- ask for a TAN from the current SCA state second;
- encode the result with `UserSig::encode(Some(pin), tan.as_deref())`;
- return raw UserSig bytes, leaving `HNSHK`/`HNSHA` rendering to a later
  signature-layer port.

Do not port one-step segment-code TAN detection, decoupled callback variants,
log filtering, SCA state clearing, or full message-signature rendering in this
slice.

## Consequences

The Rust port can now produce the same PinTAN user-signature payload bytes that
hbci4java's passport returns for the two-step SCA path. This gives the future
signature-message renderer a concrete, tested input without forcing that larger
port now.

Remaining work:

- render `SigHead`/`SigTail` (`HNSHK`/`HNSHA`) around outgoing messages;
- inject decoded `UserSig.pin` and optional `UserSig.tan` into `SigTail`;
- port one-step PIN/TAN signing rules;
- clear consumed SCA runtime state at the same lifecycle point as hbci4java;
- add QR/photoTAN/decoupled-specific callback payloads.

## Links

- `src/manager/handler.rs`
- `src/passport/user_sig.rs`
- Upstream: `org.kapott.hbci.passport.HBCIPassportPinTan.sign`
- Upstream: `org.kapott.hbci.passport.UserSig`
- Upstream: `org.kapott.hbci.security.Sig`
