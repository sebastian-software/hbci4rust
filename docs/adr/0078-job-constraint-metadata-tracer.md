# ADR 0078: Job Constraint Metadata Tracer

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl` uses `addConstraint(...)` to map high-level frontend
parameter names to low-level protocol paths and optional default values. For
example, `GVSaldoReq` registers constraints such as:

- `my.iban` -> `Saldo7.KTV.iban`;
- `my.country` -> `Saldo7.KTV.KIK.country` with default `DE`;
- `dummyall` -> `Saldo7.allaccounts` with default `N`.

The Java implementation also uses constraints for validation, log filtering,
indexed parameters, low-level parameter generation, and segment validation.

The current Rust `HbciJob` only stores the Java-facing string parameters passed
through `set_param(...)`. Renderer code reads those frontend parameter names
directly.

## Decision

Add `HbciJobConstraint` as public metadata with:

- `frontend_name`;
- `destination_name`;
- `default_value`;
- `indexed`.

Add `HbciJob::constraints()` and `HbciJob::constraint(...)`.

Populate original-near metadata for the currently rendered Saldo jobs:

- `SaldoReq`;
- `SaldoReqAll`.

Keep the metadata non-validating for now. `set_param(...)` continues to store
frontend parameter names directly, and renderers continue to consume them.

Use destination names including the current low-level segment name
(`Saldo7...`) because the Rust renderer currently targets the Saldo7 syntax
directly.

## Consequences

The Rust port now exposes a first visible equivalent of hbci4java's
constraint table for rendered Saldo jobs.

Existing pragmatic job parameter behavior remains unchanged.

This gives future port slices a stable place to add validation, indexed
parameter behavior, log filtering, default application, and low-level parameter
generation without changing public job names.

Remaining work:

- derive constraints from the protocol/job registry instead of hardcoding
  rendered Saldo jobs;
- port validation semantics from `HBCIJobImpl.verifyConstraints()`;
- port indexed constraints and multi-destination constraints;
- decide how log-filter metadata maps into Rust callback/logging APIs.

## Links

- `src/gv/mod.rs`
- `src/lib.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#addConstraint`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam`
- Upstream: `org.kapott.hbci.GV.GVSaldoReq`
