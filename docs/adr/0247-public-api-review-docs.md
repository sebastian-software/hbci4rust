# ADR 0247: Public API Review Docs

## Status

Accepted

## Context

ADR 0246 added a v1 release checklist. Its public API section requires an API
docs pass for exported v1 types and at least one migration example checked
against the current public API.

The crate currently re-exports the broad original-near surface from
`src/lib.rs`. That is useful for Java-near migration because applications can
import `HbciHandler`, `HbciJob`, `PinTanPassport`, callback types, protocol
helpers, and typed results from the crate root. The cost is that the API is
large enough that a reader needs an explicit map of which exports are primary,
which are support types, and which are replay/test or low-level helpers.

## Decision

Keep the crate-root re-export surface for v1 and document it instead of hiding
or reshaping it.

Add `docs/reference/public-api.md` as the review artifact for crate-root
exports. The document must:

- group exports by user-facing role;
- name the primary v1 PinTAN path;
- preserve the Java-near rule for job names and parameter keys;
- call out replay/test support and lower-level protocol helpers separately;
- point back to the Java migration guide for workflow examples.

Add a small `tests/public_api.rs` integration test that imports only public
crate-root APIs and exercises the common balance-request migration shape. This
keeps the documented example honest without adding another broad replay fixture.

## Consequences

The v1 API remains original-near and broad, matching the porting plan.

Release review gets a concrete artifact for exported types without requiring a
premature idiomatic Rust facade.

Future API removals or facade reshaping should be tracked as rustification work
after v1 parity, not folded into this release-hardening slice.

## Links

- `src/lib.rs`
- `docs/reference/java-to-rust-mapping.md`
- `docs/architecture/release-checklist.md`
- ADR 0003: V1 PinTAN Scope
- ADR 0009: Rustification Backlog
- ADR 0246: V1 Release Checklist
