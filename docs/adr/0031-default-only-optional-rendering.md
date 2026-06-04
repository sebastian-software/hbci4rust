# ADR 0031: Default-Only Optional Rendering

## Status

Accepted

## Context

hbci4java applies XML `<value>` defaults to message elements during syntax tree
construction. Those defaults are needed when a segment is actually rendered,
for example segment code and version in `SegHead`.

The Rust message renderer originally treated any value, including an XML default
or generated sequence number, as evidence that an optional element should be
rendered. For `CustomMsg`, that made default-only GV segments such as `HKEKA`
appear in a message that only asked for `Saldo7`. It also made optional signature
segments fail during sequence preparation because generated sequence numbers
activated otherwise empty optional segments.

## Decision

Keep XML defaults on the message tree, but do not let defaults or generated
values activate optional elements.

An optional element is considered requested for rendering only when it contains
an explicit value set through `set_value(...)` or an explicit `requested` tag.
When it is rendered, XML defaults still participate normally in the FinTS output.
Generated values such as segment sequence numbers and message size are tracked
separately from explicit values.

## Consequences

`CustomMsg` can render a single requested GV segment without dragging along
default-only sibling segments.

The renderer remains close to hbci4java's practical behavior for outgoing job
messages while still preserving defaults needed by selected segments. More
complex cases around signatures, TAN steps, and optional segment groups still
need focused parity tests.

## Links

- `src/protocol/message.rs`
- `tests/protocol_message.rs`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement.validate`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement.propagateValue`
