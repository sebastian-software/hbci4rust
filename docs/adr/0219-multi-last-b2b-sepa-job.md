# 0219 Port MultiLastB2BSEPA As Bulk B2B Direct Debit Variant

## Status
Accepted

## Context
`GVMultiLastB2BSEPA` in hbci4java implements SEPA B2B bulk direct debit. It
extends `GVLastB2BSEPA`, keeps the PAIN job name `LastSEPA`, and overrides the
lowlevel job name to `SammelLastB2BSEPA`.

In FinTS 3.0 the request segment is `SammelLastB2BSEPA1` with code `HKBME`,
version 1. The response segment is `SammelLastB2BSEPARes1` with code `HIBME`,
version 1, and an optional `orderid`. The request segment has the same shape as
the already ported CORE and COR1 bulk direct debit variants: source account,
optional total amount, optional `singletransfers`, PAIN descriptor, and PAIN
payload.

`GVMultiLastB2BSEPA` inherits the B2B direct-debit constraints from
`GVLastB2BSEPA` and adds:

- `batchbook -> sepa.batchbook`, default `""`;
- `Total.value -> Total.value`, required;
- `Total.curr -> Total.curr`, required.

Its `createSEPAFromParams()` delegates to the base direct-debit implementation
and then sets `Total` through `SepaUtil.sumBtgValueObject(sepaParams)`, just
like the CORE and COR1 bulk variants.

## Decision
Port `MultiLastB2BSEPA` as the next original-near PinTAN job slice:

- expose frontend job name `MultiLastB2BSEPA`;
- map constraints to `SammelLastB2BSEPA1`;
- use the existing indexed PAIN.008 generation path with debit type `B2B`;
- render `SammelLastB2BSEPA1` as `HKBME` with source account, `Total.*`,
  descriptor, and PAIN.008 payload;
- parse `SammelLastB2BSEPARes1.orderid` into the existing LastSEPA result shape;
- persist the request snapshot under `termlast_<orderid>` using
  `SammelLastB2BSEPA1`;
- keep `singletransfers` unexposed because hbci4java's constructor does not add
  it as a high-level constraint.

## Consequences
This completes the three SEPA bulk direct-debit variants in the v1 PinTAN scope:
CORE, COR1, and B2B. The renderer/result/persistence path remains shared and
original-near, with only lowlevel segment names, segment codes, response roots,
and debit type defaults varying by job.

## References
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVMultiLastB2BSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVLastB2BSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/AbstractGVLastSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/generators/GenLastSEPA00800101.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
