# ADR 0109: Return Structured KUms Handler Results

## Status

Accepted

## Context

The current KUms handler port renders `KUmsAll` and `KUmsNew` requests and
stores raw response content under `content.booked` and `content.notbooked`.
hbci4java also maps those response fields into the job's `GVRKUms` result:
`booked` is decoded through `Swift.decodeUmlauts` and appended as MT940 data,
while `notbooked` is decoded and appended as MT942 data.

## Decision

Add `HbciJobResultData::KUms(GvrKUms)` and populate it for `KUmsAll` and
`KUmsNew` responses when either `booked` or `notbooked` content exists.

The structured result keeps the original mapping:

- response `booked` becomes MT940 input
- response `notbooked` becomes MT942 input
- both inputs pass through Swift umlaut decoding before being appended

The existing raw `result_data` content mapping remains unchanged. It is useful
for Java-near inspection and preserves response bytes before Swift decoding.

## Consequences

`KUmsAll` and `KUmsNew` now behave like `SaldoReq`: callers can read typed result
data without manually looking up low-level response keys. The handler still only
returns a structured KUms result when the response contains KUms content, so
empty successful responses continue to produce no typed job result.

CAMT-specific KUms result integration stays outside this slice.
