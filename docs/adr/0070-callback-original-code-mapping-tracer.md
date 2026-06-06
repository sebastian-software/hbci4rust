# ADR 0070: Callback Original Code Mapping Tracer

## Status

Accepted

## Context

hbci4java exposes callback reasons and callback data types as integer constants
on `HBCICallback`.

The Rust port already has Rust-cased enums:

- `CallbackReason`;
- `CallbackDataType`.

The enums contained an `Unknown(i32)` variant, but they did not yet expose the
original hbci4java numeric constants or conversion helpers.

The v1 Rust scope remains PinTAN/HBCI-Plus only. Chipcard and key-file live
support are out of scope.

## Decision

Add original numeric constants and helpers for the callback variants currently
present in the Rust API:

- `CallbackReason::original_code()`;
- `CallbackReason::from_original_code(...)`;
- `CallbackDataType::original_code()`;
- `CallbackDataType::from_original_code(...)`.

Preserve unknown codes through `Unknown(i32)`.

Map `CallbackDataType::Select` to upstream `TYPE_TEXT` when emitting an
original code. hbci4java has no separate `TYPE_SELECT`; selections are encoded
as text response data. Decode upstream `TYPE_TEXT` back to the canonical Rust
`CallbackDataType::Text`.

Do not add Rust enum variants for chipcard, RDH/key-file, or other out-of-scope
callbacks in this slice.

## Consequences

The callback API can now retain and expose hbci4java callback numbers where the
current Rust surface has matching concepts.

Tests pin the mapped data-type constants, PinTAN/connection/institute-message
reason constants, unknown-code preservation, and the `Select`/`TYPE_TEXT`
boundary.

Remaining work:

- add more callback variants only when their flows enter v1 scope;
- decide whether status callback constants such as `STATUS_DIALOG_INIT` need a
  separate Rust enum;
- document callback reason codes in public API docs once the callback surface is
  more complete.

## Links

- `src/callback.rs`
- `tests/callback.rs`
- `docs/adr/0069-dialog-init-institute-message-callback-tracer.md`
- Upstream: `org.kapott.hbci.callback.HBCICallback`
