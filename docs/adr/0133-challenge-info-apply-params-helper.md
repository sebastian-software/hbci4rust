# ADR 0133: Challenge Info Apply Params Helper

## Status

Accepted

## Context

ADR 0131 ported `ChallengeInfo` parsing and parameter formatting. ADR 0132
then pinned `HKTAN` `ChallengeKlassParams` rendering so missing middle
parameters keep their positions.

hbci4java's `ChallengeInfo.applyParams(...)` joins these two pieces:

- look up challenge data by business transaction code;
- derive the HHD challenge spec from the selected security mechanism;
- set `challengeklass`;
- iterate challenge parameters in XML order;
- skip parameters whose BPD condition is not complied;
- resolve the source value from the originating job;
- format it according to the declared challenge parameter type;
- set `ChallengeKlassParamN` using the original one-based parameter position.

One important upstream boundary is `SegHead.code`: hbci4java's default
`HBCIJobImpl.getChallengeParam(...)` returns the business transaction code for
that path instead of reading a low-level parameter.

## Decision

Add a map-based Rust helper before wiring the full PinTAN runtime:

- `ChallengeInfo::apply_params(job_code, task_params, secmech)` returns
  `Option<AppliedChallengeParams>`;
- `None` means hbci4java would have skipped application because the job or HHD
  version has no challenge data;
- `AppliedChallengeParams` stores the challenge class plus a sparse
  one-based parameter map;
- `AppliedChallengeParams::to_hktan_params()` emits Java-style frontend keys
  such as `challengeklass` and `ChallengeKlassParam2`;
- `AppliedChallengeParams::to_message_params(segment_path)` emits direct Rust
  message paths such as
  `CustomMsg.GV.TAN2Step5.ChallengeKlassParams.param2`.

Keep this helper independent of `HbciHandler` for now. The current Rust job
model does not yet cover all PinTAN-compatible job classes, special
`getChallengeParam(...)` overrides, or the generated HKTAN job object. A
map-based helper lets the next runtime slice adapt job-specific data without
changing the already-tested ChallengeInfo logic.

## Consequences

The port now has the deterministic core of `ChallengeInfo.applyParams(...)`
available for PinTAN integration.

Remaining work:

- add job-specific challenge value adapters, including multi-SEPA sum values
  once those jobs are ported;
- wire the helper into actual HKTAN request generation;
- add replay fixtures for HITAN/HKTAN challenge flows.

## Links

- `src/manager/secmech.rs`
- `tests/secmech.rs`
- `docs/adr/0131-secmech-challenge-info-parser.md`
- `docs/adr/0132-hktan-challenge-params-position-parity.md`
- Upstream: `org.kapott.hbci.manager.ChallengeInfo.applyParams`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl.getChallengeParam`
