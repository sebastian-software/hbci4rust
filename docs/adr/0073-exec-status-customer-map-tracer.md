# ADR 0073: Exec Status Customer Map Tracer

## Status

Accepted

## Context

hbci4java's `HBCIExecStatus` is not just a flat job-result object. It stores:

- a `HBCIDialogStatus` per customer ID;
- dialog-management exceptions per customer ID;
- helper methods for customer IDs, per-customer status lookup, error strings,
  display output, and `isOK()` checks.

The current Rust handler still returns a flat `HbciExecStatus` with one
business-message response shape:

- `success`;
- `job_results`;
- `messages`;
- flat global and segment return values.

That flat shape is useful for the current PinTAN replay tests, but it is not
close enough to the original status hierarchy for later dialog execution
porting.

## Decision

Add original-near map fields to `HbciExecStatus`:

- `dialog_statuses: BTreeMap<String, HbciDialogStatus>`;
- `exception_messages: BTreeMap<String, Vec<String>>`.

Use `BTreeMap`/`BTreeSet` instead of Java's `Hashtable`/`HashSet` so Rust test
output and display order are deterministic.

Add Rust-cased helpers corresponding to the Java API:

- `customer_ids()`;
- `add_dialog_status(...)`;
- `dialog_status(...)`;
- `dialog_status_list()`;
- `add_exception_message(...)`;
- `exception_messages(...)`;
- `to_string_for_customer(...)`;
- `is_ok_for_customer(...)`;
- `is_ok()`.

Keep the existing flat status behavior as a transitional fallback:

- if no dialog map data is present, `error_string()` and `Display` keep using
  the flat `HbciMsgStatus` view;
- if dialog map data is present, `error_string()` and `Display` use the
  original-near per-customer formatting.

Do not wire `HbciHandler::execute()` into the dialog-status map yet.

## Consequences

The Rust port now has the three main hbci4java status layers represented:

- `HbciStatus`;
- `HbciMsgStatus`;
- `HbciDialogStatus`;
- `HbciExecStatus`.

The new customer-map behavior is pinned in tests, including customer-ID union,
per-customer display, grouped multi-customer error strings, status removal, and
exception-driven `is_ok` failure.

The transitional fallback lets existing handler tests keep passing while the
handler still returns flat execution data.

Remaining work:

- populate `dialog_statuses` from `init()`, `execute()`, and `close()`;
- decide how the flat `success`, `job_results`, and return-value fields should
  coexist with the original status map once the handler becomes dialog-status
  driven;
- replace string-backed exception messages only if a later port slice needs
  richer error objects.

## Links

- `src/gv_result/mod.rs`
- `src/manager/handler.rs`
- `tests/status.rs`
- `docs/adr/0071-message-status-tracer.md`
- `docs/adr/0072-dialog-status-tracer.md`
- Upstream: `org.kapott.hbci.status.HBCIExecStatus`
