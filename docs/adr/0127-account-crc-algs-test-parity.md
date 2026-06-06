# ADR 0127: Account CRC Algs Test Parity

## Status

Accepted

## Context

hbci4java exposes `org.kapott.hbci.manager.AccountCRCAlgs` as the central
checksum helper for IBANs, creditor identifiers, and German account-number
checksum algorithms.

The current Rust port already implements `Konto::check_iban()` with the same
streaming Mod-97 rule as `AccountCRCAlgs.checkIBAN(...)`, but the original
helper class itself is not represented yet. The upstream offline test class
`TestAccountCRCAlgs` currently covers:

- two valid German SEPA creditor identifiers;
- all invalid check digits for one German creditor identifier fixture;
- `alg_51(null, new int[] {0,0,0,2,6,7,1,0,7,1})`.

## Decision

Add `manager::AccountCrcAlgs` as the Rust-cased public equivalent of
hbci4java `AccountCRCAlgs`.

Port the behavior covered by upstream tests first:

- `AccountCrcAlgs::check_iban(...)`;
- `AccountCrcAlgs::check_creditor_id(...)`;
- `AccountCrcAlgs::alg_51(...)`.

Keep `alg_51(...)` original-near by accepting the unused BLZ argument as
`Option<&[u8; 8]>`, so `None` represents Java's `null` in the upstream test.

Keep Mod-97 calculation streaming instead of materializing a large decimal
string or adding a big-integer dependency. For malformed or too-short input,
return `false` instead of reproducing Java substring or `BigInteger` failures.
This matches the safety boundary already documented for `Konto::check_iban()`.

Update `Konto::check_iban()` to delegate to `AccountCrcAlgs::check_iban(...)`
so future account-check code can use one shared checksum helper.

## Consequences

The Rust crate now has a recognizable home for later BLZ/account-number CRC
algorithm ports without committing to the full algorithm table in this slice.

The upstream `TestAccountCRCAlgs` cases are represented directly in Rust tests.
The remaining `AccountCRCAlgs` algorithms still need to be ported when the
national account-number CRC strategy is addressed.

## Links

- `src/manager/account_crc.rs`
- `src/gv_result/mod.rs`
- `tests/account_crc.rs`
- `docs/adr/0049-konto-iban-crc-tracer.md`
- `docs/adr/0088-account-crc-callback-reasons.md`
- Upstream: `org.kapott.hbci.manager.AccountCRCAlgs`
- Upstream: `org.kapott.hbci4java.manager.TestAccountCRCAlgs`
