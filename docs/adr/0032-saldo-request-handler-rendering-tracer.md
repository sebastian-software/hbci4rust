# ADR 0032: Saldo Request Handler Rendering Tracer

## Status

Accepted

## Context

The initial async handler scaffold queued Java-named jobs but sent a placeholder
comma-separated list of job names to the communication client. That was useful
for testing async plumbing, but it did not move execution toward hbci4java's
actual flow, where queued jobs contribute low-level GV segments to `CustomMsg`.

`GVSaldoReq` is a small PinTAN-relevant tracer because hbci4java maps it to the
low-level `Saldo` GV. In FinTS 3.0, the original XML segment is `Saldo7` with
wire code `HKSAL`.

## Decision

Change `HbciHandler::execute` to render queued `SaldoReq` jobs into an offline
`CustomMsg` body instead of sending job names.

The first tracer supports:

- `SaldoReq` only;
- hbci4java's `my.iban` parameter mapped to `CustomMsg.GV[...].Saldo7.KTV.iban`;
- optional `my.bic`, `dummyall`, and `maxentries`;
- default `dummyall = N`, matching `GVSaldoReq`;
- repeated queued `SaldoReq` jobs through `GV`, `GV_2`, and later suffixes.

The handler still uses placeholder dialog metadata (`dialogid = 0`, `msgnum =
1`) because full dialog initialization, BPD-driven segment-version selection,
signing, TAN handling, and response matching are not ported yet.

## Consequences

Replay tests now inspect an actual FinTS `CustomMsg` payload for `SaldoReq`.

Trying to execute other queued jobs returns `Unsupported` until their low-level
mapping slices are ported. The public registry can still create those jobs so
their Java-named parameters can be staged and tested independently.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.GV.GVSaldoReq`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl`
