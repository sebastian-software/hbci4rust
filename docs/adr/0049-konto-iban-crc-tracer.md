# ADR 0049: Konto IBAN CRC Tracer

## Status

Accepted

## Context

hbci4java exposes `Konto.checkIBAN()`, which delegates to
`HBCIUtils.checkIBANCRC(...)` and then to `AccountCRCAlgs.checkIBAN(...)`.

The upstream algorithm:

- moves the first four IBAN characters to the end;
- expands digits unchanged;
- expands uppercase letters `A` through `Z` to `10` through `35`;
- computes the numeric value modulo 97;
- returns true when the remainder is 1.

The Java implementation uses `BigInteger` over the expanded decimal string. It
does not normalize lowercase letters or whitespace before checking. For short,
null, or otherwise malformed input, the upstream path can throw instead of
returning a clean boolean.

## Decision

Add `Konto::check_iban()` as the Rust-cased port of hbci4java
`Konto.checkIBAN()`.

Use a private streaming Mod-97 implementation instead of materializing a large
decimal string or adding a big-integer dependency.

Keep the original input boundary:

- accept digits and uppercase ASCII letters;
- do not uppercase lowercase input;
- do not strip spaces;
- do not validate country-specific IBAN lengths in this tracer.

Return `false` for missing, too-short, non-ASCII, lowercase, or otherwise
malformed input instead of panicking. This is a small Rust safety divergence from
the pinned Java implementation, not a change to the checksum rule.

## Consequences

`Konto` now exposes both SEPA capability (`is_sepa_account`) and IBAN checksum
checking (`check_iban`) as separate helpers, matching hbci4java's conceptual
split.

The check is useful for offline validation and later job parameter checks
without requiring the broader German account-number CRC algorithm table.

Remaining work:

- decide whether to expose an `HBCIUtils`-style public utility facade later;
- port the job-level callback flow that asks users to correct invalid IBANs;
- add country-specific length validation only if upstream parity or job behavior
  requires it;
- port `Konto.checkCRC()` separately once the BLZ/account-CRC data strategy is
  decided.

## Links

- `src/gv_result/mod.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.structures.Konto#checkIBAN`
- Upstream: `org.kapott.hbci.manager.HBCIUtils#checkIBANCRC`
- Upstream: `org.kapott.hbci.manager.AccountCRCAlgs#checkIBAN`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#_checkIBANCRC`
