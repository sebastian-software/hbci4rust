# ADR 0111: CAMT Version Detection

## Status

Accepted

## Context

`KUmsAllCamt` receives a CAMT descriptor in the FinTS response and CAMT XML
documents as payload. hbci4java's `GVKUmsAllCamt` does not blindly trust the
descriptor. It calls `SepaVersion.choose(format, booked)` so that the XML
document namespace can override an incorrect descriptor.

This is an upstream compatibility boundary before the full CAMT parser port:
the parser needs to know which CAMT.052 schema/version the document actually
uses.

## Decision

Port a small CAMT-focused `SepaVersion` subset:

- known CAMT.052.001.01 through CAMT.052.001.09 URNs;
- parsing a version from an URN;
- finding the greatest CAMT version in a list;
- autodetecting the version from a CAMT XML document namespace;
- choosing the XML-detected version over the descriptor when both are present.

Do not port PAIN versions, generator/parser class-name reflection, or CAMT
transaction parsing in this slice.

## Consequences

`KUmsAllCamt` and future CAMT parser code can resolve the same version mismatch
case hbci4java explicitly handles. The full CAMT parser can now be added behind
this version boundary instead of duplicating descriptor and namespace logic.
