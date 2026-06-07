# 0222 Port VoPCheck As Direct Verification Job

## Status
Accepted

## Context
`GVVoP` in hbci4java implements the Verification of Payee check job. It uses
the lowlevel job name `VoPCheck`, the request segment `VoPCheck1` (`HKVPP`),
and the response segment `VoPCheckRes1` (`HIVPP`). Its constructor adds these
constraints:

- `suppreports.descriptor -> suppreports.descriptor`, default `""`;
- `pollingid -> pollingid`, required;
- `maxentries -> maxentries`, required;
- `offset -> offset`, required.

The Java setter prefixes `pollingid` with `B` before storing it in lowlevel
parameters because the segment field is a FinTS `Bin` value. `skipBPDCheck()`
returns true.

The response can contain either a `reportdesc` plus binary `report` carrying a
`pain.002.001.*` payment status report, or a single `result` group with
recipient IBAN, optional corrected name, VoP status code, and optional reason.
The Java result model exposes a `VoPResult` with `vopId`, `pollingId`, bank
text, and `VoPResultItem` entries. Its status enum maps the FinTS codes
`RCVC`, `RVNM`, `RVMC`, `RVNA`, and `PDNG`.

hbci4java also contains runtime automation around `GVVoP`: it polls again when
no VoP ID was returned, asks `HAVE_VOP_RESULT` callbacks when any item is not a
match, and prepends a `VoPAuth` plus the original payment job after successful
verification. That behavior depends on dialog queue mutation and on completing
VoP items from the original SEPA task data.

## Decision
Port `VoP` as a direct original-near job slice first:

- expose the hbci4java frontend job name `VoP`;
- map constraints to `VoPCheck1`;
- prefix `pollingid` as a binary lowlevel value, matching Java's `setParam`;
- render `VoPCheck1` as `HKVPP` with supported report descriptor, polling ID,
  maximum entries, and offset;
- add a Rust `GvrVoP` result shape mirroring hbci4java's `GVRVoP` fields;
- parse `VoPCheckRes1` single-result responses into `HbciJobResultData::VoP`;
- preserve raw response content through the existing `content_data` map.

Do not port the full runtime automation in this slice. Polling retry,
`HAVE_VOP_RESULT` callback dispatch, auto-queuing `VoPAuth` with the original
task, completion from original SEPA task data, and `pain.002` report parsing
remain follow-up work.

## Consequences
This makes the `VoP` job testable and usable in offline replay tests without
claiming the complete hbci4java dialog choreography. The direct job support
unblocks request rendering, order-hash preparation, and typed parsing for
single checks while leaving the higher-risk queue and callback behavior for a
dedicated follow-up slice.

Unknown or blank VoP status codes are represented as `None`, matching
hbci4java's `VoPStatus.byCode` null result. The Rust result stores the binary
fields as the values exposed by the protocol parser; no Java passport import or
Chipcard path is involved.

## References
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVVoP.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRVoP.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
