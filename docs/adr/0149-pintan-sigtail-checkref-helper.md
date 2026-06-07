# ADR 0149: PinTAN SigTail Check Reference Helper

## Status

Accepted

## Context

hbci4java's `Sig.fillSigTail(...)` fills the user signature tail (`HNSHA`,
`SigTailUser`) after `Sig.fillSigHead(...)`. For both cryptographic security
media and PinTAN, this step copies the signature check reference from the head
segment to the tail segment:

- read `<SigHead>.seccheckref`;
- write the same value to `<SigTail>.seccheckref`.

The Rust port now has a deterministic PinTAN `SigHead` helper and a separate
helper that decodes PinTAN `UserSig` bytes into `SigTail.UserSig.pin/tan`, but
it still relies on tests or callers to set `SigTail.seccheckref` manually.

## Decision

Add a narrow manager helper:

- expose `apply_pintan_sig_tail_from_head(...)`;
- accept a mutable `HbciMessage`, a signature-head path, and a signature-tail
  path;
- require `<sigHead>.seccheckref` to be present and non-empty;
- set `<sigTail>.seccheckref` to the same value.

Keep this helper separate from `apply_pintan_user_sig_to_sig_tail(...)`, because
hbci4java fills the tail reference before calculating and applying the actual
signature payload.

Do not collect hash data, apply `UserSig`, render full messages, support
multiple passports, or wire automatic handler signing in this slice.

## Consequences

The Rust port can now build the observable `HNSHK`/`HNSHA` reference pair in the
same two-step order as hbci4java: fill the head, copy its reference into the
tail, then apply the PinTAN user signature bytes.

Remaining work:

- combine `SigHead`, tail reference, and `UserSig` helpers into a deterministic
  signer boundary;
- generate hbci4java-like check references and timestamps;
- integrate full signing into `DialogInit`, `DialogEnd`, and `CustomMsg`.

## Links

- `src/manager/signature.rs`
- `src/protocol/message.rs`
- Upstream: `org.kapott.hbci.security.Sig.fillSigTail`
