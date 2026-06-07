# ADR 0264: Legacy Job Current Relevance

## Status

Accepted

## Context

ADR 0262 and ADR 0263 classify 21 currently registered jobs as
compatibility-carried legacy surface. The obvious short version is
"non-SEPA payment jobs are legacy", but that phrase is too coarse:

- most classic domestic jobs really are old national credit-transfer or
  direct-debit rail support;
- the two `COR1` jobs are SEPA jobs, but the `COR1` local instrument itself is
  no longer relevant for new SDD Core collections;
- `UebForeign` is not domestic and foreign or foreign-currency payments remain
  a current banking need.

The project needs a sharper decision before removing, hiding, or feature-gating
already ported jobs.

## Decision

Keep all 21 jobs in the compatibility-carried legacy candidate set, but document
their current relevance with three different rationales:

- 18 classic national/DTAUS jobs are low-relevance because German national
  credit-transfer and direct-debit schemes were replaced by SEPA, and the local
  Rust constraints still use account-number/bank-sort-code and DTAUS-style
  fields rather than IBAN/BIC and PAIN payloads.
- 2 `COR1` jobs are low-relevance because EPC guidance says only `CORE` can be
  used for new SDD Core collections from 20 November 2016.
- `UebForeign` is a legacy implementation of a current payment need. Foreign
  and foreign-currency payments still exist, but the ported HKAOM/UebForeign2
  surface is not the modern strategic shape. Current cross-border payment
  evidence points toward ISO 20022, structured or hybrid postal addresses, and
  bank/corporate channels rather than this old original-near FinTS job.

`UebForeign` must therefore not be described as a classic domestic transfer and
must not be removed in the same slice as domestic pre-SEPA jobs unless a
separate ADR proves the product boundary.

## Consequences

The cleanup plan can still proceed, but the language becomes more precise:

- "non-SEPA" is evidence, not a universal proof;
- domestic account-number/bank-sort-code jobs can be cleaned up with strong SEPA
  migration evidence;
- `COR1` cleanup remains the smallest and cleanest first slice;
- `UebForeign` moves to a later, separately justified cleanup slice.

The detailed per-job evidence lives in
`docs/reference/legacy-job-relevance-audit.md`.

## Links

- `docs/reference/legacy-job-relevance-audit.md`
- `docs/reference/modern-scope-audit.md`
- `docs/architecture/legacy-cleanup-plan.md`
- ADR 0262: Non-Legacy Publication Scope
- ADR 0263: Guard Legacy Cleanup
