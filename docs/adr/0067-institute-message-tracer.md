# ADR 0067: Institute Message Tracer

## Status

Accepted

## Context

hbci4java has `org.kapott.hbci.status.HBCIInstMessage` for institute messages
reported by the bank during dialog initialization.

The upstream constructor reads:

- `<header>.betreff`;
- `<header>.text`.

It rejects the message when `betreff` is absent. It allows `text` to be absent,
and Java string concatenation renders that absent value as `null` in
`toString()`.

The current Rust `HbciExecStatus::messages` field contains compact return-value
strings collected from global and segment status values. It is not yet a list of
institute messages.

## Decision

Add `HbciInstMessage` in `src/gv_result/mod.rs`, alongside the other
status/result structures.

Use Rust-cased public field names:

- `subject` for upstream `betreff`;
- `text` as `Option<String>`.

Add `HbciInstMessage::from_values(...)` for the current flat response value map.

Render with `Display` as:

- `<subject>: <text>`;
- `<subject>: null` when `text` is absent.

Keep `HbciExecStatus::messages` unchanged in this slice. It continues to expose
return-value message strings, not `KIMsg` institute messages.

## Consequences

The port now has a first original-near representation for bank institute
messages.

Tests pin the display shape, missing-text rendering, original key names, and the
missing-subject error boundary.

Later parser work can collect `KIMsg`, `KIMsg_2`, ... values into a dedicated
field without changing the existing return-code message list.

Remaining work:

- decide where dialog-init institute messages should live in the Rust execution
  status once the status hierarchy is expanded;
- wire `KIMsg` extraction into dialog initialization;
- decide whether callbacks should receive the raw `HbciInstMessage` or only its
  display string.

## Links

- `src/gv_result/mod.rs`
- `tests/status.rs`
- Upstream: `org.kapott.hbci.status.HBCIInstMessage`
- Upstream: `org.kapott.hbci.manager.HBCIDialog`
