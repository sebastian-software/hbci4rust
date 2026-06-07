# Security Media Scope Evidence

Snapshot date: 2026-06-07.

This note explains why the scoped v1 port is a non-legacy FinTS PinTAN /
HBCI-Plus port and does not implement hbci4java's classic chipcard, PCSC,
CTAPI, DDV, RDH, RAH, RSA key-file live support, or Java passport import paths.

It is not an argument that those paths never matter. It is an argument that they
are not the current v1 acceptance gap.

## Local Port Evidence

The release-candidate audits show that the remaining upstream job/result gaps
inside the selected Java comparison are explicit v1 boundaries, not hidden
security-media holes:

```text
$ scripts/audit-job-coverage.sh
upstream=68
rust=65
missing=LastCOR1SEPA,MultiLastCOR1SEPA,Template
extra=<none>

$ scripts/audit-result-coverage.sh
upstream_raw=28
upstream_normalized=24
rust=23
missing=WPStammData
extra=<none>
```

`GVTemplate` is hbci4java's dynamic `newLowlevelJob(...)` fallback. v1 keeps a
static PinTAN job registry and does not expose arbitrary caller-selected
lowlevel segments.

`GVLastCOR1SEPA` and `GVMultiLastCOR1SEPA` were removed from the public registry
because `COR1` is no longer relevant for new SDD Core collections.

`WPStammData` is intentionally excluded because the upstream result class is
documented around the lowlevel `WPStammList` path instead of a normal
high-level job.

The security-media exclusions are therefore not hidden missing job classes. They
are separate runtime and storage surfaces that v1 intentionally does not claim.

## Current External Evidence

The following sources were checked for the v1 scope decision.

| Source | Current signal for v1 |
| --- | --- |
| Deutsche Kreditwirtschaft FinTS page | FinTS is described as a multibank standard supported by about 2,000 credit institutions. The page mentions signature-card HBCI and TAN-based procedures such as chipTAN/mobileTAN, so the standard is broader than v1. |
| FinTS official site and specification page | FinTS is the continuation of HBCI. FinTS 3.0 is described as established in the market and supports both SECCOS bank signature cards and two-step TAN variants; FinTS 4.1 adds modern internet/XML support. |
| Deutsche Bank HBCI/FinTS help | FinTS/HBCI-Plus uses PIN/password plus photoTAN or BestSign, with the bank server documented as `https://fints.deutsche-bank.de/`. |
| comdirect HBCI page | comdirect describes software banking through the familiar PIN/TAN procedure and says no additional hardware is required. |
| ING FinTS/HBCI help | ING documents FinTS/HBCI access with Internetbanking PIN or username/password and explicitly says it does not offer HBCI with chipcard. ING is also a reminder that bank-specific FinTS functionality can be narrower than the protocol. |
| Consorsbank HBCI help | Consorsbank documents FinTS/HBCI Plus for financial software and, in the current help snippet, PIN/TAN/SecurePlus-style authorization. |
| DKB electronic-banking pages | DKB documents FinTS with PIN/TAN for business electronic banking and app/chipTAN-style security media for financial-software access. |
| Sparkasse HBCI and TAN pages | Sparkasse still explains classic HBCI chipcard as secure but laborious and says it is not recommended today; its current online-banking TAN page centers pushTAN and chipTAN. |
| REINER SCT key-file note | The provider explains that copyable RDH-10 key files conflict with PSD2-era possession-factor requirements and says the DK decided that copyable key files may no longer be used under PSD2. |
| Deutsche Bundesbank PSD2 page | PSD2 strong customer authentication uses independent factors such as knowledge, possession, and inherence; dynamic TAN procedures are part of the consumer-facing explanation. |

## Interpretation

The current evidence supports a pragmatic v1 boundary:

- PinTAN/HBCI-Plus remains the best-supported path for a useful modern
  consumer-bank and small-business Rust port.
- chipTAN, QR-TAN, photoTAN, decoupled app approval, and TAN media metadata are
  in scope as PinTAN/SCA mechanisms.
- HBCI signature cards are still part of the broader FinTS ecosystem, especially
  in Sparkasse/VR/business contexts, but they require a different runtime stack:
  PCSC/CTAPI bindings, card-reader UX, DDV/signature handling, and different
  passport/storage compatibility.
- Key-file live support is a poor v1 target because it adds RDH/RAH/RSA
  signing, Java passport compatibility questions, and PSD2-era uncertainty.
- The current port should document these surfaces as future scope candidates,
  not unfinished release blockers.
- The project does not currently plan to add those historical security media;
  future support would require a new ADR and a deliberate scope expansion.

## Source Links

- Deutsche Kreditwirtschaft FinTS:
  `https://die-dk.de/zahlungsverkehr/electronic-banking/fints/`
- FinTS start page:
  `https://www.fints.org/`
- FinTS specification page:
  `https://www.fints.org/de/spezifikation`
- Deutsche Bank HBCI/FinTS:
  `https://www.deutsche-bank.de/pk/konto-und-karte/services/hbci-fints.html`
- comdirect HBCI:
  `https://www.comdirect.de/cms/kontakt-zugaenge-hbci.html`
- ING FinTS/HBCI:
  `https://www.ing.de/hilfe/log-in/fints/`
- Consorsbank HBCI:
  `https://www.consorsbank.de/web/Wissen/FAQ/kontokarten/HBCI-Schnittstelle-Konto-und-Zahlungsverkehrsprogramm`
- DKB electronic banking:
  `https://www.dkb.de/geschaeftskunden/electronic-banking/antraege-bedingungen`
- DKB financial-software FAQ:
  `https://www.dkb.de/fragen-antworten/kann-ich-eine-finanzsoftware-fuers-banking-benutzen`
- Sparkasse HBCI:
  `https://www.sparkasse.de/pk/ratgeber/finanzglossar/hbci-verfahren.html`
- Sparkasse TAN procedures:
  `https://www.sparkasse.de/fk/produkte/konto-und-zahlungsverkehr/electronic-banking/tan-verfahren.html`
- REINER SCT key file:
  `https://www.reiner-sct.com/wiki/schluesseldatei/`
- Deutsche Bundesbank PSD2:
  `https://www.bundesbank.de/en/tasks/payment-systems/psd2/psd2-775954`
