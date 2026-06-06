# ADR 0124: String Utility Helpers

## Status

Accepted

## Context

The hbci4java offline test class `TestStringUtil` covers
`org.kapott.hbci.tools.StringUtil.join(...)`. The original utility class also
contains small helpers used around HBCI code names and tolerant string parsing:

- `toInsCode(...)`;
- `toParameterCode(...)`;
- `toBoolean(...)`;
- `hasText(...)`;
- `join(...)`.

The Rust port does not yet have a general `tools` module. Several existing
slices already reimplemented small string details locally, but the upstream
utility class is a stable Phase 1 utility boundary and is cheap to port
directly.

## Decision

Add a public `tools` module with Rust-cased helper names:

- `to_ins_code(...)`;
- `to_parameter_code(...)`;
- `to_boolean(...)`;
- `has_text(...)`;
- `join_strings(...)`.

Represent Java `null` input and output with `Option`. Keep `join_strings`
close to hbci4java: `None` values list returns `None`, a `None` separator joins
without separators, and empty strings are preserved.

Port the four upstream `TestStringUtil` assertions as Rust tests and add small
tracer tests for the other helpers from the same original class.

## Consequences

The Rust port gains a reusable, original-near utility module without coupling it
to current protocol or job code. Future slices can replace local ad hoc string
behavior with these helpers when it improves parity or readability.
