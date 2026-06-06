# ADR 0101: Swift Umlaut Decoding

## Status

Accepted

## Context

hbci4java's turnover result path calls `Swift.decodeUmlauts(...)` before adding
raw MT940/MT942 payloads to `GVRKUms`. The upstream helper is intentionally
small: it replaces four SWIFT/DIN-style placeholder characters:

- `[` -> `Ä`;
- `\` -> `Ö`;
- `]` -> `Ü`;
- `~` -> `ß`.

ADR 0100 kept `KUmsAll` and `KUmsNew` result data raw because the structured
`GVRKUms`/MT940/MT942 parser is not ported yet. The decoding helper is still a
useful independent offline-domain building block for the next parser slices.

## Decision

Add `swift::decode_umlauts(input)` as an exact Rust-cased port of
`Swift.decodeUmlauts(...)`.

Do not broaden the behavior into a general character-set conversion. Lowercase
umlauts, braces, accented characters, and non-German symbols are left unchanged
unless the upstream helper replaces them.

Do not apply this helper to `HbciJobResult::result_data` yet. The KUms
`content.booked` and `content.notbooked` values remain raw until a `GvrKUms`
result type and MT940/MT942 append path are ported.

## Consequences

Future `GvrKUms` and MT940/MT942 parser slices can call the same decoding helper
that hbci4java uses without re-deciding the encoding boundary.

The function is intentionally narrow and may look incomplete compared to a full
SWIFT character-set conversion; that is original-near rather than accidental.

Remaining work:

- port `Swift.getOneBlock(...)`, `Swift.getTagValue(...)`, `packMulti(...)`, and
  `getMultiTagValue(...)` with upstream broken-MT940 fixtures;
- wire `decode_umlauts(...)` into `GvrKUms` once raw MT940/MT942 storage exists;
- port the full MT940/MT942 parser behavior.

## Links

- `src/swift/mod.rs`
- `tests/swift.rs`
- `docs/adr/0100-kums-raw-result-data-tracer.md`
- Upstream: `org.kapott.hbci.swift.Swift#decodeUmlauts`
