# ADR 0104: Swift Multitag Helpers

## Status

Accepted

## Context

hbci4java's MT940 parser reads `:86:` transaction details as "multitag" data.
Before extracting values, `GVRKUms` strips the first three characters of the
`:86:` value and calls `Swift.packMulti(...)`; it then reads fields such as
`?00`, `?10`, `?20`, `?30`, and `?31` via `Swift.getMultiTagValue(...)`.

The upstream helpers are intentionally permissive:

- `packMulti(...)` removes only exact `\r\n` sequences;
- `getMultiTagValue(...)` searches for `?` plus the requested two-character
  tag;
- a value ends only at the next `?` followed by two digits;
- a lone `?`, a non-digit `?x`, or a too-short trailing marker remains part of
  the value.

ADR 0102 and ADR 0103 added the preceding SWIFT tag/block helpers. These
multitag helpers complete the small `Swift` helper layer needed before an
original-near `GVRKUms` parser slice.

## Decision

Add:

- `swift::pack_multi(input) -> String`;
- `swift::get_multi_tag_value(input, tag) -> Option<String>`.

Keep the Java helper's two-character tag assumption for value slicing. The
current hbci4java callers pass tags such as `00`, `10`, `20`, `30`, and `60`;
support for arbitrary-length tags is not part of this original-near slice.

Do not parse `:86:` into transaction structures here. That remains owned by the
future `GVRKUms` port.

## Consequences

The Rust port can now normalize and extract the `:86:` multitag fields needed by
the later MT940 transaction parser while preserving hbci4java's tolerance for
literal question marks inside values.

Remaining work:

- wire these helpers into a `GVRKUms`-near MT940 parser;
- port upstream MT940 fixtures into result-structure tests;
- revisit the two-character tag assumption only in `docs/rustification/` after
  original-near parity is covered.

## Links

- `src/swift/mod.rs`
- `tests/swift.rs`
- `docs/adr/0102-swift-tag-value-extraction.md`
- `docs/adr/0103-swift-one-block-extraction.md`
- Upstream: `org.kapott.hbci.swift.Swift#packMulti`
- Upstream: `org.kapott.hbci.swift.Swift#getMultiTagValue`
- Upstream use: `org.kapott.hbci.GV_Result.GVRKUms#parseMT94x`
