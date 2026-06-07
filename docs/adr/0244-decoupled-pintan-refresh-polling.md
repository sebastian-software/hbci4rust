# ADR 0244: Decoupled PinTAN Refresh Polling

## Status

Accepted

## Context

ADRs 0242 and 0243 added the two prerequisites for decoupled PinTAN runtime
support:

- a `TAN2Step`/`HKTAN` status request helper with `process=S`;
- Rust callback reasons for hbci4java's `NEED_PT_DECOUPLED` and
  `NEED_PT_DECOUPLED_RETRY` constants.

The remaining runtime gap is handling the observable decoupled flow:

1. the initial HITAN challenge tells the user to approve the order in another
   app or device;
2. no TAN is entered into the FinTS signature for that challenge;
3. while the bank returns warning `3956`, hbci4java notifies the callback,
   waits according to BPD timing hints, and sends another `process=S` status
   request.

The Rust port is async-first and must not own a runtime, but it can await Tokio
timers inside async handler methods.

## Decision

Port the first automatic decoupled polling path close to hbci4java:

- when the current TAN mechanism is detected as `Decoupled`, emit
  `CallbackReason::NeedPtDecoupled` with the formatted challenge text and HHD-UC
  payload, ignore the callback response, and sign with PIN only;
- track decoupled refresh attempts as runtime PinTAN SCA state, resetting the
  counter when a new HITAN order reference is stored;
- add a handler helper that performs one decoupled status poll by:
  - rejecting a non-empty queue;
  - checking BPD max-refresh hints before sending;
  - emitting `CallbackReason::NeedPtDecoupledRetry` with the minimum wait time
    in seconds as the current value;
  - awaiting any remaining required delay after the callback returns;
  - queuing and executing the existing `process=S` status request;
  - clearing SCA state only after a successful status response that no longer
    contains warning `3956`;
- add a bounded automatic loop to the process-2 helper while status responses
  still contain `3956`.

Use BPD hints from the already ported `ParameterQuery` constants:

- `decoupled_time_before_first_status_request`;
- `decoupled_time_before_next_status_request`;
- `decoupled_max_status_requests`.

If the bank does not provide a max-refresh hint, the automatic process-2 helper
will perform only one status poll per call. Applications can call the explicit
poll helper again if they want to continue. This keeps v1 from creating an
unbounded library loop while preserving the original request shape and callback
contract.

## Consequences

The decoupled PinTAN flow becomes usable in replay tests without requiring a TAN
value from the application. The handler now has enough behavior to exercise
pending approval (`3956`) and successful approval through deterministic offline
fixtures.

The Rust port intentionally keeps QR/photoTAN callback emission separate. ADR
0245 records that later runtime decision.

## Links

- ADR 0143: SCA TAN Callback Helper
- ADR 0242: Decoupled PinTAN Status Request Helper
- ADR 0243: Decoupled PinTAN Callback Code Mapping
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/passport/AbstractPinTanPassport.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/passport/HBCIPassportPinTan.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVTAN2Step.java`
