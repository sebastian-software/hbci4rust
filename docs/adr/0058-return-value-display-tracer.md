# ADR 0058: Return Value Display Tracer

## Status

Accepted

## Context

hbci4java's `HBCIRetVal.toString()` is the compact display form for a single
bank return code.

The Rust port already has `HbciReturnValue` with the original-near fields:

- `code`;
- `segment_ref`;
- `data_ref`;
- `text`;
- `params`.

It also exposed `message()` with a string shape close to the upstream
`toString()`, but did not yet implement Rust `Display`.

## Decision

Implement `Display` for `HbciReturnValue`.

Render:

- `<code>:<text>`;
- every parameter as ` p:<param>`;
- optional segment reference as ` (<segment_ref>)`;
- optional data reference as ` (<segment_ref>:<data_ref>)`.

Keep `HbciReturnValue::message()` as a compatibility helper that delegates to
`Display`.

Do not add the upstream `element` field in this slice. The Rust parser does not
yet reconstruct that low-level path/value annotation for return values.

## Consequences

Callers can now use standard Rust formatting for individual return values while
keeping the existing `message()` API stable.

Tests pin the compact return-code display shape, parameter ordering, optional
reference rendering, and `message()` compatibility.

Remaining work:

- decide whether to store and render hbci4java's optional `element` annotation;
- add display support for grouped status and execution status types.

## Links

- `src/gv_result/mod.rs`
- `tests/status.rs`
- Upstream: `org.kapott.hbci.status.HBCIRetVal#toString`
