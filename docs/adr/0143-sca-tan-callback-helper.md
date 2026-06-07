# ADR 0143: SCA TAN Callback Helper

## Status

Accepted

## Context

hbci4java collects the TAN for two-step PinTAN signing in
`HBCIPassportPinTan.sign(...)`. After a previous HKTAN/HITAN step stored SCA
data in the passport, signing:

- skips TAN collection when return code `3076` marked the order as SCA-exempt;
- skips TAN collection when no challenge was stored;
- builds a callback message from the selected security mechanism name,
  `inputinfo`, and the HITAN challenge;
- invokes callback reason `HBCICallback.NEED_PT_TAN` with data type
  `TYPE_TEXT`;
- treats an empty callback response as an error;
- later encodes the returned TAN in the PinTAN user signature.

The Rust port already stores the short-lived SCA challenge, HHD-UC payload, and
order reference after a HITAN response, but it cannot yet ask the application
for the final TAN.

## Decision

Add an explicit async handler helper for the next original-near building block:

- expose `HbciHandler::request_tan_for_sca()`;
- inspect the runtime SCA state from `PinTanPassport`;
- return `Ok(None)` when SCA was exempted by `3076` or no challenge is present;
- require the configured async callback when a challenge is present;
- call the callback with reason `NeedPtTan`, data type `Text`, and a
  Java-shaped message of `name`, `inputinfo`, blank line, then challenge;
- pass the stored `challenge_hhd_uc` as `current_value` for now;
- return the callback response as `Some(tan)` without storing it in the
  passport.

Do not port PinTAN user-signature rendering, specialized QR/photoTAN/decoupled
callback reasons, automatic dialog repeats, or process-2 HKTAN queue patching
in this slice.

## Consequences

The Rust PinTAN runtime can now perform the same user-facing TAN callback that
hbci4java performs immediately before signing. The helper stays explicit until
the PinTAN signature/message layer is ported.

Remaining work:

- encode the returned TAN into the PinTAN user signature;
- clear consumed SCA runtime state at the same lifecycle point as signing;
- add specialized QR/photoTAN/decoupled callback variants once the public
  callback surface is decided;
- wire the helper into automatic HKTAN queue patching and dialog repeats.

## Links

- `src/manager/handler.rs`
- `src/passport/pintan.rs`
- Upstream: `org.kapott.hbci.passport.HBCIPassportPinTan.sign`
- Upstream: `org.kapott.hbci.callback.HBCICallback.NEED_PT_TAN`
