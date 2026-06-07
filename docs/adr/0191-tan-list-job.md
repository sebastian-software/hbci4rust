# ADR 0191: TAN List Job

## Status

Accepted

## Context

`GVTANList` is hbci4java's high-level job for querying TAN list metadata. The
Java job name is `TANList`, while its lowlevel segment name is `TANListList`.

The pinned FinTS 3.0 resource defines `TANListList1` / `HKTAZ` and
`TANListListRes1` / `HITAZ`. The request segment has no high-level frontend
constraints. The response contains one TAN list per `HITAZ` segment with:

- `liststatus`;
- `listnumber`;
- optional creation `date`;
- optional TAN counters;
- repeated `TANInfo` groups with `usagecode`, optional text, TAN value, and
  optional usage timestamp fields.

This job is historically tied to TAN-list based procedures, but hbci4java keeps
it as part of the high-level GV surface. The v1 Rust port should preserve that
surface while still keeping chipcard and key-file media out of scope.

## Decision

Port `TANList` as a compact original-near PinTAN job slice:

- expose no frontend constraints, matching `GVTANList`;
- render `TANListList1` as `HKTAZ` with only the request marker;
- map process-1 TAN orderhash metadata to `TANListList1` / `HKTAZ`;
- add `GvrTanList`, `GvrTanListEntry`, and `GvrTanInfo` result structs;
- collect repeated `TANListListRes1` response segments and nested `TANInfo`
  groups;
- expose raw `TANListListRes1` content through `result_data`.

Do not port `GVRTANList#toString()`, usage-code display translations, or
chipcard/key-file TAN media behavior in this slice.

## Consequences

The Rust port gains another hbci4java high-level job without adding any new
request parameter semantics.

The structured result keeps raw code and timestamp fields as strings, matching
the current original-near Rust strategy for date/time values and avoiding early
domain remodeling.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVTANList.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRTANList.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
