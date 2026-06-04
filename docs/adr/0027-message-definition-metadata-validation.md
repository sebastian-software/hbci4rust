# ADR 0027: Message Definition Metadata Validation

## Status

Accepted

## Context

hbci4java collects XML `<value>` defaults and `<valids>` lists from every parsed
`SyntaxElement`, including the top-level `MSGdef`. Those metadata entries are
checked when matching incoming data elements are parsed.

The Rust incoming mapper already validates metadata declared on `SEGdef` and
referenced `DEGdef` definitions. It did not yet validate metadata declared on
the `MSGdef` itself. HBCI protocol XML uses message-level defaults in key-file
related messages such as `SendKeys` and older initialization messages.

## Decision

After `values_for_message` walks a `MSGdef` and extracts values, validate the
resolved message values against the `MSGdef`'s own `<value>` and `<valids>`
metadata under the message root.

The same `IncomingValidation` options apply:

- XML `<value>` defaults are always checked.
- XML `<valids>` lists are checked in strict mode.
- `unchecked_valids()` skips only valid-value checks.

## Consequences

Message-level incoming parsing is closer to hbci4java's `SyntaxElement`
metadata behavior.

The implementation does not pull key-file support into v1 scope. Tests exercise
the metadata behavior directly instead of adding heavyweight key-file wire
fixtures.

## Links

- `src/protocol/wire.rs`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement`
- Upstream: `org.kapott.hbci.protocol.DE.parseValue`
