# 0233 Add A Local GV Job Coverage Audit

## Status

Accepted

## Context

After porting `GVCustomMsg`, the static Rust PinTAN registry should cover every
concrete hbci4java `GV*.java` job class in v1 scope. The only remaining upstream
`GV*` class without a Rust `new_job(...)` entry is `GVTemplate`, which ADR 0232
keeps out of v1 because it is Java's dynamic lowlevel fallback rather than a
concrete bank job.

This claim should be reproducible from the pinned upstream checkout and the
current Rust registry, not maintained as a conversational note.

The upstream checkout still lives under `target/reference/hbci4java` and is not
vendored. CI therefore cannot assume it exists without adding network-dependent
setup.

## Decision

Add a local job coverage audit that compares:

- upstream files matching `GV*.java` under
  `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV`;
- Rust registry names in `PINTAN_JOB_NAMES`.

Document the current result as:

- upstream concrete-style `GV*.java` names: 68;
- Rust static job names: 67;
- missing from Rust: `Template`;
- extra Rust job names not present as upstream `GV*.java`: none.

Keep this audit as a local hardening tool and architecture note, not a mandatory
CI gate, until the reference checkout is made available in CI without network
access.

## Consequences

The port can prove that the static v1 job surface is no longer missing concrete
hbci4java job classes, while still preserving the explicit `GVTemplate` boundary.

Future job-surface changes should update the coverage note and, when relevant,
record a new ADR before widening the public lowlevel API.

## References

- `docs/adr/0232-custom-msg-job-and-template-boundary.md`
- `scripts/fetch-upstream.sh`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV`
- `src/gv/mod.rs`
