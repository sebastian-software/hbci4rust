# 0235 Add Java To Rust Mapping Notes

## Status

Accepted

## Context

The port is intentionally original-near, but the public Rust API already uses
Rust-cased type names and async functions. New contributors and future port
slices need a single reference that explains how common hbci4java concepts map
to the current Rust crate without requiring them to infer that mapping from
tests or ADR history.

The mapping is especially important where the port deliberately keeps Java
surface data unchanged:

- job names such as `SaldoReq` and `CustomMsg`;
- parameter/property keys such as `my.iban` and `src.number`;
- lowlevel segment names such as `Saldo7` or `CustomMsg5`;
- result content keys copied from original response paths.

## Decision

Add Java-to-Rust mapping notes under `docs/reference/`.

The notes should be descriptive documentation, not a new API contract:

- list the major Java classes and their current Rust equivalents;
- explain the handler/job/passport/callback/communication/result flow;
- show the original-near rule for job names and property keys;
- point to the job and result coverage audits for exhaustive surface tracking;
- record out-of-v1 boundaries such as `GVTemplate`, chipcard, and key-file live
  support.

Do not rename public Rust types or add wrapper aliases in this slice.

## Consequences

Future contributors have a quick orientation guide before reading the detailed
ADRs or tests. The crate remains original-near without freezing the current
Rust names as a compatibility guarantee beyond the normal public API.

If later rustification work changes public names or introduces typed builders,
update the mapping notes together with that ADR.

## References

- `docs/architecture/job-coverage.md`
- `docs/architecture/result-coverage.md`
- `src/lib.rs`
- `src/manager/handler.rs`
- `src/gv/mod.rs`
