# 0217 Port MultiLastSEPA With Indexed PAIN.008 Direct Debits

## Status
Accepted

## Context
`GVMultiLastSEPA` in hbci4java implements SEPA bulk core direct debit. It
extends `GVLastSEPA`, keeps the PAIN job name `LastSEPA`, and overrides only
the lowlevel job name plus aggregate constraints. The hbci4java lowlevel name is
`SammelLastSEPA`; in FinTS 3.0 this is segment `SammelLastSEPA1` with code
`HKDME`, version 1. Its response segment is `SammelLastSEPARes1` with code
`HIDME`, version 1, and an optional `orderid`.

The request segment has the same shape as the already ported `MultiUebSEPA`
bulk transfer segment: source account, optional total amount, optional
`singletransfers`, PAIN descriptor, and PAIN payload. hbci4java does not expose
`singletransfers` as a constructor constraint for `GVMultiLastSEPA`.

`GVMultiLastSEPA` adds:

- `batchbook -> sepa.batchbook`, default `""`;
- `Total.value -> Total.value`, required;
- `Total.curr -> Total.curr`, required.

Its `createSEPAFromParams()` delegates to `GVLastSEPA` / `AbstractGVLastSEPA`
and then sets `Total` using `SepaUtil.sumBtgValueObject(sepaParams)`. The
PAIN.008 generator uses indexed direct debit parameters, emits one
`DrctDbtTxInf` per index from `0..=maxIndex`, writes `NbOfTxs` as the count,
and rejects mixed currencies while calculating the control sum.

## Decision
Port `MultiLastSEPA` as the next original-near PinTAN job slice:

- expose frontend job name `MultiLastSEPA`;
- map constraints to `SammelLastSEPA1`;
- reuse the existing LastSEPA result container and persistent snapshot shape;
- render `SammelLastSEPA1` as `HKDME` with source account, total amount,
  descriptor, and PAIN.008 payload;
- support raw `_sepapain` and generated PAIN.008 from indexed direct debit
  params;
- generate one `DrctDbtTxInf` per indexed direct debit and set `Total.*` from
  the same sum used for the PAIN control sum;
- reject mixed currencies with the same observable error text as hbci4java;
- keep `singletransfers` unexposed for now because hbci4java's constructor does
  not add it as a high-level constraint.

Defer `MultiLastCOR1SEPA` and `MultiLastB2BSEPA` to later slices. They should
reuse this renderer and generator path with only lowlevel segment names,
segment codes, response roots, and debit type defaults changed.

## Consequences
The Rust port gains the core SEPA bulk direct debit job without broadening the
scope to all direct debit variants at once. The implementation will introduce a
multi-transaction PAIN.008 generator path, which should be written generically
enough for the COR1 and B2B follow-up jobs.

The existing `sum_pain_001_transfer_values` helper name is too narrow for this
shared direct debit path. Keep it as a compatibility wrapper if renamed, but use
a neutral internal/public helper for indexed SEPA transaction totals going
forward.

## References
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVMultiLastSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVLastSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/AbstractGVLastSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/generators/GenLastSEPA00800101.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/SepaUtil.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
