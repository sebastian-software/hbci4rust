# ADR 0277: Supersede Removed Legacy Job Port ADRs

## Status

Accepted

## Context

The original-near port initially accepted individual job-port ADRs for several
hbci4java jobs that were later classified as legacy and removed from the v1
public surface:

- COR1 direct debit variants;
- DTAUS bulk jobs;
- classic national direct debit and objection jobs;
- classic national domestic transfer and account-transfer jobs;
- classic national scheduled-transfer and standing-order jobs.

ADRs 0265 through 0269 removed those jobs from the public registry. ADRs 0271
through 0273, 0275, and 0276 then removed the corresponding internal
implementation branches where they still existed.

The earlier per-job ADRs still said `Accepted`. That was historically true for
the old porting slices, but misleading as current architecture documentation:
readers could reasonably infer that the removed jobs are still supported or
should be restored.

## Decision

Mark the obsolete per-job ADRs as superseded by the later removal ADRs:

- `LastCOR1SEPA` and `MultiLastCOR1SEPA`: superseded by ADR 0265 and ADR 0271.
- `MultiUeb` and `MultiLast`: superseded by ADR 0266 and ADR 0272.
- `Last` and `StornoLast`: superseded by ADR 0267 and ADR 0273.
- `Donation`, `Ueb`, `UebBZU`, `UebEil`, `UebGar`, and `Umb`: superseded by
  ADR 0268 and ADR 0275.
- `DauerNew`, `DauerEdit`, `DauerDel`, `DauerList`, `TermUeb`, `TermUebEdit`,
  `TermUebDel`, and `TermUebList`: superseded by ADR 0269 and ADR 0276.

Keep the old ADR bodies intact as historical porting notes. Do not delete them,
because they still explain why the now-removed implementations looked the way
they did and can help future archaeology.

## Consequences

The ADR set now distinguishes historical porting decisions from the current v1
non-legacy product boundary.

Future work should treat the superseding removal ADRs as authoritative. Bringing
any superseded job back into the public API requires a new scope ADR, updated
audits, updated docs, and focused tests.

`UebForeign` is not changed by this ADR. It remains the single
compatibility-carried legacy job because foreign and foreign-currency payments
are a current user need, even though the existing HKAOM/UebForeign2 shape is
old and needs a separate product-boundary decision.

## Links

- `docs/architecture/legacy-cleanup-plan.md`
- `docs/reference/legacy-job-relevance-audit.md`
- `docs/reference/unsupported-surfaces.md`
- `scripts/audit-modern-scope.sh`
