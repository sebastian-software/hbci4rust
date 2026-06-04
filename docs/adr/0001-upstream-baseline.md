# ADR 0001: Upstream Baseline

## Status

Accepted

## Context

The Rust port needs a reproducible hbci4java reference point. The chosen upstream
tag is `hbci4j-core-4.1.11`, resolving to commit
`3b7ce667c73724daa1c836ed7333ed090c21a831`.

The tag contains a noteworthy mismatch: `gradle.properties` still reports
`version = 4.1.10`, while the tag name is `hbci4j-core-4.1.11`.

## Decision

Use tag `hbci4j-core-4.1.11` and commit
`3b7ce667c73724daa1c836ed7333ed090c21a831` as the v1 porting baseline.

Record the `gradle.properties` mismatch as release metadata, not as a reason to
switch to `master`.

## Consequences

The port has a stable source baseline, and future upstream deltas can be reviewed
separately. Any generated fixtures must record the upstream commit they came
from.

## Links

- Upstream repository: https://github.com/hbci4j/hbci4java
- Fetch script: `scripts/fetch-upstream.sh`
