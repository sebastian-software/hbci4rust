# ADR 0068: Institute Message Collection Tracer

## Status

Accepted

## Context

hbci4java collects institute messages during dialog initialization by looping
over:

- `KIMsg`;
- `KIMsg_2`;
- `KIMsg_3`;
- ...

The loop uses `HBCIUtilsInternal.withCounter("KIMsg", i)`, where index `0`
keeps the base name and later indexes append `_<index + 1>`.

For each header, hbci4java constructs `HBCIInstMessage`. That constructor throws
when `<header>.betreff` is absent, and the surrounding dialog code catches the
exception and stops the loop.

ADR 0067 introduced the single-message `HbciInstMessage` structure but did not
add collection over counted `KIMsg` entries.

## Decision

Add `HbciInstMessage::collect_from_values(...)` for the current flat response
value map.

Use the upstream counter shape:

- `KIMsg`;
- `KIMsg_2`;
- `KIMsg_3`;
- ...

Stop at the first missing `<header>.betreff`.

Allow missing `<header>.text`, preserving the single-message display behavior
that renders absent text as `null`.

Do not wire the collection into `HbciHandler::init(...)` or callbacks in this
slice.

## Consequences

The Rust port can now collect institute messages from parsed dialog-init values
with the same sequence boundary as hbci4java.

Tests pin multi-message collection, missing-text behavior, stop-at-gap behavior,
and empty-first-message behavior.

The helper is ready for later dialog-init integration without changing the
current `HbciExecStatus::messages` return-code list.

Remaining work:

- add dialog-init storage or callback delivery for collected institute messages;
- decide whether the eventual callback should receive `HbciInstMessage` values
  or their display strings;
- centralize counted-prefix naming if more status/result types need the exact
  `withCounter` semantics outside the manager/passport internals.

## Links

- `src/gv_result/mod.rs`
- `tests/status.rs`
- `docs/adr/0067-institute-message-tracer.md`
- Upstream: `org.kapott.hbci.status.HBCIInstMessage`
- Upstream: `org.kapott.hbci.manager.HBCIDialog`
- Upstream: `org.kapott.hbci.manager.HBCIUtilsInternal#withCounter`
