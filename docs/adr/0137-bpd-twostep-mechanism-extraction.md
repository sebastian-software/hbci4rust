# ADR 0137: BPD Two-Step Mechanism Extraction

## Status

Accepted

## Context

hbci4java stores BPD data as flat `Properties` and extracts available PinTAN
two-step mechanisms in `AbstractPinTanPassport.setBPD(...)`. For each BPD
property ending in `secfunc` below a `Params*.TAN2StepParY.ParTAN2Step*`
header, it creates an entry keyed by the security function. If the same
security function appears in several HKTAN segment versions, the newer segment
version wins.

The Rust port already keeps flat BPD parameters and can generate a process-1
HKTAN helper, but it still uses an explicit `tan_segment_version` fallback. That
keeps the helper testable but does not match hbci4java's selected security
mechanism flow.

## Decision

Add a Rust-native but original-near `twostep_mechanisms` map to
`PinTanPassportData`:

- keep the flat `bpd_parameters` as the source of truth;
- derive `twostep_mechanisms` from BPD using the hbci4java `setBPD` pattern;
- key entries by `secfunc`;
- copy sibling parameters from the same BPD header using their final property
  name, matching hbci4java's `lastIndexOf('.')` behavior;
- store `segversion` in each entry;
- prefer higher `segversion` when the same `secfunc` appears more than once;
- make current SecMech lookup use the selected `tan_method` when available;
- keep the explicit `tan_segment_version` field as a fallback until full TAN
  method selection is ported.

Do not port hbci4java's optional global maximum HITANS segment-version setting
yet. That is configuration-policy behavior and needs its own compatibility
decision.

## Consequences

The process-1 HKTAN helper can now resolve `orderhashmode`, `needorderaccount`,
and related SecMech parameters from the selected bank mechanism instead of only
from a hard-coded HKTAN version.

Remaining work:

- port the user/bank TAN method selection flow;
- import user-specific allowed TAN mechanisms from dialog responses;
- decide whether to support hbci4java's max HITANS segment-version knob;
- use selected SecMech metadata to drive automatic HKTAN queue patching.

## Links

- `src/passport/pintan.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.setBPD`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.getCurrentSecMechInfo`
