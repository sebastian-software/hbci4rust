# ADR 0188: Institute Information List Job

## Status

Accepted

## Context

`GVInfoList` is hbci4java's high-level job for retrieving the list of
available institute information items. The pinned FinTS 3.0 resource defines
`InfoList4` / `HKKIA` and `InfoListRes4` / `HIKIA`; the HBCI Plus resource
contains the same segment family for older protocol versions.

The original Java job exposes one frontend constraint:

- `maxentries` -> `InfoList4.maxentries`, defaulting to an empty value.

The XML segment also contains an optional `offset` field, but `GVInfoList` does
not expose it as a frontend constraint. Its result type `GVRInfoList` stores a
list of `InfoInfo` entries with `code`, `descr`, `type`, optional `version`,
optional `format`, and repeated `comment` values.

## Decision

Port `InfoList` as the next PinTAN job slice:

- expose only the original Java frontend constraint `maxentries`;
- render `InfoList4` as `HKKIA` and set optional `maxentries` when present;
- map process-1 TAN orderhash metadata to `InfoList4` / `HKKIA`;
- add a small Rust-native `GvrInfoList` / `GvrInfoListInfo` result shape that
  mirrors the original `GVRInfoList.Info` fields with Rust-cased names;
- collect raw `InfoListRes4` content into `result_data` alongside the structured
  result.

Do not expose `offset`, port `InfoOrder`, parse document ordering flows, or add
display parity for `GVRInfoList#toString()` in this slice.

## Consequences

The Rust port gains a small but structured non-payment PinTAN job and exercises
repeated DEG response parsing outside the account and TAN media families.

Keeping `offset` out of the public constraints preserves the original Java job
surface. If banks require pagination behavior later, it should be documented as
a separate deviation from `GVInfoList`.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVInfoList.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRInfoList.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0076-job-result-result-data-tracer.md`
