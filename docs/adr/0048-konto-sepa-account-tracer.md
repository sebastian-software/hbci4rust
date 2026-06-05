# ADR 0048: Konto SEPA Account Tracer

## Status

Accepted

## Context

hbci4java's `Konto` exposes `isSEPAAccount()` as a small helper used to decide
whether an account can be used for SEPA jobs.

The upstream implementation is deliberately shallow: it returns true when both
`bic` and `iban` are non-null and non-empty. It does not validate the IBAN
checksum, BIC syntax, country, account-number CRC, or bank metadata.

The Rust port already carries `Konto.bic` and `Konto.iban`, including UPD import
and passport fallback enrichment.

## Decision

Add `Konto::is_sepa_account()` as the Rust-cased port of hbci4java
`Konto.isSEPAAccount()`.

Keep the behavior original-near:

- return true only when BIC is present and non-empty;
- return true only when IBAN is present and non-empty;
- do not run IBAN checksum validation;
- do not run BIC validation;
- do not consult BPD/UPD job metadata.

## Consequences

SEPA-capability checks can now use the same cheap account predicate hbci4java
exposes.

The helper is intentionally not a correctness validator. An account with a
malformed but non-empty IBAN and BIC still returns true, matching the upstream
method.

Remaining work:

- port `Konto.checkIBAN()` and `HBCIUtils.checkIBANCRC(...)` as a separate
  tracer;
- port `Konto.checkCRC()` only after deciding how much of the bank-code and
  account-CRC algorithm table belongs in v1;
- decide where SEPA job renderers should call `is_sepa_account()` once SEPA jobs
  enter the PinTAN scope.

## Links

- `src/gv_result/mod.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.structures.Konto#isSEPAAccount`
- Upstream: `org.kapott.hbci.structures.Konto#checkIBAN`
- Upstream: `org.kapott.hbci.structures.Konto#checkCRC`
