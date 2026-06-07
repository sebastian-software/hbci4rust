# ADR 0198: DauerList Job

## Status

Accepted

## Context

`GVDauerList` queries existing non-SEPA standing orders. It is the classic
counterpart to the already ported `DauerSEPAList` job and returns
hbci4java's `GVRDauerList`.

For HBCI 300 the protocol XML provides `DauerList1` through `DauerList5`.
Versions 4 and 5 use `KTV3`; version 5 adds the optional capability flags
`canchange`, `canskip`, and `candel` in the response. The Java job constructor
adds constraints for national account data, optional `orderid`, and optional
`maxentries`. It also checks the account CRC for `my`.

## Decision

Port `DauerList` as an original-near classic standing-order query job.

- Use `DauerList5` / `HKDAB` version 5 for HBCI 300 requests and
  `DauerListRes5` / `HIDAB` version 5 for responses.
- Keep Java-compatible frontend parameters: `my.country`, `my.blz`,
  `my.number`, `my.subnumber`, `orderid`, and `maxentries`.
- Render the account as national `KTV3` under `KTV`, matching the HBCI 300
  segment definition.
- Reuse the existing `GvrDauerList` and `GvrDauerListEntry` result structures
  instead of creating a separate classic-standing-order result type.
- Extend the `GvrDauerListEntry` extraction path so it reads classic response
  fields (`Other`, `BTG`, `key`, `addkey`, `usage`, `date`, and
  `DauerDetails`) when no SEPA `sepapain` payload is present.
- Preserve optional capability flags from `DauerListRes5`; if they are absent,
  keep the current default of allowing change, skip, and delete, matching the
  earlier SEPA-list tracer behavior.
- Store `dauer_<orderid>` passport snapshots from classic list results using
  the same content-data snapshot helper as `DauerSEPAList`.
- Verify account CRC for `my`, matching hbci4java's `checkAccountCRC("my")`.

## Consequences

This adds the classic standing-order list job without widening the public API
style. Reusing `GvrDauerList` keeps the Java result-family shape intact, while
the parser now supports both SEPA payload-derived entries and classic direct
wire fields.
