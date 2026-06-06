# ADR 0088: Account CRC Callback Reasons

## Status

Accepted

## Context

hbci4java's `GVSaldoReq.verifyConstraints()` calls
`HBCIJobImpl.checkAccountCRC("my")` after the base constraint verification.
That helper validates national BLZ/account-number pairs and IBANs, and lets the
application correct invalid values through callback reasons:

- `HBCICallback.HAVE_CRC_ERROR`;
- `HBCICallback.HAVE_IBAN_ERROR`.

The Rust port already has an async callback API and original-code mapping for
the PinTAN/connection/institute-message callback reasons that had entered the
ported runtime. `Konto::check_iban()` is also ported, but the job-level account
CRC callback loop is still missing.

## Decision

Add Rust-cased callback variants and original numeric mapping for the two
account-check reasons:

- `CallbackReason::HaveCrcError` -> `19`;
- `CallbackReason::HaveIbanError` -> `30`.

Keep the Java callback numbers exactly, but keep the Rust callback payload shape
unchanged: `CallbackEvent.current_value` carries the mutable value that Java
would have passed through `StringBuffer`.

Do not port national BLZ/account-number algorithm tables or the full
`checkAccountCRC(...)` correction loop in this slice. This ADR only adds the
public callback surface needed by the later async account-check tracer.

## Consequences

The v1 PinTAN API can now represent the original callback reasons needed by
`GVSaldoReq`, `GVSaldoReqAll`, and later account-bearing jobs without importing
chipcard or key-file callbacks.

Tests pin the two original callback numbers and round-trip decoding.

Remaining work:

- port the async `checkAccountCRC("my")` flow for `SaldoReq` and
  `SaldoReqAll`;
- decide how national account-number CRC tables are sourced for v1;
- wire corrected callback values back into frontend and low-level job
  parameters.

## Links

- `src/callback.rs`
- `tests/callback.rs`
- `docs/adr/0049-konto-iban-crc-tracer.md`
- `docs/adr/0080-job-constraint-verification-tracer.md`
- Upstream: `org.kapott.hbci.callback.HBCICallback`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#checkAccountCRC`
- Upstream: `org.kapott.hbci.GV.GVSaldoReq#verifyConstraints`
