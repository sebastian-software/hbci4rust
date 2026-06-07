# ADR 0190: Status Protocol Job

## Status

Accepted

## Context

`GVStatus` is hbci4java's high-level job for querying the status protocol of
previously submitted orders. The Java job name and lowlevel segment name are
both `Status`.

The pinned FinTS 3.0 resource defines `Status4` / `HKPRO` and `StatusRes4` /
`HIPRO`. The request segment contains optional `startdate`, `enddate`,
`maxentries`, and `offset` fields. The Java high-level job exposes only
`startdate`, `enddate`, `maxentries`, and a frontend-only `jobid` helper; it
does not register `offset` as a public constraint.

`GVStatus#setParam("jobid", value)` parses the date prefix before the first `/`
from a Java job id shaped like `yyyyMMdd/dialogid/msgnum/segref` and writes that
date to both `startdate` and `enddate`. It does not put `jobid` on the wire.

`GVRStatus` stores repeated entries containing:

- the referenced dialog id;
- the referenced message number;
- the response segment reference;
- the submission date and time;
- the returned status value.

## Decision

Port `Status` as a small original-near PinTAN job slice:

- expose the Java frontend constraints `startdate`, `enddate`, `maxentries`,
  and pseudo-constraint `jobid`;
- render only `Status4` fields represented by those frontend constraints;
- keep `offset` unexposed in this slice despite its XML presence;
- implement the `jobid` convenience setter by extracting the leading ISO date
  and setting both `startdate` and `enddate`;
- map process-1 TAN orderhash metadata to `Status4` / `HKPRO`;
- add `GvrStatus` / `GvrStatusEntry` and collect repeated `StatusRes4` entries;
- expose raw `StatusRes4` content through `result_data`.

Do not port `GVRStatus#toString()`, `getJobEntry(...)`, or Java's
`InvalidUserDataException` ignore-error behavior in this slice.

## Consequences

The Rust port gains the original status protocol query surface without widening
the public API beyond hbci4java's high-level job constraints.

Because `jobid` is not a lowlevel value, callers can use the Java-style helper
while the generated FinTS message remains identical to setting `startdate` and
`enddate` directly.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVStatus.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRStatus.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
