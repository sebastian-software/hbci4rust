# ADR 0083: Saldo Renderer Lowlevel Param Source

## Status

Accepted

## Context

hbci4java does not render queued jobs directly from high-level frontend
parameter names. `HBCIDialog` iterates over each task's `getLowlevelParams()`
and writes those values under the counted `GV` message header. `HBCIJobImpl`
fills that low-level map through checked `setParam(...)` calls and through
`verifyConstraints()`.

ADR 0082 added a persistent low-level parameter map to `HbciJob`, but the
current Saldo renderer still read `my.*`, `dummyall`, and `maxentries` directly
from the frontend parameter map.

## Decision

Change the Saldo renderer to prefer low-level parameter values:

- `Saldo7.KTV.iban` before `my.iban`;
- `Saldo7.KTV.bic` before `my.bic`;
- `Saldo7.KTV.KIK.country` before `my.country`;
- `Saldo7.KTV.KIK.blz` before `my.blz`;
- `Saldo7.KTV.number` before `my.number`;
- `Saldo7.KTV.subnumber` before `my.subnumber`;
- `Saldo7.allaccounts` before `dummyall`;
- `Saldo7.maxentries` before `maxentries`.

Keep the frontend fallback in this slice. Earlier handler tracers intentionally
allowed direct `set_param(...)` staging before low-level state existed, and
removing that compatibility would mix a renderer-source change with a broader
API behavior change.

## Consequences

Checked setters and account helpers now feed the Saldo renderer through the same
kind of low-level state that hbci4java uses.

A replay test deserializes an `HbciJob` with only low-level parameters and
verifies that the outgoing `HKSAL` segment is rendered from those values.

Remaining work:

- route `verify_constraints()` defaults into persistent low-level state before
  rendering;
- replace the current hand-written Saldo renderer with a generic low-level GV
  segment renderer;
- remove frontend fallbacks once all supported job setup paths write low-level
  values;
- preserve ADR 0036's passport account fallback as an explicit runtime rule.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0036-passport-account-fallback-tracer.md`
- `docs/adr/0082-job-lowlevel-param-store-tracer.md`
- Upstream: `org.kapott.hbci.manager.HBCIDialog`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#getLowlevelParams`
