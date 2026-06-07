# ADR 0142: HITAN SCA State Extraction

## Status

Accepted

## Context

hbci4java's PinTAN flow stores short-lived SCA data after a process-1 HKTAN
response. In `AbstractPinTanPassport.checkSCAResponse(...)` it examines the
message status and response data:

- return code `3076` means that no SCA/TAN is required;
- `TAN2StepRes*`/`HITAN` response data may contain `challenge`,
  `challenge_hhd_uc`, and `orderref`;
- challenge value `nochallenge` is intentionally ignored;
- the values are stored in passport persistent data and consumed later during
  signing/TAN entry.

The Rust port can already render process-1 HKTAN jobs and ask for TAN method
and TAN media, but it does not yet remember HITAN response data. That blocks the
later `NeedPtTan` callback and final step-2 HKTAN submission.

## Decision

Add a small runtime SCA state to `PinTanPassport`:

- keep it out of `PinTanPassportData` and encrypted passport storage;
- store `challenge`, `challenge_hhd_uc`, `orderref`, and whether SCA was
  exempted by return code `3076`;
- update this state from `CustomMsgRes` response values after `execute()`;
- scan for any `TAN2StepRes*` root in the flattened response values;
- ignore empty `challenge` values and the literal `nochallenge`;
- clear challenge/order references when `3076` is seen.

Do not port final TAN signing, decoupled refresh handling, dialog repeats, or
process-2 HKTAN submission in this slice.

## Consequences

The Rust handler can now carry the same observable HITAN/SCA data that
hbci4java stores before asking the user for a TAN.

Remaining work:

- add the final `NeedPtTan` callback and TAN response collection;
- build the process-1 step-2 HKTAN message using `orderref`;
- port process variant 2;
- port decoupled SCA refresh handling for return code `3956`.

## Links

- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.checkSCAResponse`
- Upstream: `org.kapott.hbci.passport.HBCIPassportPinTan.sign`
