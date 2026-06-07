# ADR 0134: HKTAN TAN2Step5 Rendering

## Status

Accepted

## Context

The Rust handler currently renders a small set of queued business transactions
but not the `TAN2Step` job, even though `TAN2Step` is already in the PinTAN job
registry.

hbci4java's PinTAN runtime creates `GVTAN2Step` jobs for HKTAN flows. The
constructor adds original frontend constraints such as `process`,
`ordersegcode`, `orderhash`, `challengeklass`, and
`ChallengeKlassParam1..9`. For segment version 5 it renders the protocol
segment `TAN2Step5`, whose wire code is `HKTAN`.

ADR 0131 through ADR 0133 ported the challenge metadata, parameter formatting,
and apply-params helper. ADR 0132 pinned the positional rendering behavior of
`ChallengeKlassParams`.

## Decision

Add the first queued `TAN2Step` renderer for HKTAN segment version 5.

Keep this slice intentionally narrow:

- add original-near v5 constraints for `TAN2Step`;
- render queued `TAN2Step` jobs as `CustomMsg.GV.TAN2Step5`;
- force the request tag by setting the grouping element to `requested`;
- map order account, order hash, order reference, list index, TAN flags,
  challenge class, challenge params, and TAN media;
- preserve hbci4java's `GVTAN2Step.setParam("orderhash", ...)` behavior for
  frontend parameters by prefixing `B` before rendering the `Bin` value;
- keep direct low-level `TAN2Step5.orderhash` values raw so replay/import tests
  can inject already-renderable low-level data.

Do not yet auto-insert HKTAN before/with a business transaction. That belongs
to the later PinTAN dialog-flow slice, where order-hash creation, SCA process
selection, and HITAN response handling can be tested together.

## Consequences

The handler can now render an explicit offline `TAN2Step` job and can consume
the challenge params produced by `ChallengeInfo::apply_params(...)`.

Remaining work:

- calculate order hashes from rendered business transaction segments;
- auto-create HKTAN process 1 and process 2 messages from BPD/SCA metadata;
- parse HITAN responses and drive callback/TAN entry flows.

## Links

- `src/gv/mod.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0131-secmech-challenge-info-parser.md`
- `docs/adr/0132-hktan-challenge-params-position-parity.md`
- `docs/adr/0133-challenge-info-apply-params-helper.md`
- Upstream: `org.kapott.hbci.GV.GVTAN2Step`
