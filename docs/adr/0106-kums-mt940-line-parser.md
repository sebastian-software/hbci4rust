# ADR 0106: KUms MT940 Line Parser

## Status

Accepted

## Context

ADR 0105 added a `GVRKUms`-near result shell that splits MT940/MT942 blocks and
parses account, counter, opening balance, and closing balance fields. The next
upstream parser layer loops over repeated `:61:` tags and fills `UmsLine`
records before `:86:` multitag details are applied.

hbci4java's `:61:` parser performs several small but important transformations:

- `valuta` comes from the first six characters;
- booking date is optional and falls back to the block opening balance date, or
  to `valuta` when no opening balance exists;
- four-character booking dates reuse the value date year and are corrected by
  one year when they differ from value date by more than 180 days;
- credit/debit and reversal markers decide the sign of the booked amount;
- MT942 lines without an opening balance use `EUR` as the line currency;
- the running balance after each line is accumulated from the opening balance;
- customer reference, institution reference, `/OCMT/`, and `/CHGS/` data are
  parsed before `:86:` processing.

## Decision

Port the `:61:` line parser into the existing `GvrKUms` shell.

This slice fills:

- `GvrKUmsLine.valuta`;
- `GvrKUmsLine.bdate`;
- `GvrKUmsLine.value`;
- `GvrKUmsLine.is_storno`;
- `GvrKUmsLine.saldo`;
- `GvrKUmsLine.customerref`;
- `GvrKUmsLine.instref`;
- `GvrKUmsLine.orig_value`;
- `GvrKUmsLine.charge_value`.

Keep dates as original-near `yyMMdd` strings for now, matching ADR 0105's
string-backed `Saldo.date` choice. Implement the Java half-year correction on
those `yyMMdd` strings instead of introducing a date dependency in this slice.

Do not parse `:86:` transaction details yet. That means `gvcode`, `text`,
`primanota`, `usage`, `other`, `addkey`, and `additional` remain empty until the
next MT940 parser slice.

Malformed `:61:` values are skipped by the Rust shell for now. hbci4java throws
inside `parseMT94x`; a later hardening slice can decide how to model parser
errors without destabilizing the current result API.

## Consequences

`GvrKUms::get_flat_data()` and `get_flat_data_unbooked()` now return real
transaction lines for valid `:61:` tags.

The parser still does not expose human-readable usage text or counter-account
data from `:86:`; upstream `TestMT940Parse` parity is closer but not complete.

Remaining work:

- port `:86:` multitag mapping into `GvrKUmsLine`;
- port final balance correction after closing balance parsing;
- decide parser error reporting for malformed MT940 input;
- integrate structured `GvrKUms` into `KUmsAll`/`KUmsNew` handler results.

## Links

- `src/gv_result/mod.rs`
- `tests/structures.rs`
- `docs/adr/0105-kums-mt940-result-shell.md`
- Upstream: `org.kapott.hbci.GV_Result.GVRKUms#parseMT94x`
- Upstream test: `org.kapott.hbci4java.swift.TestMT940Parse`
