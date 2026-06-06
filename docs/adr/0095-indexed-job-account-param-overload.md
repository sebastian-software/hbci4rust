# ADR 0095: Indexed Job Account Parameter Overload

## Status

Accepted

## Context

hbci4java's `HBCIJobImpl#setParam(String,Integer,Konto)` maps a structured
account object to indexed frontend parameters:

- `<base>.country`;
- `<base>.blz`;
- `<base>.number`;
- `<base>.subnumber`;
- `<base>.name`;
- `<base>.curr`;
- `<base>.bic`;
- `<base>.iban`.

For each derived name, Java checks `acceptsParam(...)` and skips unknown or
empty fields. Accepted fields are delegated to the indexed string setter, where
non-indexed constraints fail.

The Rust port already has:

- `HbciJob::set_param_account(...)`;
- `HbciJob::try_set_indexed_param(...)`;
- `HbciJob::try_set_indexed_param_value(...)`;
- `Konto`.

ADR 0087 left indexed account helpers as remaining work.

## Decision

Add `HbciJob::try_set_indexed_param_account(base, index, account)`.

The helper:

- derives the same frontend field names as hbci4java;
- ignores fields that are not accepted by the job;
- ignores `None` and empty account fields;
- delegates accepted non-empty fields to `try_set_indexed_param(...)`;
- returns the same indexed-parameter errors as the string setter when an
  accepted field is not indexed.

Do not store plain frontend map entries for indexed account values. Indexed
parameters are represented as indexed low-level paths, matching ADR 0087 and
ADR 0094.

## Consequences

Repeated account groups can now be populated from Rust `Konto` structures once
jobs with indexed account constraints enter the port.

Synthetic tests pin successful account-field mapping, ignored optional fields,
and non-indexed rejection.

Remaining work:

- port indexed date helpers;
- port `verifyConstraints()` indexed lookup for index `0`;
- apply indexed account parameters to concrete batch or repeated-account jobs
  once their constraints are ported.

## Links

- `src/gv/mod.rs`
- `tests/bootstrap.rs`
- `docs/adr/0079-job-account-param-overload-tracer.md`
- `docs/adr/0087-indexed-job-param-setter-tracer.md`
- `docs/adr/0094-indexed-job-value-param-overload.md`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Integer,Konto)`
- Upstream: `org.kapott.hbci.GV.HBCIJobImpl#setParam(String,Integer,String)`
