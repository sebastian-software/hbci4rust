# 0224 Port WPDepotUms As Raw Depot Transactions Job

## Status
Accepted

## Context
`GVWPDepotUms` in hbci4java implements the securities depot transaction
retrieval job. It uses lowlevel job name `WPDepotUms` and result class
`GVRWPDepotUms`. Its constructor adds these constraints:

- `my.number -> Depot.number`, required;
- `my.subnumber -> Depot.subnumber`, default `""`;
- `my.country -> Depot.KIK.country`, defaulted from UPD account country;
- `my.blz -> Depot.KIK.blz`, defaulted from UPD account bank code;
- `quality -> quality`, default `""`;
- `maxentries -> maxentries`, default `""`;
- `startdate -> startdate`, default `""`;
- `enddate -> enddate`, default `""`;
- `dummy -> alldepots`, default `"N"`.

`GVWPDepotUms#redoAllowed()` returns true and `verifyConstraints()` delegates
to the base checks and then calls `checkAccountCRC("my")`.

In FinTS 3.0 the request segment is `WPDepotUms5`, code `HKWDU`, version 5.
The response segment is `WPDepotUmsRes5`, code `HIWDU`, version 5, and carries
the transaction payload as binary `data536`. Older protocol versions can return
`data572`, but the Rust v1 port currently targets the FinTS 3.0 path for
PinTAN/HBCI-Plus work.

The Java result parser buffers `data536`, decodes SWIFT umlaut placeholders,
splits MT536 blocks, and parses a large structured depot transaction model:
statement timestamp, depot account, financial instruments, start and end
balances, prices, transaction details, settlement dates, parties, and free-text
details.

The Java constructor also exposes `quality`, although `WPDepotUms5` does not
contain a `quality` data element. We preserve the accepted frontend parameter
for original-near API behavior, but do not render it into a nonexistent FinTS
3.0 field.

## Decision
Port `WPDepotUms` as the next registered PinTAN job slice with the FinTS 3.0
wire shape:

- keep frontend job name `WPDepotUms`;
- map hbci4java constraints to `WPDepotUms5`;
- use the passport account as fallback for the depot KTV fields, matching the
  existing account-fallback pattern used by depot statement jobs;
- render `WPDepotUms5` as `HKWDU` with depot account, `alldepots`, optional
  start date, optional end date, and optional max entries;
- add an `orderhash_source_job_info` mapping so the job can participate in
  PinTAN order-hash preparation;
- expose a Rust `GvrWPDepotUms` result that stores decoded raw `data536`,
  parser remainder, and an empty structured `entries` list for now;
- preserve the usual `content.*` raw response data.

Do not port the full MT536/MT572 depot transaction parser in this slice. That
parser is a separate piece of substantial domain behavior and should be ported
with dedicated upstream fixtures and golden outputs.

## Consequences
`WPDepotUms` stops being a registered-but-unrenderable job and becomes
executable in offline replay tests. The public result surface is intentionally
smaller than hbci4java's final `GVRWPDepotUms`: callers can access the raw
decoded transaction payload now, while structured security transaction parity
remains tracked as follow-up work.

The Java constructor has no public `offset` constraint even though
`WPDepotUms5` contains an optional `offset` field, so this slice does not expose
a new Rust-only `offset` parameter.

## References
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVWPDepotUms.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRWPDepotUms.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
