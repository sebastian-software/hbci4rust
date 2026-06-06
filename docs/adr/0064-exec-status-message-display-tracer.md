# ADR 0064: Exec Status Message Display Tracer

## Status

Accepted

## Context

hbci4java has three related status aggregation layers:

- `HBCIMsgStatus` for one message exchange, with global and segment status;
- `HBCIDialogStatus` for dialog init, business messages, and dialog end;
- `HBCIExecStatus` for all executed customer-id dialogs.

The current Rust `HbciExecStatus` is still a flat handler result. It stores:

- global return values;
- segment return values;
- job results;
- raw status messages;
- a computed `success` flag.

It does not yet model the full hbci4java `HBCIExecStatus` customer-id map or the
full `HBCIDialogStatus` hierarchy.

## Decision

Add `HbciExecStatus::error_string()` and `Display` using the current flat global
and segment return-value fields.

Match hbci4java's `HBCIMsgStatus` text aggregation:

- `error_string()` combines `global_status().error_string()` and
  `segment_status().error_string()`;
- `Display` combines `global_status().to_string()` and
  `segment_status().to_string()`;
- empty parts are omitted, matching the effective result of the upstream
  append-plus-`trim()` behavior.

Do not use `messages` for this display form. In the Rust port it is a collected
flat list of return-value messages, while the original display APIs operate on
the status objects.

Do not add or reinterpret `is_ok()` in this slice. `HbciExecStatus::success`
currently represents the handler's stricter execution result, while
`HBCIMsgStatus#isOK()` checks only the global message status.

## Consequences

Callers can format the current execution result and error text in the same shape
as hbci4java's single-message status output.

The implementation avoids prematurely creating the full Java status hierarchy
before the Rust dialog model is ready for it.

Remaining work:

- introduce explicit Rust types for `HBCIMsgStatus`, `HBCIDialogStatus`, and the
  multi-customer `HBCIExecStatus` equivalent if the port needs that hierarchy;
- decide how `HbciExecStatus::success` should relate to future original-near
  `is_ok()` APIs;
- carry exception objects or structured execution errors instead of plain
  strings if later parity tests require it.

## Links

- `src/gv_result/mod.rs`
- `tests/status.rs`
- Upstream: `org.kapott.hbci.status.HBCIMsgStatus#getErrorString`
- Upstream: `org.kapott.hbci.status.HBCIMsgStatus#toString`
- Upstream: `org.kapott.hbci.status.HBCIExecStatus`
- Upstream: `org.kapott.hbci.status.HBCIDialogStatus`
