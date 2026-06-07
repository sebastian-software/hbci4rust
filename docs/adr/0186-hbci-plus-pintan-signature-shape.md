# ADR 0186: HBCI Plus PinTAN Signature Shape

## Status

Accepted

## Context

Most runtime replay tests in the Rust port currently use the FinTS 3.0
protocol resource (`hbci-300.xml`). Its PinTAN signature head (`SigHeadUser`
/ `HNSHK`) contains a `SecProfile` group with `method` and `version` fields.

The HBCI Plus protocol resource (`hbci-plus.xml`) defines an older
`SigHeadUser` shape for the same `HNSHK` segment. It has the same `secfunc`,
check reference, range, role, security identification, timestamp, hash
algorithm, signature algorithm, key name, and signature tail `UserSig`, but it
does not contain the `SecProfile` group.

The `KUmsZeitSEPA7` statement segment exists only in the pinned HBCI Plus
resource, so replay-testing it through the handler requires the PinTAN
signature writer to support both original signature head shapes.

## Decision

Make `apply_pintan_sig_head` original-near across the two pinned PinTAN
signature shapes:

- keep writing `SecProfile.method` and `SecProfile.version` when the current
  syntax contains those fields;
- skip only those two fields when the current syntax does not define
  `SecProfile`;
- keep all other signature head and tail fields unchanged;
- keep the same PIN/TAN `UserSig` encoding for both protocol resources.

Do not port non-PinTAN security media, RDH/RAH/DDV signatures, or broader
HBCI 2.x dialog behavior in this slice.

## Consequences

The handler can render signed HBCI Plus custom messages for PinTAN jobs whose
segments only exist in `hbci-plus.xml`, while preserving the existing FinTS 3.0
signature behavior.

The compatibility rule is deliberately narrow: only absent `SecProfile`
fields are skipped. Any other missing signature field remains a protocol error.

## Links

- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0153-pintan-signature-input-generation.md`
- `docs/adr/0185-kums-zeit-sepa-job.md`
