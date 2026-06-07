# ADR 0276: Remove Classic Scheduled And Standing Internals

## Status

Accepted

## Context

ADR 0269 removed `DauerNew`, `DauerEdit`, `DauerDel`, `DauerList`,
`TermUeb`, `TermUebEdit`, `TermUebDel`, and `TermUebList` from the public
PinTAN job registry. ADR 0270 then made the static registry a hard execution
boundary, so manually constructed `HbciJob` values for these names cannot be
queued or rendered through the public handler path.

The source still carried internal implementation branches for the removed jobs:

- account CRC routing for classic scheduled transfers and standing orders;
- frontend-to-lowlevel constraints for `Dauer*` and `TermUeb*` classic jobs;
- CustomMsg render branches for `DauerNew5`, `DauerEdit5`, `DauerDel4`,
  `DauerList5`, `TermUeb4`, `TermUebEdit4`, `TermUebDel3`, and
  `TermUebList3`;
- orderhash metadata for `HKDAN`, `HKDAE`, `HKDAB`, `HKDAL`, `HKTUB`,
  `HKTUE`, `HKTUL`, and `HKTUA`;
- response-root and result-data routing for the classic response segments;
- passport persistent-data snapshot helpers used only to rehydrate classic
  edit/delete requests.

These branches are classic national standing-order and scheduled-transfer code.
They are not needed for the modern SEPA standing-order and scheduled-transfer
jobs.

## Decision

Remove the internal implementation branches for `DauerNew`, `DauerEdit`,
`DauerDel`, `DauerList`, `TermUeb`, `TermUebEdit`, `TermUebDel`, and
`TermUebList`.

Keep these adjacent paths intact:

- `DauerSEPANew`, `DauerSEPAEdit`, `DauerSEPADel`, `DauerSEPAList`,
  `DauerLastSEPANew`, and `DauerLastSEPAList`;
- `TermUebSEPA`, `TermUebSEPAEdit`, `TermUebSEPADel`, `TermUebSEPAList`, and
  `TermMultiUebSEPA`;
- typed result structs such as `GvrDauerList`, `GvrDauerNew`, `GvrDauerEdit`,
  `GvrTermUeb`, `GvrTermUebEdit`, and `GvrTermUebList`, because the supported
  SEPA jobs still normalize their responses into those original-near result
  shapes.

Do not change the public registry counts or audit expectations in this slice.
The missing upstream job list already includes the removed classic scheduled
and standing jobs.

## Consequences

The implementation now better matches the non-legacy public boundary: the
removed classic scheduled-transfer and standing-order jobs are not only
unavailable through `new_job(...)`, they also no longer have dead
render/constraint/orderhash/snapshot code in `src`.

`HbciJob::new("DauerNew")`, `HbciJob::new("TermUeb")`, and similar manual
construction remains possible as plain values, but checked queueing and
execution reject them before rendering.

SEPA standing-order and scheduled-transfer behavior remains in scope and keeps
the existing original-near typed result names where hbci4java reuses those
result shapes.

## Links

- `src/gv/mod.rs`
- `src/manager/handler.rs`
- `docs/adr/0269-remove-classic-scheduled-standing-public-jobs.md`
- `docs/adr/0270-enforce-public-job-registry-boundary.md`
- `docs/architecture/legacy-cleanup-plan.md`
