# ADR 0100: KUms Raw Result Data Tracer

## Status

Accepted

## Context

`GVKUmsAll#extractResults(...)` reads the raw `booked` and `notbooked` Bin
payloads from `HIKAZ`/`HIKAN` response segments. Upstream then decodes umlauts
and appends the payloads to `GVRKUms` as MT940 and MT942 data. It also stores
the raw `notbooked` payload in generic job result data.

The Rust port can already parse incoming `Bin` values by removing the `@len@`
envelope, and ADR 0098/0099 added request-only `KUmsAll` and `KUmsNew`
renderers. The full `GVRKUms`, SWIFT umlaut decoding, MT940 parsing, and MT942
parsing are still unported.

Dropping response payloads entirely would make replay fixtures less useful for
the upcoming offline-domain parser work.

## Decision

For `KUmsAll` and `KUmsNew`, copy parsed raw response payloads into
`HbciJobResult::result_data` using the existing content-data convention:

- `content.booked`;
- `content.notbooked`.

Keep `HbciJobResult::result` as `None` for these jobs. Do not introduce a
`GvrKUms` public result variant until the MT940/MT942 parser and upstream
fixtures are ported.

Do not implement SWIFT umlaut decoding in this slice. The incoming protocol
parser exposes the string payload after `Bin` envelope removal; later SWIFT
parsing can decide where decoding belongs.

## Consequences

Replay tests for `HIKAZ` and `HIKAN` can now preserve turnover payloads without
claiming structured parsing support.

The result surface remains intentionally raw and small, but the data needed for
future MT940/MT942 golden fixtures is available.

Remaining work:

- port `GVRKUms` with raw MT940/MT942 storage;
- port SWIFT umlaut decoding;
- port MT940 and MT942 parsing using upstream fixtures;
- extend CAMT result handling separately.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0098-kums-all-request-tracer.md`
- `docs/adr/0099-kums-new-request-tracer.md`
- Upstream: `org.kapott.hbci.GV.GVKUmsAll#extractResults`
- Upstream: `org.kapott.hbci.GV_Result.GVRKUms`
- Upstream: `org.kapott.hbci.swift.Swift`
