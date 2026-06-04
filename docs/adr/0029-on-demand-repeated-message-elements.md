# ADR 0029: On-Demand Repeated Message Elements

## Status

Accepted

## Context

hbci4java stores repeated XML references in `MultipleSyntaxElements`
containers. When `propagateValue(...)` receives a path such as `RetVal_2.text`,
the corresponding second element can be created on demand as long as the XML
`maxnum` permits it.

The Rust message tree initially instantiated one element for each child
reference, or the required `minnum` count when greater than one. That was enough
for simple messages, but it blocked original-near outgoing paths that use
hbci4java counter suffixes for repeated elements, for example repeated return
values, usage lines, SEPA payload fragments, or repeated segment returns.

## Decision

When `HbciMessage::set_value` targets a missing path whose immediate child name
matches an existing child template with a suffix such as `_2`, create the missing
occurrences up to that suffix.

The generated occurrences:

- reuse the same Java-near path suffix convention as hbci4java;
- respect XML `maxnum`, where `0` remains unbounded;
- preserve XML default values such as segment code and version;
- clear explicit values and request tags from the template occurrence.

## Consequences

Outgoing message construction can now set repeated paths without pre-building
every possible occurrence. This keeps the tree close to hbci4java's behavior
without eagerly expanding unbounded XML references.

The implementation still uses a flattened `SyntaxElement` tree rather than
explicit `MultipleSyntaxElements` containers. More complicated cases, especially
large `GV`/`GVRes` syntax functions and names containing dotted path fragments,
still need focused parity tests before they are considered complete.

## Links

- `src/protocol/message.rs`
- `tests/protocol_message.rs`
- Upstream: `org.kapott.hbci.protocol.SyntaxElement.propagateValue`
- Upstream: `org.kapott.hbci.protocol.MultipleSyntaxElements.propagateValue`
