# ADR 0040: Dialog End Close Tracer

## Status

Accepted

## Context

ADR 0038 and ADR 0039 established a replayable dialog start and carried the
bank-assigned dialog id into subsequent job messages. The dialog lifecycle still
had no explicit end step.

hbci4java exposes `HBCIHandler.close()` and uses `HBCIDialogEnd` to send the
`DialogEnd` template. `HBCIDialogEnd.applyData(...)` sets
`DialogEndS.dialogid` from the current dialog context. The upstream code can
ignore some dialog-end failures through `client.errors.ignoreDialogEndErrors` or
anonymous-dialog handling.

## Decision

Add `HbciHandler::close().await` as the Rust tracer for hbci4java
`HBCIHandler.close()`.

For this tracer:

- calling `close()` without an open dialog is a no-op;
- calling `close()` with an open dialog renders `DialogEnd` through the original
  `hbci-*.xml` message tree;
- `MsgHead.dialogid` and `DialogEndS.dialogid` use the current dialog id;
- `MsgHead.msgnum` and `MsgTail.msgnum` use the current message number;
- no signature or encryption segments are rendered yet;
- the response is parsed as `DialogEndRes`;
- successful global or segment return values reset the volatile dialog context;
- error return values become a protocol error and keep the dialog context for
  diagnosis or a later retry.

Do not implement hbci4java's configurable dialog-end-error ignore behavior yet.
That policy needs runtime parameter handling and live-bank fixtures before it can
be ported faithfully.

## Consequences

The replayable PinTAN lifecycle now covers the shape:

1. `DialogInit`
2. `CustomMsg`
3. `DialogEnd`

The handler can prove that `HKEND` uses the dialog id and current message number
from the prior dialog state.

Remaining gaps:

- no signed or encrypted dialog end;
- no `DialogEndAnon`;
- no response message-reference validation;
- no passport resource close side effects;
- no configurable `client.errors.ignoreDialogEndErrors` policy;
- no automatic close-on-drop behavior.

## Links

- `src/dialog/mod.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `resources/protocol/hbci-300.xml`
- Upstream: `org.kapott.hbci.manager.HBCIHandler#close`
- Upstream: `org.kapott.hbci.dialog.HBCIDialogEnd`
