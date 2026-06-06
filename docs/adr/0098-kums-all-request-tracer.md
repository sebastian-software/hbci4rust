# ADR 0098: KUmsAll Request Tracer

## Status

Accepted

## Context

`GVKUmsAll` is hbci4java's account-turnover request with an explicit time range
(`HKKAZ`). In the upstream constructor it registers account constraints,
optional `startdate`, `enddate`, and `maxentries` parameters, and an
`allaccounts` flag defaulting to `N`. During verification it delegates to
`HBCIJobImpl.verifyConstraints()` and then calls `checkAccountCRC("my")`.

The Rust port already has:

- the `KUmsAll` Java job name in the PinTAN registry;
- date parameter helpers;
- account parameter helpers;
- replay-based `CustomMsg` rendering for `SaldoReq` and `SaldoReqAll`;
- protocol resources containing `KUmsZeit4` through `KUmsZeit7` request
  segments and their response segments.

No MT940/SWIFT result parser is ported yet, so a full `GVRKUms` result would be
premature in this slice.

## Decision

Add a request-only `KUmsAll` tracer using FinTS 3.0 segment `KUmsZeit7`.

The tracer registers original-near constraints:

- `my.bic` -> `KUmsZeit7.KTV.bic`;
- `my.iban` -> `KUmsZeit7.KTV.iban`;
- `my.country` -> `KUmsZeit7.KTV.KIK.country` with default `DE`;
- `my.blz` -> `KUmsZeit7.KTV.KIK.blz`;
- `my.number` -> `KUmsZeit7.KTV.number`;
- `my.subnumber` -> `KUmsZeit7.KTV.subnumber` with default empty string;
- `startdate` -> `KUmsZeit7.startdate` with default empty string;
- `enddate` -> `KUmsZeit7.enddate` with default empty string;
- `maxentries` -> `KUmsZeit7.maxentries` with default empty string;
- `dummy` -> `KUmsZeit7.allaccounts` with default `N`.

Render queued `KUmsAll` jobs into `CustomMsg.GV[...].KUmsZeit7`, using the same
passport account fallback style as the Saldo tracers and rendering only
non-empty optional date/count parameters.

Include `KUmsAll` in the account CRC callback path for checked queue admission,
matching upstream `GVKUmsAll.verifyConstraints()`.

Do not port `GVRKUms`, MT940, MT942, or SWIFT umlaut decoding in this slice.
Those belong to the offline-domain phase and need upstream fixtures.

Do not add BPD-driven segment-version selection yet. The current tracer follows
the same hard-coded modern-segment approach as the existing `Saldo7` renderer.

## Consequences

`KUmsAll` becomes the first non-balance queued request that renders as a real
FinTS GV segment.

The result surface for `KUmsAll` remains basic status/result-data only until the
MT940/SWIFT parser enters the port.

Remaining work:

- port `GVRKUms`, MT940, and MT942 parsing with original fixtures;
- add BPD-driven `KUmsZeit` segment-version selection;
- decide how to represent `canNationalAcc(...)` once BPD job parameters are
  available to the registry;
- add `KUmsNew`, `KUmsZeitSEPA`, and CAMT variants as separate focused slices.

## Links

- `src/gv/mod.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0032-saldo-request-handler-rendering-tracer.md`
- `docs/adr/0097-job-date-param-overloads.md`
- Upstream: `org.kapott.hbci.GV.GVKUmsAll`
- Upstream: `org.kapott.hbci.GV_Result.GVRKUms`
