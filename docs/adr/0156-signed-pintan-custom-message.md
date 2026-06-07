# ADR 0156: Signed PinTAN CustomMsg

## Status

Accepted

## Context

ADR 0154 and ADR 0155 wired PinTAN signing into the fixed dialog lifecycle
messages, `DialogInit` and `DialogEnd`. The queued business request message,
`CustomMsg`, still renders without user `SigHead`/`SigTail`.

In hbci4java, the kernel sends prepared request messages through the same
signature path before transmission. Leaving `CustomMsg` unsigned keeps offline
rendering simple, but it does not move the v1 PinTAN runtime toward a realistic
bank-facing dialog flow.

## Decision

Render queued `CustomMsg` requests through the PinTAN signature shell:

- keep rendering queued jobs into the original `CustomMsg.GV` positions;
- generate a `PinTanSignatureContext` per outgoing message;
- derive and apply `CustomMsg.SigHead` from the PinTAN passport;
- sign with the SCA-aware `UserSig` helper, so normal job requests are PIN-only
  and a stored SCA challenge may include a TAN through the callback path;
- apply `CustomMsg.SigTail` before final segment enumeration and message-size
  calculation.

The handler render path becomes async and mutable because signing may request
PIN/TAN callback data and cache the runtime PIN in the passport. The execute
path clones the FinTS endpoint before rendering so no immutable passport borrow
is held across signing.

Keep one-step segment-code TAN-required detection out of scope for this slice.

## Consequences

All currently ported handler request types, `DialogInit`, `CustomMsg`, and
`DialogEnd`, now share the same original-near PinTAN signing boundary.

Queued business job segment numbers shift behind `HNSHK`; for example a single
`SaldoReq` moves from segment 2 to segment 3, and `HNSHA`/`HNHBS` follow after
the queued jobs.

Remaining work:

- feed the collected signed range into one-step TAN-required detection;
- add full signed replay fixtures across init, execute, and close;
- port further PinTAN-compatible jobs behind the signed `CustomMsg` boundary.

## Links

- `docs/adr/0154-signed-pintan-dialog-init.md`
- `docs/adr/0155-signed-pintan-dialog-end.md`
- `src/manager/handler.rs`
- `src/manager/signature.rs`
- Upstream: `org.kapott.hbci.manager.HBCIKernelImpl`
- Upstream: `org.kapott.hbci.security.Sig.signIt`
