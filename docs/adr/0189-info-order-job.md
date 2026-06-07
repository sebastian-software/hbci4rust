# ADR 0189: Institute Information Order Job

## Status

Accepted

## Context

`GVInfoOrder` is hbci4java's high-level job for ordering institute information
items listed by `GVInfoList`. The Java job name is `InfoOrder`, while its
lowlevel segment name is `InfoDetails`.

The pinned FinTS 3.0 resource defines `InfoDetails4` / `HKINF` and
`InfoDetailsRes4` / `HIINF`. The request segment contains repeated
`InfoCodes.code` values and an optional `Address2` group. The response contains
repeated `Info` groups with `code` and `msg`.

The original Java constraints are:

- required `code` -> `InfoDetails4.InfoCodes.code`;
- address fields `name`, `name2`, `street`, `ort`, `plz`, `country`, `tel`,
  `fax`, and `email`;
- the frontend field `plz` maps to both `Address.plz_ort` and `Address.plz`;
- additional code fields generated as `code_2` through `code_10`.

The XML defines `InfoCodes.code` with `maxnum="9"`, while hbci4java still
creates nine additional frontend code constraints after the required base
`code`. The Rust port should preserve that public surface first and let the
protocol message tree enforce concrete syntax limits when a bank/test uses the
upper edge.

## Decision

Port `InfoOrder` as the next PinTAN job slice:

- expose the original Java frontend constraints, including duplicate `plz`
  mapping and `code_2` through `code_10`;
- render `InfoDetails4` as `HKINF` and set all present code/address fields;
- map process-1 TAN orderhash metadata to `InfoDetails4` / `HKINF`;
- add `GvrInfoOrder` / `GvrInfoOrderInfo` as a small structured result shape
  mirroring `GVRInfoOrder.Info`;
- collect raw `InfoDetailsRes4` content into `result_data`.

Do not port document delivery behavior, postal address validation, `InfoList`
pagination, or display parity for `GVRInfoOrder#toString()` in this slice.

## Consequences

The Rust port can now follow the `InfoList` discovery job with a matching
information ordering job while preserving hbci4java's public parameter names.

Duplicate frontend constraints are intentional here: setting `plz` writes both
original lowlevel destinations, matching the Java job's historical
compatibility behavior across `Address1` and `Address2`.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVInfoOrder.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRInfoOrder.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0188-info-list-job.md`
