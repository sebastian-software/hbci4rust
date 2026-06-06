# ADR 0103: Swift One Block Extraction

## Status

Accepted

## Context

hbci4java parses MT940/MT942 data by repeatedly calling
`Swift.getOneBlock(StringBuffer)` and then deleting the returned block from the
caller-owned buffer. The helper itself does not mutate the `StringBuffer`.

The Java helper searches for the next `\r\n:20:` marker starting at index `1`,
so the first block marker at the beginning of a stream is ignored and the next
marker separates the following block. If no later marker exists, the whole
non-empty stream is returned. An empty stream returns Java `null`.

ADR 0102 added `swift::get_tag_value(...)`; the next MT940 parser slices need
the same original-near block boundary helper.

## Decision

Add `swift::get_one_block(input) -> Option<String>`.

The function is a pure, Rust-cased port of `Swift.getOneBlock(...)`:

- search for `\r\n:20:` starts at byte offset `1`, matching the Java index;
- if a later block marker exists, return the prefix before that marker;
- if no later marker exists and input is non-empty, return the whole input;
- if the input is empty, return `None` as the Java `null` equivalent.

Do not model `StringBuffer.delete(...)` here. Buffer advancement remains the
responsibility of the future `GVRKUms`/MT940 parser slice, just as hbci4java
keeps mutation in the caller.

## Consequences

The Rust port can split concatenated MT940 blocks at the same boundary used by
hbci4java while keeping this slice independent from the full transaction parser.

Remaining work:

- port `Swift.packMulti(...)` and `Swift.getMultiTagValue(...)`;
- add a `GVRKUms`-near MT940 parser that owns the remaining buffer and rest
  fields;
- port the upstream `TestMT940Parse` fixtures after transaction-line structures
  are available.

## Links

- `src/swift/mod.rs`
- `tests/swift.rs`
- `docs/adr/0102-swift-tag-value-extraction.md`
- Upstream: `org.kapott.hbci.swift.Swift#getOneBlock`
- Upstream use: `org.kapott.hbci.GV_Result.GVRKUms#parseMT94x`
