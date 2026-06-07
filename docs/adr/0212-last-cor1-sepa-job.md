# ADR 0212: LastCOR1SEPA Job

## Status

Accepted

## Context

`GVLastCOR1SEPA` submits a SEPA COR1 direct debit. It extends
`AbstractGVLastSEPA`, uses the lowlevel job name `LastCOR1SEPA`, defaults to
the same `SepaVersion.PAIN_008_001_01` generator path as `LastSEPA`, and returns
`GVRLastCOR1SEPA`, which only inherits the optional order id from
`AbstractGVRLastSEPA`.

For HBCI 300 the protocol XML defines `LastCOR1SEPA1` / `HKDSC` version 1. The
request has the same visible shape as `LastSEPA1`: creditor account,
`sepadescr`, and binary `sepapain`. `LastCOR1SEPARes1` / `HIDSC` version 1 may
return `orderid`.

hbci4java stores successful `AbstractGVLastSEPA` submissions under
`termlast_<orderid>`, shared across CORE, COR1, and B2B variants.

## Decision

Port `LastCOR1SEPA` as a narrow original-near variant of the existing
`LastSEPA` implementation.

- Use `LastCOR1SEPA1` / `HKDSC` version 1 for HBCI 300 requests.
- Use `LastCOR1SEPARes1` / `HIDSC` version 1 for response content extraction.
- Reuse the PAIN.008.001.01 direct-debit generator and descriptor default
  `urn:sepade:xsd:pain.008.001.01`.
- Keep the same frontend parameters and defaults as `AbstractGVLastSEPA`, but
  set `type=COR1` and keep `batchbook=0` for the single-order lowlevel job.
- Reuse the `GvrLastSepa` result shape because `GVRLastCOR1SEPA` adds no fields
  beyond the inherited optional order id.
- Store successful submitted lowlevel data under `termlast_<orderid>`, matching
  the abstract hbci4java persistence behavior.
- Defer `LastB2BSEPA`, `MultiLastCOR1SEPA`, and standing direct-debit jobs to
  separate ADRs and commits.

## Consequences

The port gains the COR1 direct-debit job without introducing new PAIN
generation logic or widening the slice to B2B/multi-order behavior. Tests can
assert only the changed segment names and `type=COR1`, while relying on the
already-ported PAIN.008 path for the shared XML shape.
