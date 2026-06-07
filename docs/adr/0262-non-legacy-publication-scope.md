# ADR 0262: Non-Legacy Publication Scope

## Status

Accepted

## Context

ADR 0261 documented why v1 excludes chipcard, PCSC, CTAPI, DDV, RDH, RAH,
RSA key-file live support, and Java passport import.

The phrase "PinTAN first" is misleading for this project. It sounds as if the
classic security media and historical payment rails are merely deferred roadmap
items. The intended publication stance is narrower: this is an original-near
hbci4java port without strategic support for legacy banking paths.

After auditing the current Rust registry, the project contains three different
kinds of surfaces:

- modern v1 surfaces that should be documented and recommended;
- legacy-adjacent surfaces that were ported for original-near hbci4java
  compatibility and tests;
- intentionally unsupported legacy surfaces such as chipcard/key-file security
  media and Java passport import.

The legacy-adjacent jobs are not hidden implementation gaps, but they should not
be presented as equally strategic to CAMT, SEPA, PinTAN/SCA, instant transfer,
or verification-of-payee workflows.

## Decision

Use "non-legacy port" language for public positioning.

The v1 documentation should:

- avoid "PinTAN first" wording;
- state that there is no current plan to add chipcard/key-file/Java-passport
  support;
- prefer modern FinTS paths in examples and README text;
- clearly label ported historical payment jobs as compatibility-carried
  surfaces, not recommended new integration paths;
- keep those compatibility-carried jobs visible until a separate ADR decides
  whether to remove, hide, or feature-gate them.

Add a reference audit for lower-relevance surfaces currently present in the
crate. The audit is documentation and product-scope guidance, not a code removal
decision.

## Consequences

The README can describe the crate as a useful modern FinTS PinTAN/HBCI-Plus port
without implying future support for historical security media.

Classic domestic transfer/direct-debit jobs, DTAUS bulk jobs, and COR1 SEPA
direct-debit variants remain technically present for now, but they are
documented as compatibility debt before public API stabilization.

If the project removes or feature-gates those jobs later, that is a separate
breaking-surface decision requiring an ADR, updated audit expectations, docs,
and tests.

## Links

- `docs/reference/modern-scope-audit.md`
- `docs/reference/security-media-scope.md`
- `docs/reference/unsupported-surfaces.md`
- `src/gv/mod.rs`
- ADR 0003: V1 PinTAN Scope
- ADR 0233: GV Job Coverage Audit
- ADR 0252: Unsupported V1 Surface Reference
- ADR 0261: Security Media Scope Evidence
