# 0213 Port LastB2BSEPA As Abstract Last SEPA Variant

## Status

Accepted

## Context

`GVLastB2BSEPA` in hbci4java extends `AbstractGVLastSEPA` and only specializes the
direct debit type to `B2B`. Its result class `GVRLastB2BSEPA` is an empty subtype
of `AbstractGVRLastSEPA`, so the observable result surface is the same order-id
container used by the already ported `LastSEPA` and `LastCOR1SEPA` jobs.

The pinned upstream protocol resources define HBCI 300 `LastB2BSEPA1` as segment
code `HKBSE` version 1 and `LastB2BSEPARes1` as segment code `HIBSE` version 1
with an optional `orderid`.

`AbstractGVLastSEPA` uses `SepaVersion.PAIN_008_001_01` as the default PAIN
descriptor for all of these dated single direct debit jobs, including B2B. We keep
that original-near behavior even though the concrete `GVLastB2BSEPA` source
comment references a different PAIN family.

## Decision

Port `LastB2BSEPA` as a thin sibling of `LastSEPA` and `LastCOR1SEPA`:

- expose frontend job name `LastB2BSEPA`;
- map constraints to lowlevel segment `LastB2BSEPA1`;
- set `type` default to `B2B`;
- set `batchbook` default to `0` for the single-order job;
- render request segment `HKBSE`;
- parse response segment `HIBSE`;
- reuse `GvrLastSepa`/`HbciJobResultData::LastSepa` for the order-id-only result;
- persist the generated request snapshot under `termlast_<orderid>` like
  hbci4java does.

Do not add multi-order B2B direct debit support in this slice.

## Consequences

This keeps the port package-near and behavior-near while avoiding another result
type that would only wrap the same optional order id. The shared Last-SEPA
renderer and persistence helper now carry the lowlevel segment name as explicit
configuration.

If later bank research or live replay fixtures show that B2B requires a newer PAIN
descriptor by default, that will be a compatibility decision with its own ADR
rather than a silent cleanup in this original-near porting phase.
