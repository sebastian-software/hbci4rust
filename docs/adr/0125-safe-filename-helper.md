# ADR 0125: Safe Filename Helper

## Status

Accepted

## Context

The hbci4java offline test `TestIOUtils#testSafeFilename` covers
`org.kapott.hbci.tools.IOUtils.safeFilename(...)`. The helper protects file
names by removing characters outside `[a-zA-Z0-9_.-]` and truncating the file
name component to 25 characters.

The original method accepts Java `null`, returns unchanged `null` or empty
strings, works on the absolute file path, and only sanitizes the last path
component.

## Decision

Add `tools::safe_filename(...)` beside the existing string utility helpers.
Represent Java `null` as `Option<&str>`/`Option<String>`.

Keep the implementation original-near:

- `None` returns `None`;
- an empty string returns an empty string;
- only ASCII letters, digits, `_`, `.`, and `-` remain in the file name;
- the sanitized file name is truncated to 25 characters;
- relative inputs are resolved against the current working directory before
  returning, matching Java `File(...).getAbsoluteFile()` behavior.

Add Rust tests for the three upstream assertions and small tracer tests for
`None`, empty input, and parent-directory preservation.

## Consequences

The Rust port gains another small Phase 1 utility helper covered by upstream
offline behavior. Future storage or file-writing slices can use this helper
instead of reimplementing filename cleanup locally.
