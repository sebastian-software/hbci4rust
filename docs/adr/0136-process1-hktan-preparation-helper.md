# ADR 0136: Process-1 HKTAN Preparation Helper

## Status

Accepted

## Context

hbci4java patches queued business jobs before sending them when the selected
PinTAN security mechanism requires two-step TAN handling. For process variant 1
it creates a separate HKTAN message before the actual order:

1. determine the selected TAN mechanism and HKTAN segment version from BPD;
2. create a `TAN2Step` job with process `1`;
3. render the original order segment with sequence number `3`;
4. hash that rendered segment with the BPD `orderhashmode`;
5. set the HKTAN `orderhash`, order segment code, optional order account, TAN
   medium, and challenge parameters.

The Rust port currently renders explicitly queued `TAN2Step` jobs but requires
tests or callers to set the order hash manually. That is not close enough to the
original process and blocks a faithful PinTAN runtime later.

## Decision

Add a narrow preparation helper before implementing full automatic queue
patching:

- keep BPD parameters in the PinTAN passport as original-near flat
  `Properties`;
- resolve `orderhashmode` through the already ported `ParameterFinder` query
  `BPD_PINTAN_ORDERHASHMODE`;
- assume HKTAN segment version `5` for the current tracer because the ported
  renderer is `TAN2Step5`;
- expose a handler helper that creates a process-1 `TAN2Step` job for a ported
  business job;
- render the originating business segment with sequence number `3`, matching
  hbci4java's `task.createJobSegment(3)` call;
- leave full message-queue insertion, HKTAN version negotiation, process
  variant 2, and final TAN submission for later slices.

## Consequences

The port can now generate the HKTAN order hash from the same rendered order
segment shape hbci4java hashes, instead of requiring callers to inject a hash.

The helper is intentionally explicit. It gives us a testable stepping stone for
the future automatic `patchMessagesFor2StepMethods` port without changing the
existing queue behavior for jobs that do not yet have PinTAN metadata.

Remaining work:

- extract the selected TAN security mechanism from BPD like
  `AbstractPinTanPassport.setBPD`;
- use bank-provided HKTAN segment versions instead of the current `TAN2Step5`
  tracer;
- insert process-1 HKTAN messages automatically before TAN-required jobs;
- port process variant 2 and final TAN submission.

## Links

- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.getOrderHashMode`
- Upstream:
  `org.kapott.hbci.passport.AbstractPinTanPassport.patchMessagesFor2StepMethods`
