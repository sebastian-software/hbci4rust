# ADR 0110: KUmsAllCamt Request Tracer

## Status

Accepted

## Context

hbci4java exposes `KUmsAllCamt` as a CAMT-based account-turnover request. The
job uses low-level segment `KUmsZeitCamt`, FinTS segment code `HKCAZ`, and
response segment `HICAZ`.

The upstream job is close to `KUmsAll`, but uses CAMT-specific fields:

- `formats.suppformat`, defaulting to a CAMT.052 URN;
- `allaccounts`, defaulting to `N`;
- optional `startdate`, `enddate`, `maxentries`, and `offset`.

Upstream result extraction stores received CAMT documents in
`GVRKUms.camtBooked` and `GVRKUms.camtNotBooked`, then parses them through the
SEPA/CAMT parser stack into `BTag`/`UmsLine` data.

## Decision

Add a first `KUmsAllCamt` tracer that supports:

- Java-compatible job creation and constraints;
- checked queue admission, including account CRC checks;
- rendering `KUmsZeitCamt1` into `CustomMsg` requests;
- raw response content collection for `KUmsZeitCamtRes1`;
- structured `HbciJobResultData::KUms(GvrKUms)` containing raw CAMT documents in
  `camt_booked` and `camt_not_booked`.

Use `urn:iso:std:iso:20022:tech:xsd:camt.052.001.01` as the default
`suppformat`, matching hbci4java's `GVKUmsAllCamt#getDefaultPainVersion()`.

Do not parse CAMT XML into booking-day or transaction-line structures in this
slice. That requires the broader SEPA/CAMT parser port and original CAMT
fixtures.

## Consequences

`KUmsAllCamt` becomes executable in the same offline replay harness as `KUmsAll`
and `KUmsNew`, while keeping the larger CAMT parser work isolated.

Callers can inspect received CAMT documents immediately through the typed KUms
result shell. Detailed CAMT turnover lines remain unavailable until the parser
port lands.
