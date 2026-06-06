# ADR 0105: KUms MT940 Result Shell

## Status

Accepted

## Context

hbci4java represents account turnover results with `GVRKUms`. The class keeps
separate MT940 and MT942 buffers, appends raw `booked` and `notbooked` response
data, and parses lazily when callers ask for grouped or flat turnover data.

The full Java parser is large: it splits MT940 blocks, parses account metadata,
opening and closing balances, every `:61:` transaction line, `:86:` multitag
details, optional original/charge values, SEPA hints, and balance corrections.
ADR 0102 through ADR 0104 ported the reusable SWIFT helpers needed by that
parser.

The Rust port already collects raw `KUmsAll`/`KUmsNew` result payloads as
`content.booked` and `content.notbooked`, but it does not yet expose a
`GVRKUms`-near structured result type.

## Decision

Add an original-near Rust result shell:

- `GvrKUms`;
- `GvrKUmsBTag`;
- `GvrKUmsLine`.

The first parser slice preserves the hbci4java shape and lazy lifecycle:

- `append_mt940_data(...)` appends booked MT940 payloads;
- `append_mt942_data(...)` appends unbooked MT942 payloads;
- `get_data_per_day(...)` and `get_data_per_day_unbooked(...)` trigger lazy
  parsing;
- `get_flat_data(...)` and `get_flat_data_unbooked(...)` flatten parsed line
  vectors;
- `rest_mt940` and `rest_mt942` keep the remaining unparsed buffer text;
- `camt_booked` and `camt_not_booked` exist as public vectors matching the Java
  fields, but CAMT parsing remains out of this slice.

This slice parses only block-level MT940/MT942 data using the ported SWIFT
helpers:

- split blocks with `swift::get_one_block(...)`;
- read account information from `:25:`;
- read statement counter from `:28C:`;
- read opening balances from `:60F:` or `:60M:`;
- read closing balances from `:62F:` or `:62M:`.

It intentionally does not parse `:61:` transaction lines or `:86:` multitag
details yet. `GvrKUmsLine` is added with the Java field shape so the next slice
can fill it without reshaping the public data model.

For MT940 balance timestamps, keep the original six-character `yyMMdd` text in
`Saldo.date` for now. Do not invent an ISO conversion in this slice; hbci4java
stores a `Date` and formats later, while the current Rust `Saldo` stores strings.

Do not wire `GvrKUms` into `HbciHandler::execute()` yet. Handler integration
will be a separate ADR once the parser has at least `:61:` line parity.

## Consequences

The port now has the core `GVRKUms` result shape and lazy MT940/MT942 buffer
semantics needed for the larger transaction parser.

The flat data accessors return empty lists until the `:61:` parser slice lands,
even when block headers parse successfully.

Remaining work:

- port `:61:` transaction-line parsing;
- port `:86:` multitag mapping into `GvrKUmsLine`;
- port balance correction logic;
- integrate `GvrKUms` into `KUmsAll`/`KUmsNew` handler results;
- port upstream `TestMT940Parse` fixtures once flat transaction lines exist.

## Links

- `src/gv_result/mod.rs`
- `tests/structures.rs`
- `docs/adr/0102-swift-tag-value-extraction.md`
- `docs/adr/0103-swift-one-block-extraction.md`
- `docs/adr/0104-swift-multitag-helpers.md`
- Upstream: `org.kapott.hbci.GV_Result.GVRKUms`
- Upstream: `org.kapott.hbci.GV.GVKUmsAll#extractResults`
