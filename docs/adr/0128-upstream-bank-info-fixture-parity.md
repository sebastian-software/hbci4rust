# ADR 0128: Upstream Bank Info Fixture Parity

## Status

Accepted

## Context

hbci4java's `TestBankInfo` exercises `HBCIUtils.getBankInfo(...)` and
`HBCIUtils.searchBankInfo(...)` against the full upstream `blz.properties`
resource. The exact fixture row used by that test is:

`86050200=Sparkasse Muldental|Grimma|SOLADES1GRM|20|i052.s-fints-sn.de|https://banking-sn5.s-fints-pt-sn.de/fints30|220|300|`

The Rust port already has `BankInfo`, `BankInfoRegistry`, and search behavior,
but its tests only use synthetic bank rows.

ADR 0053 deliberately avoided vendoring the complete upstream BLZ table until
the resource strategy is decided. That decision still stands.

## Decision

Add a narrow upstream-derived `bank_info` fixture for the rows needed to mirror
the observable parts of `TestBankInfo`:

- direct lookup of BLZ `86050200`;
- BLZ prefix search for `86050`;
- BIC prefix search for `SOLADES`;
- location substring search for `Grim`;
- name substring search for `Muldent`;
- short-query empty result for `12`.

Do not copy the full `blz.properties` resource in this slice. The test will not
preserve hbci4java's full-table result-count assertions such as `>= 100` for
`SOLADES`; it instead pins the exact relevant rows and sorted BLZ order within
the small fixture.

Keep `BankInfoRegistry::search_bank_info(...)` non-nullable as decided in ADR
0054. Java's null-query case remains represented by callers choosing not to
call the method.

## Consequences

The bank-info tests now exercise real upstream BLZ data for the canonical
`TestBankInfo` bank without changing the crate's production resource policy.

Full-resource parity remains open and should be addressed together with the
larger BLZ/account-number CRC strategy.

## Links

- `tests/fixtures/hbci4java/bank_info/test-bank-info.properties`
- `tests/bank_info.rs`
- `docs/adr/0053-bank-info-registry-tracer.md`
- `docs/adr/0054-bank-info-search-tracer.md`
- Upstream: `org.kapott.hbci4java.manager.TestBankInfo`
- Upstream resource: `src/main/resources/blz.properties`
