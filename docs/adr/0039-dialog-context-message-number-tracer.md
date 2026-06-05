# ADR 0039: Dialog Context Message Number Tracer

## Status

Accepted

## Context

ADR 0038 added a replay-testable `DialogInit` tracer, but follow-up job messages
still rendered as if no dialog had been opened: `MsgHead.dialogid = 0` and
`MsgHead.msgnum = 1`.

hbci4java carries a dialog id and increments the user message number during a
dialog. The original message definitions reflect this: `DialogInitRes.MsgHead`
returns the bank-assigned dialog id, and subsequent user messages use
`MsgHead.dialogid`, `MsgHead.msgnum`, and `MsgTail.msgnum`.

## Decision

Use the existing Rust `DialogContext` as the handler's volatile dialog state.

`DialogContext` now defaults to no dialog id and message number `1`, preserving
the current offline `execute()` behavior when no initialization happened.

After a successful `DialogInitRes` parse, `HbciHandler::init` reads
`DialogInitRes.MsgHead.dialogid`, stores it in `DialogContext`, and sets the
next outgoing message number to `2`.

`HbciHandler::execute` renders `CustomMsg.MsgHead.dialogid` from the current
dialog context and uses the current message number for both `MsgHead.msgnum` and
`MsgTail.msgnum`. Once a transport response is received, the handler advances
the message number.

Expose `HbciHandler::dialog_context()` for replay tests and early integration
inspection.

## Consequences

Init-to-execute replay tests now cover the first dialog continuity invariant:
the bank-assigned dialog id from `DialogInitRes` is used by the next `CustomMsg`,
and the message number moves from `1` to `2`.

This is still a tracer, not complete dialog lifecycle management:

- dialog ids and message counters are not persisted;
- response message references are not validated against the outgoing request;
- dialog end is not implemented;
- failed transport sends do not advance the counter because no response was
  received;
- SCA/TAN continuation messages still need their own context rules.

## Links

- `src/dialog/mod.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `resources/protocol/hbci-300.xml`
- Upstream: `org.kapott.hbci.dialog.HBCIDialog`
