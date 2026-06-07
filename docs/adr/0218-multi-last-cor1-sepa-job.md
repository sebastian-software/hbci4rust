# 0218 Port MultiLastCOR1SEPA As Bulk COR1 Direct Debit Variant

## Status

Superseded by ADR 0265 and ADR 0271

## Context
`GVMultiLastCOR1SEPA` in hbci4java implements SEPA COR1 bulk direct debit. It
extends `GVLastCOR1SEPA`, keeps the PAIN job name `LastSEPA`, and overrides the
lowlevel job name to `SammelLastCOR1SEPA`.

In FinTS 3.0 the request segment is `SammelLastCOR1SEPA1` with code `HKDMC`,
version 1. The response segment is `SammelLastCOR1SEPARes1` with code `HIDMC`,
version 1, and an optional `orderid`. The request segment has the same shape as
`SammelLastSEPA1`: source account, optional total amount, optional
`singletransfers`, PAIN descriptor, and PAIN payload.

`GVMultiLastCOR1SEPA` inherits the COR1 direct-debit constraints from
`GVLastCOR1SEPA` and adds:

- `batchbook -> sepa.batchbook`, default `""`;
- `Total.value -> Total.value`, required;
- `Total.curr -> Total.curr`, required.

Its `createSEPAFromParams()` follows the same pattern as `GVMultiLastSEPA`: it
delegates to the base direct-debit implementation and then sets `Total` through
`SepaUtil.sumBtgValueObject(sepaParams)`.

## Decision
Port `MultiLastCOR1SEPA` as the next original-near PinTAN job slice:

- expose frontend job name `MultiLastCOR1SEPA`;
- map constraints to `SammelLastCOR1SEPA1`;
- use the existing indexed PAIN.008 generation path with debit type `COR1`;
- render `SammelLastCOR1SEPA1` as `HKDMC` with source account, `Total.*`,
  descriptor, and PAIN.008 payload;
- parse `SammelLastCOR1SEPARes1.orderid` into the existing LastSEPA result
  shape;
- persist the request snapshot under `termlast_<orderid>` using
  `SammelLastCOR1SEPA1`;
- keep `singletransfers` unexposed because hbci4java's constructor does not add
  it as a high-level constraint.

Defer `MultiLastB2BSEPA` to a later slice. It should reuse the same code path
with the B2B segment names, codes, and debit type.

## Consequences
The already ported `MultiLastSEPA` bulk direct-debit path becomes shared across
CORE and COR1 variants. This makes the following B2B variant a smaller, more
mechanical port, while still keeping this commit focused on one hbci4java job.

## References
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVMultiLastCOR1SEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVLastCOR1SEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/AbstractGVLastSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/generators/GenLastSEPA00800101.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
