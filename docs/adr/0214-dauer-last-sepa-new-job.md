# 0214 Port DauerLastSEPANew Before DauerLastSEPAList

## Status

Accepted

## Context

`GVDauerLastSEPANew` in hbci4java creates a new recurring SEPA direct debit
order. It extends `AbstractGVLastSEPA`, uses `GVRDauerLastNew`, and adds the
standing-order recurrence fields `firstdate`, `timeunit`, `turnus`, `execday`,
and `lastdate`.

The pinned HBCI 300 protocol resource defines request segment
`DauerLastSEPANew1` with code `HKDDE` version 1 and response segment
`DauerLastSEPANewRes1` with code `HIDDE` version 1 and optional `orderid`.

Unlike the already ported single dated direct debit jobs, upstream does not add a
`batchbook` constraint for `DauerLastSEPANew`. It inherits the Last-SEPA direct
debit base constraints, specializes `type` to `CORE`, and then appends the five
`DauerDetails` fields.

`GVDauerLastSEPAList` is adjacent, but its result extraction parses PAIN data from
the response into a `GVRDauerLastList` structure and persists bank-returned
snapshots. That is a larger parsing/result slice than creating a new recurring
order.

## Decision

Port `DauerLastSEPANew` first as a focused original-near slice:

- expose frontend job name `DauerLastSEPANew`;
- map constraints to `DauerLastSEPANew1`;
- reuse the Last-SEPA direct debit constraints without `batchbook`;
- set `type` default to `CORE`;
- add required `DauerDetails.firstdate`, `timeunit`, `turnus`, and `execday`;
- add optional `DauerDetails.lastdate` with empty default;
- generate PAIN.008.001.01 like `AbstractGVLastSEPA`;
- render request segment `HKDDE`;
- parse response segment `HIDDE` through the existing order-id-only result shape;
- persist the submitted request snapshot under `dauer_<orderid>`.

Defer `DauerLastSEPAList` to a later ADR and implementation slice.

## Consequences

The shared Last-SEPA constraint helper needs to support variants with and without
`batchbook`. The handler can reuse most of the SEPA standing-order rendering path,
but with the PAIN.008 descriptor and Last-SEPA direct debit result behavior.

Keeping the list job separate avoids smuggling an incomplete `GVRDauerLastList`
parser into the new-order slice.
