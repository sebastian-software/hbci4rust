# ADR 0014: Incoming Segment Value Extraction

## Status

Accepted

## Context

ADR 0011 tokenizes raw FinTS wire messages into segments, fields, and
components. ADR 0013 resolves each wire segment to an XML `SEGdef`.

The next incoming-parser step needs to preserve the hbci4java path vocabulary so
ported tests and future job/result code can compare parsed values against the
same names used by the Java implementation.

## Decision

Add a first flat value-extraction step on `ResolvedWireSegment`.

`ResolvedWireSegment::values(&ProtocolSyntax)` walks the resolved `SEGdef` and
referenced `DEGdef` children while consuming the already-tokenized wire fields
and components. It returns a `BTreeMap<String, String>` keyed by original-near
paths.

Path rules match the existing message tree:

- The segment definition id is used as the root, for example `RetGlob`.
- Child `name` is preferred over child `type`; if `name` is absent, `type` is
  used.
- Repeated occurrences use hbci4java-style suffixes where the first occurrence
  has no suffix and later occurrences use `_2`, `_3`, and so on.

The first slice supports `DE` and `DEG` children inside resolved segments and
data-element groups. It preserves empty values when an empty field/component is
present on the wire.

This does not yet reconstruct full `MSG`/`SF` context paths, run datatype parse
conversion, validate value ranges, or apply hbci4java rewrite hooks.

## Consequences

Incoming RetGlob/HIRMG-style segments can now be represented as Java-like path
maps such as `RetGlob.RetVal_2.text`.

Keeping this as flat segment-level extraction avoids guessing message context
too early. A later message parser can place resolved segments into `MSG` and
`SF` containers and either reuse these paths or prefix them with the surrounding
message path.

## Links

- `src/protocol/wire.rs`
- `tests/protocol_wire.rs`
- Upstream: `org.kapott.hbci.protocol.DE.extractValues`
- Upstream: `org.kapott.hbci.protocol.MultipleSyntaxElements.extractValues`
