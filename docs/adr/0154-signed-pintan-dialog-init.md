# ADR 0154: Signed PinTAN DialogInit

## Status

Accepted

## Context

hbci4java signs outgoing request messages in `HBCIKernelImpl` by invoking
`Sig.signIt(...)` after the clear-text message has been built. For PinTAN this
adds user `SigHead`/`SigTail` segments, collects the signed range, calls the
passport signer, and writes `UserSig.pin/tan` into `HNSHA`.

The Rust port now has the pieces needed for a first handler-level signing slice:

- `PinTanSignatureContext` for random check reference and timestamp metadata;
- `PinTanSigHead` derivation from `PinTanPassport`;
- deterministic `SigHead`/`SigTail`/`UserSig` shell application;
- optional-aware signature range collection.

`DialogInit` is the smallest runtime entry point because it has no prior SCA
challenge and should sign with the PIN only.

## Decision

Render `DialogInit` through the PinTAN signature shell in the handler:

- build the normal `DialogInit` message as before;
- generate a `PinTanSignatureContext`;
- request or reuse the PinTAN PIN through the existing async callback path;
- encode a PinTAN `UserSig` with no TAN for this initial message;
- apply `SigHead`, `SigTail.seccheckref`, and `UserSig` before final segment
  enumeration and message-size calculation.

Keep `CustomMsg` and `DialogEnd` unsigned in this slice. Also keep one-step TAN
decision logic out of this slice; `DialogInit` only exercises the PIN-only path.

Tests may use a cached PIN to avoid making unrelated dialog replay tests depend
on callback scripting.

## Consequences

The first real handler-rendered request now carries observable PinTAN
`HNSHK`/`HNSHA` segments instead of only testing those helpers in isolation.

Remaining work:

- wire the same signer boundary into `CustomMsg` and `DialogEnd`;
- collect and pass the signature range into one-step TAN-required detection;
- make replay fixtures cover signed full dialog flows.

## Links

- `src/manager/handler.rs`
- `src/manager/signature.rs`
- Upstream: `org.kapott.hbci.manager.HBCIKernelImpl`
- Upstream: `org.kapott.hbci.security.Sig.signIt`
