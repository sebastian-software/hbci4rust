# ADR 0023: Incoming Valid Check Option

## Status

Accepted

## Context

hbci4java's incoming `MSG` constructor accepts a `checkValids` flag. When this
flag is disabled, the parser passes `null` instead of a valid-values table, so
`DE.parseValue` skips XML `<valids>` checks. XML `<value>` predefinitions are
still collected and checked.

The Rust incoming parser initially always checked both defaults and valid
values. That is a good strict default, but it did not preserve the original
escape hatch used by hbci4java rewriters and tolerant parsing paths.

## Decision

Add `IncomingValidation` as the Rust-side incoming value extraction option.

The default is `IncomingValidation::strict()`, which checks XML `<value>` and
`<valids>` metadata. `IncomingValidation::unchecked_valids()` skips only
`<valids>` validation while keeping XML `<value>` defaults enforced.

The option is exposed on resolved segment values, flat resolved wire-message
values, and direct `MSGdef` value mapping. Existing `values(...)` methods keep
strict behavior.

## Consequences

Strict incoming parsing remains the default for offline tests and normal use.
Ported rewrite paths can now mirror hbci4java's `DONT_CHECK_VALIDS` behavior
without weakening predefined fixed-field checks such as `hbciversion`,
`SegHead.code`, or `SegHead.version`.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.MSG`
- Upstream: `org.kapott.hbci.protocol.DE.parseValue`
