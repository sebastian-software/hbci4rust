# ADR 0072: Dialog Status Tracer

## Status

Accepted

## Context

hbci4java has `HBCIDialogStatus` for the status of one complete HBCI dialog. It
aggregates:

- `initStatus` for dialog initialization;
- `msgStatus[]` for business-message exchanges;
- `endStatus` for dialog termination.

The Java class has original-near behavior that matters for later handler
porting:

- `isOK()` is strict about the dialog shape: init and end status must both be
  present and OK;
- each business-message status is checked through `HBCIMsgStatus.isOK()`, which
  only checks the global status;
- `getErrorString()` joins the init, business-message, and end error strings
  without adding section labels;
- `toString()` renders fixed sections using localized labels from
  `hbci4java-messages.properties`.

ADR 0071 introduced `HbciMsgStatus`, but the Rust handler still exposes the
current flat `HbciExecStatus`.

## Decision

Add `HbciDialogStatus` as a pure status aggregate with Rust-cased public fields:

- `message_statuses`;
- `init_status`;
- `end_status`.

Add original-near methods:

- `new()`;
- `set_init_status(...)`;
- `set_message_statuses(...)`;
- `set_end_status(...)`;
- `is_ok()`;
- `has_exceptions()`;
- `error_string()`;
- `Display`.

Use the English upstream message-bundle labels for display output:

- `DIALOG-INIT`;
- `DIALOG-MSG`;
- `DIALOG-END`.

Keep this as a structural/status tracer for now. Do not wire the handler's
dialog execution into `HbciDialogStatus` yet.

## Consequences

The Rust port now has an explicit equivalent for hbci4java's
`HBCIDialogStatus`.

The strict dialog `is_ok()` rule is pinned in tests: missing init or end status
is not OK.

The asymmetric message-status rule is also pinned at the dialog layer: a
business-message segment error does not make the dialog fail if the
message-global status is OK.

Because the Rust port does not yet have a localization layer, display uses the
English upstream resource labels directly.

Remaining work:

- wire dialog init, business messages, and dialog end into `HbciDialogStatus`;
- decide whether `HbciExecStatus` should wrap a single dialog status or a
  customer-id keyed map once multi-customer behavior is ported;
- introduce a localization/resource decision if user-facing status labels need
  to vary by locale.

## Links

- `src/gv_result/mod.rs`
- `tests/status.rs`
- `docs/adr/0071-message-status-tracer.md`
- Upstream: `org.kapott.hbci.status.HBCIDialogStatus`
- Upstream resource: `hbci4java-messages.properties`
