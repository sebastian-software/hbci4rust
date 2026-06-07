# ADR 0261: Security Media Scope Evidence

## Status

Accepted

## Context

ADR 0003 scoped v1 to FinTS PinTAN / HBCI-Plus and excluded chipcard, PCSC,
CTAPI, DDV, RDH, RAH, RSA key-file live support, and Java passport import.

After the release-candidate checklist closed, the scope argument needed a
clearer evidence trail. The project had extensive ADRs, but the README and
public docs did not yet explain whether the remaining gaps were true
implementation gaps, legacy security-media gaps, or unsupported lowlevel API
surfaces.

Local audits show that the in-scope high-level Java job and typed-result gaps
are narrow:

- `scripts/audit-job-coverage.sh` reports only `Template` missing from the
  upstream `GV*.java` comparison.
- `scripts/audit-result-coverage.sh` reports only `WPStammData` missing from
  the normalized upstream `GVR*.java` comparison.

Both gaps are lowlevel boundaries, not chipcard or key-file runtime paths.

External market evidence is mixed but supports keeping v1 focused on PinTAN:
FinTS still defines both signature-card/security-media paths and TAN-based
paths, while current bank help pages for common consumer and small-business
usage emphasize FinTS/HBCI-Plus, PIN/TAN, photoTAN, BestSign, SecurePlus,
pushTAN, chipTAN, or app-based SCA. Key-file support is additionally weakened
by PSD2-era requirements around copyable possession factors.

## Decision

Keep the v1 scope unchanged and document the scope evidence publicly.

The v1 completion claim means:

- PinTAN/HBCI-Plus functionality is release-candidate complete against the
  operational checklist.
- The only missing in-scope upstream job/result audit entries are the accepted
  lowlevel boundaries `GVTemplate` and `WPStammData`.
- Non-PinTAN security media remain outside v1 by design.
- chipTAN challenge handling is not the same as HBCI signature-card support:
  TAN-generator/chipTAN SCA metadata is in scope; PCSC/CTAPI/DDV/signature-card
  access is not.

Add a reference note with the local audit evidence and the current external
bank/provider source snapshot. Expand the README so new readers can understand
the useful v1 surface without reading hundreds of ADRs first.

## Consequences

The project can state the v1 boundary more confidently without pretending that
classic HBCI signature cards have disappeared everywhere.

Future work on chipcard, DDV, RDH/RAH/RSA, Java passport import, or dynamic
lowlevel jobs still requires a new ADR, updated audits, public docs, and focused
tests.

If bank evidence changes materially, update the reference note and record a new
ADR before widening v1 acceptance.

## Links

- `docs/reference/security-media-scope.md`
- `docs/reference/unsupported-surfaces.md`
- `docs/architecture/job-coverage.md`
- `docs/architecture/result-coverage.md`
- `scripts/audit-job-coverage.sh`
- `scripts/audit-result-coverage.sh`
- ADR 0003: V1 PinTAN Scope
- ADR 0232: CustomMsg Job And Template Boundary
- ADR 0233: GV Job Coverage Audit
- ADR 0234: GV Result Coverage Audit
- ADR 0252: Unsupported V1 Surface Reference
