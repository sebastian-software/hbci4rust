# ADR 0126: Parameter Finder Helpers

## Status

Accepted

## Context

hbci4java uses `org.kapott.hbci.tools.ParameterFinder` to query BPD/UPD
property trees with dotted paths and simple `*` wildcards. The offline test
class `TestParameterFinder` covers the behavior most relevant to PinTAN:

- recursive matching with keys collapsed to the remaining suffix;
- full-path matching that preserves original property keys;
- predefined PinTAN query constants;
- parameterized query formatting;
- rejecting a query whose parameters were not set.

The Rust port currently has BPD/UPD parsing and PinTAN metadata caches, but no
shared equivalent for this original helper.

## Decision

Add `tools::ParameterFinder` and `tools::ParameterQuery`.

Represent Java `Properties` as `BTreeMap<String, String>` for deterministic
tests while keeping the same observable key/value semantics. Provide:

- `ParameterFinder::find(...)`;
- `ParameterFinder::find_query(...)`;
- `ParameterFinder::find_all(...)`;
- `ParameterFinder::find_all_query(...)`;
- `ParameterFinder::get_value(...)`;
- `ParameterFinder::get_value_query(...)`.

Represent hbci4java query constants as associated constants on
`ParameterQuery`, and provide `with_parameters(...)` for the original
`MessageFormat`-style `{0}` replacement needed by order hash mode lookup.

When a query requiring parameters is used before `with_parameters(...)`, return
`HbciErrorKind::InvalidArgument`.

## Consequences

PinTAN and BPD/UPD code can reuse an original-near parameter lookup helper
instead of hard-coding wildcard scans. The first implementation intentionally
matches only the wildcard behavior exercised by hbci4java: one `*` fragment per
path segment with starts-with, ends-with, contains, or exact matching.
