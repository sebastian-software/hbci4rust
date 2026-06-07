# ADR 0275: Remove Classic Domestic Transfer Internals

## Status

Accepted

## Context

ADR 0268 removed `Donation`, `Ueb`, `UebBZU`, `UebEil`, `UebGar`, and `Umb`
from the public PinTAN job registry. ADR 0270 then made the static registry a
hard execution boundary, so manually constructed `HbciJob` values for these
names cannot be queued or rendered through the public handler path.

The source still carried internal implementation branches for the removed jobs:

- account CRC routing;
- frontend-to-lowlevel constraints;
- the `UebBZU` check-digit validator;
- CustomMsg render branches for `Ueb5`, `UebEil1`, `UebGar1`, and `Umb2`;
- orderhash metadata for `HKUEB`, `HKEIL`, `HKGUB`, and `HKUMB`;
- the `UebGar` response-root helper and raw result-data branch.

These branches are classic national domestic transfer/account-transfer code.
They are not needed for modern SEPA transfer jobs, `UmbSEPA`, or the still
product-sensitive `UebForeign` / `HKAOM` path.

## Decision

Remove the internal implementation branches for `Donation`, `Ueb`, `UebBZU`,
`UebEil`, `UebGar`, and `Umb`.

Keep these adjacent paths intact:

- `UebSEPA`, `MultiUebSEPA`, `TermUebSEPA`, `TermMultiUebSEPA`, `InstUebSEPA`,
  and `UmbSEPA`;
- `UebForeign`, because foreign and foreign-currency payments remain a current
  product need even though the existing `HKAOM` job shape is legacy-carried;
- shared national-account helpers still used by scheduled/standing internals,
  securities/account queries, and `UebForeign`.

Do not change the public registry counts or audit expectations in this slice.
The missing upstream job list already includes the removed classic domestic
jobs.

## Consequences

The implementation now better matches the non-legacy public boundary: the
removed classic domestic jobs are not only unavailable through `new_job(...)`,
they also no longer have dead render/constraint/orderhash code in `src`.

`HbciJob::new("Ueb")` and similar manual construction remains possible as a
plain value, but checked queueing and execution reject it before rendering.

Future work can remove classic scheduled/standing internals separately, because
those still share helpers such as `CLASSIC_USAGE_LINE_COUNT`,
`classic_usage_name(...)`, and national-account rendering code.

## Links

- `src/gv/mod.rs`
- `src/manager/handler.rs`
- `docs/adr/0268-remove-classic-domestic-transfer-public-jobs.md`
- `docs/adr/0270-enforce-public-job-registry-boundary.md`
- `docs/architecture/legacy-cleanup-plan.md`
