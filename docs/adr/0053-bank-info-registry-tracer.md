# ADR 0053: Bank Info Registry Tracer

## Status

Accepted

## Context

hbci4java loads `blz.properties` through
`HBCIUtils.refreshBLZList(InputStream)`. That method reads Java `Properties`,
parses every value with `BankInfo.parse(...)`, sets the BLZ from the property
key, and stores the result in the process-wide
`HBCIUtilsInternal.banks` map.

`HBCIUtils.getBankInfo(String)` returns a bank entry from that map.
`HBCIUtils.getNameForBLZ(String)` returns the bank name, or an empty string
when the BLZ is unknown or the stored name is null.

The Rust port already has the `BankInfo` value parser. It still needs a lookup
surface for offline fixtures and later BLZ resource loading.

## Decision

Add `BankInfoRegistry` under the `manager` module.

Expose:

- `BankInfoRegistry::parse_properties(...)`;
- `BankInfoRegistry::get_bank_info(...)`;
- `BankInfoRegistry::name_for_blz(...)`;
- `BankInfoRegistry::len(...)`;
- `BankInfoRegistry::is_empty(...)`.

Keep the registry explicit and owned by the caller. Do not introduce a global
mutable bank-info map in this slice.

Parse simple Properties-like fixture text line by line:

- skip blank lines;
- skip lines starting with `#` or `!`;
- split at the first `=` or `:`;
- trim leading whitespace before a line;
- trim trailing whitespace after the key;
- trim leading whitespace before the value;
- treat a line without a separator as a key with an empty value.

This is intentionally not a full Java `Properties` parser. Escapes,
continuations, Unicode escapes, and whitespace-only separators are deferred
until we either vendor the original `blz.properties` resource or need exact
Java loader parity.

Do not vendor the full upstream `blz.properties` file in this slice. Do not
port `searchBankInfo(...)` yet. Do not wire `Konto::Display` to bank lookup
yet, because that needs an explicit formatting context or a deliberate global
lookup decision.

## Consequences

Offline tests can now build a small BLZ lookup table without vendoring the full
upstream resource.

The public Rust API mirrors the useful `HBCIUtils.getBankInfo(...)` and
`getNameForBLZ(...)` behavior while avoiding a process-wide singleton for now.

Remaining work:

- decide whether the production BLZ table is vendored, generated, or fetched;
- decide whether exact Java `Properties` semantics are required for that table;
- add `searchBankInfo(...)` parity if caller-facing bank search becomes
  relevant;
- decide how `Konto::Display` should access bank-name lookup data.

## Links

- `src/manager/bank_info.rs`
- `src/manager/mod.rs`
- `src/lib.rs`
- `tests/bank_info.rs`
- Upstream: `org.kapott.hbci.manager.HBCIUtils#refreshBLZList`
- Upstream: `org.kapott.hbci.manager.HBCIUtils#getBankInfo`
- Upstream: `org.kapott.hbci.manager.HBCIUtils#getNameForBLZ`
- Upstream resource: `src/main/resources/blz.properties`
