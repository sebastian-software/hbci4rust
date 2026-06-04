# ADR 0002: License And Attribution

## Status

Accepted

## Context

hbci4java declares LGPL 2.1 in its repository README and includes an LGPL 2.1
license file. Some individual files have older or inconsistent headers.

The desired port is intentionally direct and original-near, not a clean-room
rewrite.

## Decision

License `hbci4rust` as `LGPL-2.1-or-later` and treat the upstream repository as
LGPL 2.1 at project level.

Preserve upstream attribution in `NOTICE`, ADRs, and ported source files when
source material is copied or closely translated.

## Consequences

This matches the direct-port goal and avoids pretending the Rust crate is
independent of hbci4java. Header inconsistencies remain visible risks and should
be rechecked before publishing releases.

## Links

- `LICENSE`
- `NOTICE`
