# ADR 0079: Job Account Param Overload Tracer

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl` has overloads for complex parameter types. The account
overload:

- receives a frontend base name such as `my`;
- checks each possible field via `acceptsParam(...)`;
- sets only accepted and non-empty fields;
- maps account fields to ordinary frontend parameter names such as
  `my.country`, `my.blz`, `my.number`, `my.bic`, and `my.iban`.

ADR 0078 added non-validating job constraint metadata for the currently ported
Saldo jobs, which gives the Rust port enough information to know whether a
frontend account field is accepted.

## Decision

Add `HbciJob::set_param_account(base, account)`.

The helper:

- checks `HbciJob::accepts_param(...)` before setting each field;
- ignores `None` and empty account fields;
- sets the same frontend parameter names that callers can already set manually
  via `set_param(...)`.

Keep this additive and non-validating. Existing direct string parameter setting
remains unchanged.

Do not port the `Value`, `Date`, integer, or indexed overloads in this slice.
Those need positive tests against jobs that actually expose matching
constraints.

## Consequences

Saldo jobs can now be parameterized with a `Konto` in the same broad shape as
hbci4java's `setParam("my", Konto)`.

`SaldoReq` accepts known account fields and ignores unconstrained fields such as
`my.name` and `my.curr` in the current Saldo7 metadata.

`SaldoReqAll` ignores account fields because it does not expose account
constraints.

Remaining work:

- port `setParam(String, Value)` once a value-parameter job is rendered;
- port date, integer, and indexed parameter overloads;
- add full `verifyConstraints()` semantics once low-level parameter generation
  is in place.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0078-job-constraint-metadata-tracer.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Konto)`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#acceptsParam`
