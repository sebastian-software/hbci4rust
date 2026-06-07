# 0223 Port WPDepotList As Raw Depot Statement Job

## Status
Accepted

## Context
`GVWPDepotList` in hbci4java implements the securities depot statement
retrieval job. It uses lowlevel job name `WPDepotList` and result class
`GVRWPDepotList`. Its constructor adds these constraints:

- `my.number -> Depot.number`, required;
- `my.subnumber -> Depot.subnumber`, default `""`;
- `my.country -> Depot.KIK.country`, defaulted from UPD account country;
- `my.blz -> Depot.KIK.blz`, defaulted from UPD account bank code;
- `quality -> quality`, default `""`;
- `maxentries -> maxentries`, default `""`.

`GVWPDepotList#redoAllowed()` returns true and `verifyConstraints()` delegates
to the base checks and then calls `checkAccountCRC("my")`.

In FinTS 3.0 the request segment is `WPDepotList6`, code `HKWPD`, version 6.
The response segment is `WPDepotListRes6`, code `HIWPD`, version 6, and carries
the statement payload as binary `data535`. Older protocol versions can return
`data571`, but the Rust v1 port currently targets the FinTS 3.0 path for
PinTAN/HBCI-Plus work.

The Java result parser buffers the `data535` payload, decodes SWIFT umlaut
placeholders, splits MT535 blocks, and parses a large structured depot model:
statement timestamp, depot account, total value, security positions, prices,
balances, sub-balances, and many optional descriptive fields.

## Decision
Port `WPDepotList` as the next registered PinTAN job slice with the FinTS 3.0
wire shape:

- keep frontend job name `WPDepotList`;
- map hbci4java constraints to `WPDepotList6`;
- use the passport account as fallback for the depot KTV fields, matching the
  existing account-fallback pattern used by other jobs;
- render `WPDepotList6` as `HKWPD` with depot account, optional quality,
  optional max entries, and optional offset;
- add an `orderhash_source_job_info` mapping so the job can participate in
  PinTAN order-hash preparation;
- expose a Rust `GvrWPDepotList` result that stores decoded raw `data535` and
  parser remainder, and map it through `HbciJobResultData::WPDepotList`;
- preserve the usual `content.*` raw response data.

Do not port the full MT535/MT571 depot parser in this slice. That parser is a
separate piece of substantial domain behavior and should be ported with
dedicated upstream fixtures and golden outputs.

## Consequences
`WPDepotList` becomes executable in offline replay tests and no longer remains a
registered-but-unrenderable job. The public result surface is intentionally
smaller than hbci4java's final `GVRWPDepotList`: callers can access the raw
decoded depot payload now, while structured security position parity remains
tracked as follow-up work.

The Java constructor has no public `curr` constraint even though `WPDepotList6`
contains an optional `curr` field, so this slice does not expose a new
Rust-only `curr` parameter.

## References
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVWPDepotList.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRWPDepotList.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
