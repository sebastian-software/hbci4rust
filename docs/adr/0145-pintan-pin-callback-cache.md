# ADR 0145: PinTAN PIN Callback Cache

## Status

Accepted

## Context

hbci4java's `HBCIPassportPinTan.sign(...)` asks for the PIN before it handles
TAN collection. The behavior is small but important:

- if `getPIN()` returns `null`, callback reason `HBCICallback.NEED_PT_PIN` is
  invoked;
- the callback data type is `TYPE_SECRET`;
- the default English callback message is
  `Please enter your PIN for PIN/TAN now`;
- an empty callback response raises `EXCMSG_PINZERO`;
- a non-empty callback response is cached with `setPIN(...)`;
- `clearPIN()` removes the cached PIN.

The Rust port already has callback constants, SCA TAN callback handling, and
`UserSig` encoding, but it has no original-near place to cache the PIN between
signing steps.

## Decision

Add a runtime-only PIN cache to `PinTanPassport`:

- keep the cached PIN out of `PinTanPassportData` and encrypted passport
  storage;
- expose `pin()`, `set_pin(...)`, and `clear_pin()` on `PinTanPassport`;
- expose `HbciHandler::request_pin()` as the async callback boundary;
- when a PIN is already cached, return it without invoking the callback;
- otherwise call the configured callback with reason `NeedPtPin`, data type
  `Secret`, message `Please enter your PIN for PIN/TAN now`, and no current
  value;
- reject empty callback responses with a callback error;
- cache and return non-empty callback responses.

Do not render `HNSHK`/`HNSHA`, encode `UserSig`, or clear PIN automatically
during signing in this slice.

## Consequences

The Rust PinTAN runtime now has the same PIN lifecycle boundary needed by the
later signing layer: first collect/cache PIN, then combine it with a TAN or an
SCA exemption via `UserSig`.

Remaining work:

- combine cached PIN and collected TAN with `UserSig::encode(...)`;
- render PinTAN signature head/tail segments;
- clear the cached PIN at the same lifecycle boundary as hbci4java;
- apply PIN length BPD metadata once the signing path consumes it.

## Links

- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- Upstream: `org.kapott.hbci.passport.HBCIPassportPinTan.sign`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.setPIN`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.clearPIN`
- Upstream: `org.kapott.hbci.callback.HBCICallback.NEED_PT_PIN`
