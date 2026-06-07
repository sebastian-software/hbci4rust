# ADR 0259: Release Candidate Check Runner

## Status

Accepted

## Context

The v1 release checklist requires final offline gates and package checks to be
rerun after the last release-candidate commit.

During porting, these commands are run frequently, but the final checklist
cannot honestly be closed until the command set is executed on the actual
release-candidate commit. Running the commands manually also makes it easy to
miss a gate or lose the exact output needed for review.

## Decision

Add a local release-candidate check runner under `scripts/`.

The runner must:

- execute the required offline gate commands from
  `docs/architecture/release-checklist.md`;
- write full per-command logs under `target/release-gates/`;
- print concise pass/fail summaries to the terminal;
- keep package checks explicit behind a `--package` flag, because package
  verification is a final release-candidate action rather than an everyday
  development gate;
- stay local and source-controlled, without adding a library dependency or a
  service-owned release process.

Do not mark final release-candidate gate items as checked merely because this
runner exists. Those checklist items are checked only after the runner, and any
required package checks, pass on the actual release-candidate commit.

## Consequences

The final v1 release pass becomes easier to reproduce and review.

Porting work can continue to run the same gate set before final release without
pretending that the final release-candidate evidence has already been captured.

If the release checklist command set changes, update the runner and record a new
ADR only when the release acceptance semantics change.

## Links

- `scripts/run-release-candidate-checks.sh`
- `docs/architecture/release-checklist.md`
- `docs/reference/packaging.md`
- ADR 0246: V1 Release Checklist
