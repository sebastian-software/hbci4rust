# ADR 0102: Swift Tag Value Extraction

## Status

Accepted

## Context

hbci4java's MT940 parser uses `Swift.getTagValue(...)` to extract values from
SWIFT tags such as `:60M:` or `:25:`. The helper also tolerates malformed bank
output where a dash appears before the next tag:

- `\r\n-:61:...`;
- `\r\n-\r\n:61:...`.

The upstream repository has focused `TestBrokenMT940` fixtures for this behavior.
ADR 0101 ported the smaller `Swift.decodeUmlauts(...)` helper and left the other
SWIFT helpers for later MT940 parser slices.

## Decision

Add `swift::get_tag_value(input, tag, counter) -> Option<String>`.

The function is an original-near Rust-cased port of `Swift.getTagValue(...)`:

- search starts at `\r\n:<tag>:` and also supports `\r\n-:<tag>:`;
- the end of a value is the next CRLF tag marker matching hbci4java's
  `\r\n(-|-\r\n)?:\d{2}[A-Z]?:` shape;
- `counter` is zero-based, as in the upstream helper;
- Java `null` becomes Rust `None`.

For the final tag in a stream, keep hbci4java's permissive cleanup of trailing
line-break/dash noise. This intentionally supports broken MT940 blocks before a
full parser exists.

Do not port `Swift.getOneBlock(...)`, `packMulti(...)`, or
`getMultiTagValue(...)` in this slice.

## Consequences

The Rust port can now extract simple MT940 tag values and tolerate the broken
line shapes covered by upstream fixtures.

This is still not a full MT940 parser; it is a reusable helper for the upcoming
`GVRKUms`/MT940 slices.

Remaining work:

- port `Swift.getOneBlock(...)`;
- port `Swift.packMulti(...)` and `Swift.getMultiTagValue(...)`;
- wire the helpers into an original-near MT940 parser.

## Links

- `src/swift/mod.rs`
- `tests/swift.rs`
- `docs/adr/0101-swift-umlaut-decoding.md`
- Upstream: `org.kapott.hbci.swift.Swift#getTagValue`
- Upstream test: `org.kapott.hbci4java.swift.TestBrokenMT940`
