# ADR 0054: Bank Info Search Tracer

## Status

Accepted

## Context

hbci4java exposes `HBCIUtils.searchBankInfo(String)` as a lookup helper over
the loaded BLZ table.

The original behavior:

- trims the query when it is not null;
- returns an empty list for null or fewer than three query characters;
- compares case-insensitively for BIC prefix, bank name substring, and location
  substring;
- compares BLZ with prefix matching;
- returns each matching `BankInfo` once;
- sorts the result list by BLZ.

The Rust port now has an explicit `BankInfoRegistry`, so search can be added
without introducing hbci4java's global `HBCIUtilsInternal.banks` singleton.

## Decision

Add `BankInfoRegistry::search_bank_info(&self, query: &str) -> Vec<&BankInfo>`.

Keep the Rust method non-nullable because Rust callers can represent missing
input before calling the API. Preserve the original empty-result behavior for
trimmed queries shorter than three characters.

Match the original search predicates:

- `blz.starts_with(query)`;
- `bic.to_lowercase().starts_with(query)`;
- `name.to_lowercase().contains(query)`;
- `location.to_lowercase().contains(query)`.

Use the registry's `BTreeMap` iteration order as the BLZ sort order.

Do not add fuzzy search, Unicode normalization, richer bank metadata indexing,
or a global `searchBankInfo(...)` equivalent in this slice.

## Consequences

The BLZ subsystem now covers the useful read-only lookup trio:

- `get_bank_info`;
- `name_for_blz`;
- `search_bank_info`.

Offline fixtures can exercise bank search behavior without the full upstream
`blz.properties` resource.

Remaining work:

- decide whether exact Java locale-dependent `String.toLowerCase()` behavior is
  needed for full resource parity;
- decide whether to expose a process-wide bank-info registry later;
- decide whether and how the full upstream BLZ resource enters the crate.

## Links

- `src/manager/bank_info.rs`
- `tests/bank_info.rs`
- Upstream: `org.kapott.hbci.manager.HBCIUtils#searchBankInfo`
