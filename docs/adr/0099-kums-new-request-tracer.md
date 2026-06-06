# ADR 0099: KUmsNew Request Tracer

## Status

Accepted

## Context

`GVKUmsNew` is hbci4java's request for newly available account turnovers
(`HKKAN`). It subclasses `GVKUmsAll`, registers account constraints, optional
`maxentries`, and an `allaccounts` flag defaulting to `N`, then calls
`checkAccountCRC("my")` during verification.

The upstream constructor also registers `my.curr -> curr` with default `EUR`.
That field exists in older `KUmsNew4`, but the modern FinTS 3.0 `KUmsNew7`
segment used by this Rust tracer has no `curr` data element.

ADR 0098 introduced a request-only `KUmsAll` tracer using the modern
`KUmsZeit7` segment and explicitly left related turnover request jobs for later
focused slices.

## Decision

Add a request-only `KUmsNew` tracer using FinTS 3.0 segment `KUmsNew7`.

The tracer registers original-near constraints that are renderable by
`KUmsNew7`:

- `my.bic` -> `KUmsNew7.KTV.bic`;
- `my.iban` -> `KUmsNew7.KTV.iban`;
- `my.country` -> `KUmsNew7.KTV.KIK.country` with default `DE`;
- `my.blz` -> `KUmsNew7.KTV.KIK.blz`;
- `my.number` -> `KUmsNew7.KTV.number`;
- `my.subnumber` -> `KUmsNew7.KTV.subnumber` with default empty string;
- `maxentries` -> `KUmsNew7.maxentries` with default empty string;
- `dummyall` -> `KUmsNew7.allaccounts` with default `N`.

Do not register `my.curr` for this hard-coded `KUmsNew7` tracer, matching the
same modern-segment boundary used for `Saldo7` and `KUmsZeit7`.

Render queued `KUmsNew` jobs into `CustomMsg.GV[...].KUmsNew7`, using the same
passport account fallback and optional `maxentries` handling as the `KUmsAll`
tracer.

Include `KUmsNew` in the account CRC callback path for checked queue admission,
matching upstream `GVKUmsNew.verifyConstraints()`.

Do not port `GVRKUms`, MT940, MT942, or SWIFT umlaut decoding in this slice.

## Consequences

The Rust port can now queue and render both turnover request variants:

- `KUmsAll` / `HKKAZ` for a time range;
- `KUmsNew` / `HKKAN` for newly available turnovers.

The result surface remains basic status/result-data until the MT940/SWIFT
offline parser is ported.

Remaining work:

- port `GVRKUms`, MT940, and MT942 parsing with original fixtures;
- add BPD-driven `KUmsNew` segment-version selection;
- revisit `my.curr` when lower `KUmsNew` segment versions or BPD-driven version
  selection enter scope;
- add CAMT variants as separate focused slices.

## Links

- `src/gv/mod.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0098-kums-all-request-tracer.md`
- Upstream: `org.kapott.hbci.GV.GVKUmsNew`
- Upstream: `org.kapott.hbci.GV_Result.GVRKUms`
