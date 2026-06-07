# ADR 0242: Decoupled PinTAN Status Request Helper

## Status

Accepted

## Context

hbci4java supports decoupled PinTAN flows where the user confirms an order in a
separate app or device. While the approval is still pending, banks can return
FinTS return code `3956`. hbci4java reacts by notifying the callback layer,
waiting according to BPD timing hints, and repeating a status request.

The upstream status request is another `HKTAN`/`TAN2Step` message with
`process=S`, the stored order reference, and `notlasttan=N`. It polls the
existing decoupled order. It must not request a fresh TAN from the user.

The Rust port already stores HITAN/SCA state, can render `TAN2Step5`, and keeps
`KnownReturncode::W3956`. However, it does not yet have a status request helper
or the full decoupled retry loop.

## Decision

Add the first decoupled building block:

- expose `HbciHandler::new_tan2step_decoupled_status_job()`;
- build the job from the current PinTAN SCA order reference;
- set `process=S`, `orderref=<stored order reference>`, and `notlasttan=N`;
- reject the helper when no non-empty order reference is available;
- treat `process=S` as a status poll that signs with the cached/requested PIN
  but does not request another TAN from the callback.

Do not yet port the full hbci4java decoupled refresh loop, retry counters,
minimum waiting times, or dedicated decoupled callback reasons. Those remain
separate runtime slices and are addressed later by ADRs 0243 and 0244.

## Consequences

The Rust port can now render and replay-test the core FinTS message shape needed
for decoupled status polling. This reduces the remaining decoupled gap without
changing the public `execute()` or `execute_with_tan2step()` control flow.

ADR 0244 builds on this helper to detect `3956`, emit decoupled callback events,
respect BPD wait/max-refresh hints, and merge status poll results like
hbci4java.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/passport/AbstractPinTanPassport.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVTAN2Step.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/callback/HBCICallback.java`
- ADR 0142: HITAN SCA State Extraction
- ADR 0167: PinTAN TAN Process Dispatcher
