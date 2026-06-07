# ADR 0155: Signed PinTAN DialogEnd

## Status

Accepted

## Context

ADR 0154 wired PinTAN signing into `DialogInit`, the first handler-rendered
request. `DialogEnd` still renders without user `SigHead`/`SigTail`, even though
hbci4java sends request messages through the same `Sig.signIt(...)` path before
transmission.

For PinTAN, the Java passport signer is not limited to initialization messages.
It signs the prepared message range and returns `UserSig` data, usually PIN-only
unless a stored SCA challenge requires TAN input.

## Decision

Render `DialogEnd` through the PinTAN signature shell in the handler:

- build the normal `DialogEnd` message as before;
- generate a `PinTanSignatureContext`;
- derive and apply `SigHead`;
- use the existing SCA-aware PinTAN `UserSig` helper, so ordinary dialog end
  sends PIN-only while a stored challenge may still request TAN through the
  callback path;
- apply `SigTail.seccheckref` and `UserSig` before final segment enumeration and
  message-size calculation.

Keep `CustomMsg` unsigned in this slice. Also keep one-step segment-code
TAN-required detection out of scope.

## Consequences

The runtime handler now signs the two fixed dialog lifecycle requests,
`DialogInit` and `DialogEnd`, using the same original-near PinTAN signature
shell.

Remaining work:

- wire signing into `CustomMsg`;
- feed the collected signed range into one-step TAN-required detection;
- add full signed replay fixtures across init, execute, and close.

## Links

- `docs/adr/0154-signed-pintan-dialog-init.md`
- `src/manager/handler.rs`
- `src/manager/signature.rs`
- Upstream: `org.kapott.hbci.manager.HBCIKernelImpl`
- Upstream: `org.kapott.hbci.security.Sig.signIt`
