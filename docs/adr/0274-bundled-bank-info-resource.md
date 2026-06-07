# ADR 0274: Bundled Bank Info Resource

## Status

Accepted

## Context

ADR 0052 and ADR 0053 introduced `BankInfo` and `BankInfoRegistry` but
deliberately avoided shipping the full hbci4java `blz.properties` table.
ADR 0128 added only a narrow upstream-derived fixture and left the production
resource decision open.

The crate now needs a setup/search surface comparable to the bank lists used by
Hibiscus/Jameica-style FinTS tools. The Willuhn wiki lists are useful as user
documentation, but their wiki license is not a clean source for copying into
this LGPL crate. The pinned hbci4java baseline already ships the technical
`src/main/resources/blz.properties` table consumed by the upstream parser.

## Decision

Vendor the pinned hbci4java `blz.properties` file at
`resources/bank_info/blz.properties`, preserving field content while
normalizing line endings to LF so the repository whitespace checks stay clean.

This closes the deferred resource decision from ADR 0052, ADR 0053, and ADR
0128 for the current pinned baseline.

Expose a lazy bundled registry through `BankInfoRegistry::bundled()`. Keep the
registry immutable and caller-facing, not a mutable process-wide
`HBCIUtilsInternal.banks` port.

Add iteration and PinTAN convenience helpers:

- `BankInfoRegistry::banks()`;
- `BankInfoRegistry::pin_tan_banks()`;
- `BankInfoRegistry::search_pin_tan_banks(...)`;
- `BankInfo::supports_pin_tan()`;
- `BankInfo::supports_rdh()`.

Treat PinTAN/RDH support as present when the corresponding address has
non-whitespace text or the corresponding HBCI version column is present.

## Consequences

Callers can now build a bank setup/search UI from the crate itself:

- search by BLZ, BIC prefix, name, or location;
- show only entries with a PinTAN endpoint;
- read endpoint URLs and FinTS versions from the same format hbci4java uses.

The bundled table is a pinned snapshot, not a live guarantee that a bank still
supports a specific FinTS URL or access method. Future maintenance should
refresh it by updating the pinned upstream baseline or deliberately copying a
new upstream `blz.properties` snapshot with matching attribution updates.

## Links

- `resources/bank_info/blz.properties`
- `resources/bank_info/README.md`
- `src/manager/bank_info.rs`
- `tests/bank_info.rs`
- `docs/adr/0052-bank-info-parser-tracer.md`
- `docs/adr/0053-bank-info-registry-tracer.md`
- `docs/adr/0054-bank-info-search-tracer.md`
- `docs/adr/0128-upstream-bank-info-fixture-parity.md`
- Upstream resource: `target/reference/hbci4java/src/main/resources/blz.properties`
