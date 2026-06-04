# ADR 0015: Incoming Datatype Parsing

## Status

Accepted

## Context

Outgoing message rendering already routes data-element values through
hbci4java-like datatype handling. Incoming segment value extraction initially
preserved raw wire values only.

hbci4java parses incoming data elements through the same datatype classes used
for rendering. Some types keep their wire representation, while others expose a
more human-readable value through `toString()`, for example `Date`, `Time`,
`Ctr`, `Float`, and `Bin`.

## Decision

Add `parse_data_element` beside `render_data_element` in the protocol datatype
module and use it during `ResolvedWireSegment::values`.

The first incoming datatype slice supports the types already relevant to the
current protocol parser:

- String-like values: `AN`, `Code`, `DTAUS`, `JN`, `ID`
- Numeric values: `Num`, `Dig`
- Country and currency values: `Ctr`, `Cur`
- Temporal values: `Date`, `Time`
- Decimal values: `Float`, `Wrt`
- Binary values: `Bin`

Incoming `Date` and `Time` are converted from HBCI wire format (`YYYYMMDD`,
`HHMMSS`) into the same readable ISO-style values accepted by outgoing rendering
(`YYYY-MM-DD`, `HH:MM:SS`). Incoming `Bin` strips the `@len@` envelope and
returns the payload. Empty wire components remain empty values, preserving ADR
0014's segment extraction behavior for explicitly empty optional elements.

For `Ctr`, the Rust port rejects unknown numeric country codes for now. hbci4java
falls back to `DE` after logging; the stricter behavior is easier to test and can
be revisited if a real-bank fixture requires the fallback.

## Consequences

Incoming segment maps now contain typed, Java-near values instead of only raw
wire strings. For example, `StatusRes4.date` is exposed as `2024-02-29`.

The datatype module now has both render and parse directions, which makes later
golden tests easier to express around one protocol boundary.

Some edge cases remain intentionally deferred: complete numeric binary handling,
full locale-equivalent decimal formatting, and hbci4java's country-code fallback.

## Links

- `src/protocol/datatype.rs`
- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.datatypes.SyntaxDate`
- Upstream: `org.kapott.hbci.datatypes.SyntaxTime`
- Upstream: `org.kapott.hbci.datatypes.SyntaxCtr`
- Upstream: `org.kapott.hbci.datatypes.SyntaxBin`
- Upstream: `org.kapott.hbci.datatypes.SyntaxFloat`
