# ADR 0057: Saldo Result Display Tracer

## Status

Accepted

## Context

hbci4java's `GVRSaldoReq` result class implements `toString()` for human-readable
saldo summaries.

The original nested `GVRSaldoReq.Info.toString()` renders lines in this order:

- `Konto: <konto>`;
- `  Gebucht: <ready>`;
- optional `  Pending: <unready>`;
- optional `  Kredit: <kredit>`;
- optional `  Verfügbar: <available>`;
- optional `  Benutzt: <used>`.

The outer `GVRSaldoReq.toString()` appends each info block and trims the final
line separator.

The Rust port already has `GvrSaldoReq`, `GvrSaldoReqInfo`, and display support
for `Konto`, `Saldo`, and `Value`.

## Decision

Implement `Display` for:

- `GvrSaldoReqInfo`;
- `GvrSaldoReq`.

Use the original German labels and field order.

Join lines with `\n` as the stable Rust display boundary. Do not use platform
dependent line separators, even though hbci4java uses
`System.getProperty("line.separator")`.

Render optional fields only when present. Render an empty `GvrSaldoReq` as an
empty string.

Do not add `Display` for `HbciJobResult`, `HbciExecStatus`, or other `GVR*`
types in this slice.

## Consequences

Saldo result summaries can now be printed in a recognizable hbci4java shape.

Tests pin the line order, optional-field behavior, multiple-entry joining, and
absence of a trailing newline.

Remaining work:

- add display support for `HbciJobResult` and execution status types;
- port display output for additional PinTAN-compatible result classes as they
  are implemented;
- revisit line separator handling if an exact Java golden requires platform
  dependent output.

## Links

- `src/gv_result/mod.rs`
- `tests/structures.rs`
- Upstream: `org.kapott.hbci.GV_Result.GVRSaldoReq#toString`
- Upstream: `org.kapott.hbci.GV_Result.GVRSaldoReq.Info#toString`
