# ADR 0148: PinTAN SigHead Fill Helper

## Status

Accepted

## Context

hbci4java's `Sig.fillSigHead(...)` fills the user signature head (`HNSHK`,
`SigHeadUser`) before the message range is hashed/signed. For PinTAN, the
passport is not backed by cryptographic keys, but the same segment still carries
observable security metadata:

- security function from `getCurrentTANMethod(false)`;
- `PIN` security profile, version `1` for one-step TAN method `999` and `2` for
  two-step methods;
- request security identification function `1`;
- system id default `0`;
- signature id default `1`;
- dummy PinTAN algorithms `HashAlg.alg = 999`, `SigAlg.alg = 10`, and
  `SigAlg.mode = 16`;
- key name from the PinTAN passport, with key number and key version `0`.

The Rust port can now encode PinTAN `UserSig` bytes and apply them to
`SigTail.UserSig`, but tests still hand-fill `SigHead` fields. Keeping those
fields as ad hoc test data risks drifting away from hbci4java's PinTAN defaults.

## Decision

Add a narrow deterministic manager helper for PinTAN request signature heads:

- introduce `PinTanSigHead` as the explicit value object for the fields that
  `Sig.fillSigHead(...)` writes;
- derive PinTAN defaults from `PinTanPassport` with `PinTanSigHead::from_passport(...)`;
- require caller-provided `seccheckref`, `secref`, date, and time so tests and
  future replay fixtures stay deterministic;
- expose `apply_pintan_sig_head(...)` to write the value object into any
  `HbciMessage` `SigHead` path such as `DialogEnd.SigHead`;
- keep the helper in `manager`, not `protocol`, because these values are
  security-method/passport concepts rather than generic XML syntax.

Do not generate random check references, read the current clock, persist or
increment a signature id, collect hash data, fill `SigTail`, or wire automatic
handler signing in this slice.

## Consequences

The Rust port gets a concrete original-near bridge for `HNSHK` PinTAN request
metadata and can stop treating signature-head fields as arbitrary test literals.
The larger signer can later combine this helper with deterministic check-ref and
timestamp generation, `UserSig` signing, and `SigTail` rendering.

Remaining work:

- generate hbci4java-like random `seccheckref` values at the signer boundary;
- add a persisted PinTAN signature counter if needed for live compatibility;
- fill matching `SigTail.seccheckref` from the head;
- collect the signed message range and integrate full signing into
  `DialogInit`, `DialogEnd`, and `CustomMsg`.

## Links

- `src/manager/signature.rs`
- `src/passport/pintan.rs`
- `src/protocol/message.rs`
- Upstream: `org.kapott.hbci.security.Sig.fillSigHead`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport`
