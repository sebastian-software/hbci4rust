# ADR 0157: PinTAN Segment TAN Info

## Status

Accepted

## Context

ADR 0156 signed `CustomMsg`, but the one-step PinTAN signer still does not
inspect the signed message range to decide whether a TAN is required.

hbci4java's `HBCIPassportPinTan.sign(...)` handles one-step method `999`
differently from two-step methods:

- collect segment codes from the signed range;
- call `AbstractPinTanPassport.getPinTanInfo(code)` for each code;
- request one TAN if any code is marked `J`;
- log and continue for `N`;
- warn for unknown/empty business transaction entries.

`getPinTanInfo(...)` reads the BPD `PinTanGV` entries:

- `Params*.PinTanPar*.ParPinTan*.PinTanGV*.segcode`;
- the sibling `needtan` value;
- `Params*.SegHead.code`, converted from `HK...` to parameter code `HI...S`,
  to distinguish known business transactions from admin segments;
- admin segments return `A` and do not trigger a TAN.

## Decision

Add a narrow Rust-native `PinTanPassport::pin_tan_info_for_segment_code(...)`
helper with original-near return values:

- return `Some("J")` or `Some("N")` when a matching `PinTanGV` BPD entry is
  found;
- return `Some("A")` for admin segments that are not known business
  transactions in the BPD;
- return `None` when the passport has no BPD data, or when a known business
  transaction has no matching `PinTanGV` entry.

Keep TAN-verify mode out of scope because v1 does not port that Java key
management workflow.

## Consequences

The handler signer can now ask the passport the same question hbci4java asks
before prompting for a one-step TAN, while keeping BPD traversal close to the
original property layout.

Remaining work:

- collect segment codes from the signed range with FinTS quote and binary-block
  awareness;
- request one-step TANs from `CustomMsg` signing based on these codes;
- preserve two-step SCA challenge behavior unchanged.

## Links

- `src/passport/pintan.rs`
- `src/tools.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.getPinTanInfo`
- Upstream: `org.kapott.hbci.passport.HBCIPassportPinTan.sign`
