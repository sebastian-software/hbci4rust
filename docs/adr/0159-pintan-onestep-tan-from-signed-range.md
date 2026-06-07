# ADR 0159: PinTAN One-Step TAN From Signed Range

## Status

Accepted

## Context

ADRs 0157 and 0158 provide the two missing pieces for hbci4java's one-step
PinTAN signer behavior:

- BPD lookup for segment `needtan`;
- segment code collection from the signed message range.

The handler currently signs `UserSig` with PIN-only unless a two-step SCA
challenge is stored. That keeps two-step flows moving, but one-step method `999`
still cannot ask for a TAN for business jobs marked `J` in the bank parameters.

## Decision

When creating PinTAN `UserSig` bytes:

- request/cache the PIN as before;
- if the current TAN method is one-step `999`, inspect the collected signed
  range;
- collect segment codes from that range;
- ask `PinTanPassport::pin_tan_info_for_segment_code(...)` for each code;
- request exactly one TAN through callback reason `NeedPtTan` when at least one
  code is marked `J`;
- encode that TAN into `UserSig`;
- keep two-step methods on the existing SCA challenge path.

The message signing helper now applies `SigHead` and `SigTail`, prepares segment
numbers, collects the signed range, builds `UserSig`, applies it to `SigTail`,
and lets the caller perform the final outgoing preparation/message-size pass.

Keep TAN-verify mode and automatic process-2 HKTAN queue patching out of scope.

## Consequences

One-step PinTAN `CustomMsg` requests can now include `PIN:TAN` in `HNSHA` when
the BPD marks the actual signed business segment as TAN-required.

This is intentionally range-based rather than queue-based, so admin/signature
segments are seen and ignored the same way hbci4java sees them.

Remaining work:

- add broader replay fixtures covering full one-step TAN and two-step SCA
  dialog paths;
- clear consumed SCA runtime state at the exact hbci4java lifecycle point;
- port automatic HKTAN process-2 queue patching.

## Links

- `docs/adr/0157-pintan-segment-tan-info.md`
- `docs/adr/0158-pintan-signed-range-segment-codes.md`
- `src/manager/handler.rs`
- Upstream: `org.kapott.hbci.passport.HBCIPassportPinTan.sign`
