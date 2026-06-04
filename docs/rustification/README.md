# Rustification Backlog

This backlog collects ideas that should wait until original-near behavior is
covered by tests.

## Candidate

- Add typed builders over the string-property job API.
- Add an explicit runtime context API beside the global `HBCIUtils`-style API.
- Split the monolithic crate into smaller crates if compile time or ownership
  boundaries justify it.
- Replace broad error categories with more specific public error variants.
- Introduce Cargo feature flags once real optional boundaries exist.
- Revisit Java package-near module names after the first parity milestone.

## Deferred

- Java PinTAN passport import.
- RDH/RAH/RSA key-file support.
- Chipcard, PCSC, CTAPI, DDV, and native card support.

## Accepted

No rustification changes have been accepted yet.
