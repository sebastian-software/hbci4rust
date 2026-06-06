# ADR 0096: Indexed Constraint Verification Fallback

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl#verifyConstraints()` first checks a constraint's
plain low-level destination with `getLowlevelParam(destination)`. If that value
is absent and the frontend parameter is indexed, Java checks
`getLowlevelParam(insertIndex(destination, 0))` before falling back to defaults.

ADR 0087 ported `insertIndex(...)` for indexed setters, but
`HbciJob::verify_constraints()` still ignored the Java index-`0` fallback. This
left jobs populated through `try_set_indexed_param(...)` unable to satisfy
required indexed constraints during queue admission.

## Decision

Extend `HbciJob::verify_constraints()` so each constraint resolves in this
order:

1. non-empty plain low-level destination value;
2. non-empty indexed low-level destination value for index `0`, only when the
   constraint is indexed;
3. non-empty frontend value, retaining the temporary compatibility bridge from
   ADR 0084;
4. configured default value;
5. missing-required-parameter error.

Do not copy an existing indexed index-`0` value into the plain low-level
destination. In hbci4java the indexed fallback is assigned to `givenContent`,
so the later default-persistence branch does not call `setLowlevelParam(...)`.

Keep ADR 0085's persistence behavior for frontend and default fallbacks: when no
plain or indexed low-level value was found, a non-empty resolved value is stored
at the plain low-level destination.

The returned resolved map remains keyed by the plain constraint destination,
because it represents constraint resolution, not the exact source key found in
the mutable low-level store.

## Consequences

Indexed low-level values written at index `0` can now satisfy verification,
matching hbci4java's queue-admission lifecycle.

The mutable low-level store remains original-near:

- existing plain low-level values win over indexed index-`0` values;
- existing indexed index-`0` values are not duplicated under the plain key;
- defaults for indexed constraints are still persisted under the plain key.

Remaining work:

- port indexed date helpers;
- port segment validation after low-level propagation;
- derive indexed constraints from concrete repeated PinTAN jobs instead of
  synthetic tracer fixtures.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0084-lowlevel-aware-constraint-verification.md`
- `docs/adr/0085-constraint-verification-default-persistence.md`
- `docs/adr/0087-indexed-job-param-setter-tracer.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#verifyConstraints`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#insertIndex`
