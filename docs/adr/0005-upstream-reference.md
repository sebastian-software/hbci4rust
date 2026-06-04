# ADR 0005: Upstream Reference Handling

## Status

Accepted

## Context

The port needs regular access to the Java baseline, but vendoring the whole
repository would increase size and blur which code is Rust implementation versus
reference material.

## Decision

Do not vendor hbci4java.

Provide `scripts/fetch-upstream.sh`, which downloads the pinned upstream tag into
`target/reference/hbci4java`.

## Consequences

The repository stays focused on Rust code and generated/selected assets. Porting
work remains reproducible when network access is available.

## Links

- `scripts/fetch-upstream.sh`
