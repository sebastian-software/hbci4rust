# ADR 0042: Open Dialog Response Id Validation Tracer

## Status

Accepted

## Context

ADR 0041 validates response `MsgHead.MsgRef` against the outgoing request. That
proves a response points to the request message, but it still did not check that
the response itself belongs to the currently open dialog.

`DialogInitRes` is a special case: the request uses dialog id `0`, and the bank
assigns the real dialog id in the response message head. For later messages in
an opened dialog, `CustomMsgRes.MsgHead.dialogid` and
`DialogEndRes.MsgHead.dialogid` should remain the current dialog id.

The early offline `CustomMsg` tracer can still run without `init()`, using
dialog id `0`. That mode is useful for fixture development, but it is not a
complete live-bank dialog.

## Decision

Validate response message-head dialog ids only for already opened dialogs.

For `CustomMsgRes` and `DialogEndRes`, if the outgoing request reference has a
non-zero dialog id, require:

- `{MessageName}.MsgHead.dialogid == request_ref.dialog_id`.

If the outgoing request uses dialog id `0`, skip this validation to preserve the
existing offline tracer behavior.

Do not apply this validation to `DialogInitRes`; it continues to provide the
new dialog id for `DialogContext`.

## Consequences

Replay tests now reject `CustomMsgRes` and `DialogEndRes` responses whose
`MsgRef` is correct but whose own response dialog id belongs to a different
dialog.

This tightens the live-dialog path without breaking no-init offline fixtures.

Remaining work:

- decide when the no-init `CustomMsg` path should be deprecated or moved behind
  an explicit test/replay mode;
- validate SCA continuation responses once those messages are ported;
- preserve response dialog-id details in richer status diagnostics.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0041-response-message-reference-validation-tracer.md`
- `resources/protocol/hbci-300.xml`
