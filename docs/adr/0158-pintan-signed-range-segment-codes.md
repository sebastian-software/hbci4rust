# ADR 0158: PinTAN Signed Range Segment Codes

## Status

Accepted

## Context

ADR 0157 added the passport-side BPD lookup for whether a segment code needs a
one-step TAN. The signer still needs the list of segment codes from the exact
message data hbci4java passes into `HBCIPassportPinTan.sign(...)`.

Upstream uses `AbstractPinTanPassport.collectSegCodes(String msg)` on the
signed range. It:

- starts at the current position and reads until `:`;
- treats that prefix as the segment code;
- skips to the next segment delimiter `'`;
- respects FinTS quoting with `?`;
- skips length-prefixed binary blocks `@len@...`;
- repeats until no further segment header is found.

## Decision

Add `collect_pintan_segment_codes(...)` in the manager signature module:

- accept the already collected signed range;
- return segment codes in range order;
- preserve admin/signature segment codes such as `HNSHK`, because hbci4java
  passes them through to `getPinTanInfo(...)`;
- skip FinTS quoted delimiters and binary blocks while looking for the next
  segment delimiter.

Do not derive codes from queued jobs or parsed message models in this slice.
Those sources are convenient, but they are not the same input boundary as the
original PinTAN signer.

## Consequences

The one-step signer can now combine:

- the signed range from ADR 0151;
- segment code collection from this ADR;
- BPD `PinTanGV` lookup from ADR 0157.

Remaining work:

- call this helper from the PinTAN `UserSig` path for one-step method `999`;
- request only one TAN even if multiple `J` segments are found;
- keep two-step SCA challenge handling unchanged.

## Links

- `docs/adr/0151-pintan-signature-range-collection.md`
- `docs/adr/0157-pintan-segment-tan-info.md`
- `src/manager/signature.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.collectSegCodes`
