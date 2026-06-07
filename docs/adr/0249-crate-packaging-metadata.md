# ADR 0249: Crate Packaging Metadata

## Status

Accepted

## Context

ADR 0246 added a v1 release checklist with open items for crate publication
metadata, package contents, and copied upstream artifact attribution.

The crate is a direct, original-near port of hbci4java, but it is not published
from the hbci4java repository. The local Git remote for this Rust port is
`https://github.com/sebastian-software/hbci4rust.git`, while the upstream Java
baseline remains `https://github.com/hbci4j/hbci4java.git` at tag
`hbci4j-core-4.1.11` and commit
`3b7ce667c73724daa1c836ed7333ed090c21a831`.

`Cargo.toml` currently has two different metadata needs:

- crate publication metadata should identify the Rust crate and its repository;
- upstream metadata should preserve the Java baseline for auditability and
  attribution.

## Decision

Use the Rust port repository as the package-level Cargo `repository` value:
`https://github.com/sebastian-software/hbci4rust`.

Keep the hbci4java repository, tag, and commit in
`[package.metadata.hbci4rust.upstream]`, `NOTICE`, resource README files, fixture
README files, and ADR links. Do not use the package-level Cargo `repository`
field for upstream attribution.

Add a packaging reference note that records:

- the reviewed Cargo package metadata;
- the reviewed `cargo package --list` content groups;
- where copied upstream protocol resources and test fixtures are attributed;
- the remaining release blocker to recheck upstream header inconsistencies
  before publishing.

## Consequences

Crate consumers and package indexes will be pointed at the Rust port repository
instead of the Java source repository, while the direct-port baseline remains
visible and machine-readable.

The release checklist can mark package metadata, package listing review, and
copied artifact attribution as covered for the current v1 hardening slice.
Final release candidates still have to rerun packaging checks and re-evaluate
upstream header inconsistencies before publication.

## Links

- `Cargo.toml`
- `LICENSE`
- `NOTICE`
- `docs/architecture/release-checklist.md`
- `docs/reference/packaging.md`
- `resources/protocol/README.md`
- `tests/fixtures/hbci4java/README.md`
- ADR 0002: License And Attribution
- ADR 0246: V1 Release Checklist
