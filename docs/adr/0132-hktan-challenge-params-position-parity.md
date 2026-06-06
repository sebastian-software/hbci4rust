# ADR 0132: HKTAN Challenge Params Position Parity

## Status

Accepted

## Context

ADR 0131 ported `ChallengeInfo` parsing and parameter formatting, but deferred
the `applyParams(...)` runtime hook that writes the selected challenge class
and parameters into an `HKTAN` job.

hbci4java's `ChallengeInfoTest.testDEG` protects a subtle wire-format rule:
`ChallengeKlassParams.param1` through `param9` are positional. Missing
parameters at the start or in the middle must remain empty fields so later
parameters keep their original positions. Trailing missing parameters may be
trimmed.

The upstream example renders a `TAN2Step5` segment with `param1` and `param4`
missing, while `param2`, `param3`, and `param5` are present.

## Decision

Port the upstream DEG/HKTAN position-preservation check as a protocol-message
golden test before adding full runtime `ChallengeInfo.applyParams(...)`
integration.

Keep this slice in the protocol/message test layer:

- build `CustomMsg` from the original `hbci-300.xml` syntax;
- request `CustomMsg.GV.TAN2Step5` explicitly, matching hbci4java's
  request-tag behavior;
- set the same message, account, order hash, challenge class, and
  `ChallengeKlassParams` values used by the upstream test;
- assert the full rendered FinTS message, including the same middle empty
  fields and trimmed trailing empty parameters.

Do not introduce a separate HKTAN helper yet. The next runtime slice should
connect parsed `ChallengeInfo` data to job parameters and then reuse the
already-tested message renderer.

## Consequences

This pins the most failure-prone HKTAN parameter-layout behavior now, while the
larger PinTAN dialog implementation can still arrive incrementally.

Remaining work:

- port `ChallengeInfo.applyParams(...)` as a Rust helper that fills HKTAN
  parameters from an original-near job object;
- connect that helper to PinTAN dialog message generation;
- add replay fixtures that include HITAN/HKTAN challenge transitions.

## Links

- `tests/protocol_message.rs`
- `resources/protocol/hbci-300.xml`
- `docs/adr/0131-secmech-challenge-info-parser.md`
- Upstream: `org.kapott.hbci4java.secmech.ChallengeInfoTest.testDEG`
