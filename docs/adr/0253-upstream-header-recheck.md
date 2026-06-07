# ADR 0253: Upstream Header Recheck

## Status

Accepted

## Context

ADR 0002 records the project-level licensing assumption: hbci4java is treated as
LGPL 2.1 at repository level, and `hbci4rust` is licensed as
`LGPL-2.1-or-later` because this is a direct, original-near port.

The v1 release checklist still required a recheck of upstream header
inconsistencies before publishing. The current local upstream reference is the
pinned hbci4java tag `hbci4j-core-4.1.11` at
`3b7ce667c73724daa1c836ed7333ed090c21a831`.

The recheck found:

- upstream `readme.md` declares LGPL 2.1 and notes the pre-2016 GPLv2 history;
- upstream root `LICENSE` and `src/main/resources/COPYING` contain the LGPL 2.1
  license text;
- 325 of 399 checked core Java files under
  `src/main/java/org/kapott/hbci` contain the classic
  "GNU Lesser General Public" header text;
- 74 checked core Java files do not contain that exact header text, including
  several newer SEPA parser/generator classes and some v1-irrelevant classes;
- copied protocol XML/DTD resources and copied offline fixtures do not carry
  individual license headers, but they come from the same pinned repository and
  are attributed through `NOTICE` and directory README files.

## Decision

Keep ADR 0002's project-level LGPL treatment for v1:

- `hbci4rust` remains `LGPL-2.1-or-later`;
- attribution remains in `NOTICE`, `docs/adr/0002-license-and-attribution.md`,
  `resources/protocol/README.md`, and `tests/fixtures/hbci4java/README.md`;
- copied protocol resources and fixtures remain in the crate because they are
  required for original-near offline behavior and are explicitly attributed;
- the header recheck is documented in `docs/reference/upstream-header-review.md`.

Do not add copied upstream headers into individual Rust source files unless a
future slice copies a substantial source body directly. Prefer file-level
comments only when they clarify a specific translation source or fixture origin.

## Consequences

The release checklist can treat the upstream header inconsistency recheck as
covered for the current pinned baseline.

This is not legal advice and does not remove the need for a final publication
review. If the upstream baseline changes, copied upstream artifacts change, or
additional generated/copied upstream material is added, rerun the header review
and record a new ADR.

## Links

- `docs/reference/upstream-header-review.md`
- `docs/reference/packaging.md`
- `docs/architecture/release-checklist.md`
- `NOTICE`
- `LICENSE`
- `resources/protocol/README.md`
- `tests/fixtures/hbci4java/README.md`
- ADR 0002: License And Attribution
- ADR 0249: Crate Packaging Metadata
