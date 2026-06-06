# ADR 0108: KUms MT940 Balance Correction

## Status

Accepted

## Context

ADR 0106 and ADR 0107 ported the MT940 `:61:` and `:86:` transaction parsing
layers into `GvrKUmsLine`. hbci4java applies one more block-level correction
after reading the closing balance from `:62F:` or `:62M:`.

If a bank sends a wrong opening balance, the running balances calculated from
the opening balance and transaction amounts do not match the closing balance.
hbci4java detects this by comparing the last transaction's balance with the
closing balance. When they differ, it walks transaction lines backwards, assigns
the current closing-derived balance to each line, and subtracts that line's
amount for the next earlier line.

## Decision

Port the closing-balance correction into `GvrKUms`.

Keep the original behavior:

- run the correction only when a block has transaction lines and an ending
  balance;
- compare the last line's balance value with `btag.end.value`;
- when they differ, walk lines from newest to oldest;
- replace only `line.saldo.value`;
- keep `line.saldo.date`, booking date, value, references, and multitag fields
  unchanged;
- use the closing balance currency for corrected line balances.

If a malformed line lacks a parsable value or balance, skip correction rather
than introducing a new parser-error surface in this slice.

## Consequences

`GvrKUms` now mirrors hbci4java's tolerance for banks that report inconsistent
opening balances while still preserving the successfully parsed transaction
details.

Remaining work:

- integrate structured `GvrKUms` into `KUmsAll`/`KUmsNew` handler results;
- port fixture-level `TestMT940Parse` expectations through the public result
  API;
- decide parser error reporting for malformed MT940 input.

## Links

- `src/gv_result/mod.rs`
- `tests/structures.rs`
- `docs/adr/0106-kums-mt940-line-parser.md`
- `docs/adr/0107-kums-mt940-multitag-mapping.md`
- Upstream: `org.kapott.hbci.GV_Result.GVRKUms#parseMT94x`
