# ADR 0074: Handler Dialog Status Integration Tracer

## Status

Accepted

## Context

ADRs 0071-0073 introduced the original-near status hierarchy:

- `HbciMsgStatus`;
- `HbciDialogStatus`;
- `HbciExecStatus` with customer-ID keyed dialog status data.

The current Rust handler API is still split into explicit calls:

- `init()`;
- `execute()`;
- `close()`.

hbci4java's `HBCIHandler.execute()` produces an `HBCIExecStatus` after the
dialog engine has managed the complete flow. The Rust port cannot fully mirror
that yet without changing the public control flow.

## Decision

Add an internal `dialog_status: HbciDialogStatus` field to `HbciHandler`.

Expose `HbciHandler::dialog_status()` as a read-only transitional helper.

Populate the internal dialog status in the split handler flow:

- `init()` parses `DialogInitRes.RetGlob`/`RetSeg` into `init_status`;
- `execute()` parses `CustomMsgRes.RetGlob`/`RetSeg` into a business-message
  status and appends it when a prior init status exists;
- `close()` parses `DialogEndRes.RetGlob`/`RetSeg` into `end_status` before
  applying the existing close-success check.

When `execute()` is called after `init()`, attach the current dialog status to
the returned `HbciExecStatus` under the effective customer ID.

When `execute()` is called directly for the older offline replay style, keep the
result flat and do not attach a customer-ID dialog map.

Use the same original-near `HbciMsgStatus::is_ok()` rule for dialog end as the
Java status layer: a segment error does not make the message status fail when
the global status is OK.

## Consequences

The handler now starts feeding the status hierarchy that was ported in ADRs
0071-0073.

Existing flat replay tests remain valid because direct `execute()` calls still
return a flat `HbciExecStatus`.

For the split API, `execute()` returns a dialog status with init and business
messages but no end status yet. Therefore `HbciExecStatus::success` can be true
while `HbciExecStatus::is_ok()` is false until `close()` has run and the final
dialog end status exists.

`close()` stores the end status internally, but it still returns `()`. A later
slice must decide whether to return an updated execution status, add a complete
dialog-run method, or make `execute()` own the full init/business/end flow.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0071-message-status-tracer.md`
- `docs/adr/0072-dialog-status-tracer.md`
- `docs/adr/0073-exec-status-customer-map-tracer.md`
- Upstream: `org.kapott.hbci.manager.HBCIHandler`
- Upstream: `org.kapott.hbci.status.HBCIExecStatus`
