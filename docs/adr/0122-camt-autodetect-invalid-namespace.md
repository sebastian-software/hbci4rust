# ADR 0122: CAMT Autodetect Invalid Namespace Boundary

## Status

Accepted

## Context

The hbci4java CAMT parser tests `TestCamtParse#test002` and
`TestCamtParse#test003` cover the namespace boundary of
`SepaVersion.autodetect(InputStream)` before any CAMT parsing happens.

The original behavior distinguishes two cases:

- a `Document` without a namespace returns `null`;
- a `Document` with a syntactically invalid SEPA namespace throws
  `IllegalArgumentException`.

The Rust port already returns `None` for documents without a namespace, but the
current CAMT autodetection treats an unknown namespace as `None` too. That is
too permissive for the invalid upstream fixture
`test-camt-parse-invalid.xml`.

## Decision

Copy the two upstream fixtures
`test-camt-parse-none.xml` and `test-camt-parse-invalid.xml` into
`tests/fixtures/hbci4java/sepa/` and add Rust tests mirroring
`TestCamtParse#test002` and `TestCamtParse#test003`.

Keep `SepaVersion::autodetect` returning `Ok(None)` when the root element has
no namespace. When a namespace is present but does not resolve to one of the
currently known CAMT versions, return `HbciErrorKind::InvalidArgument`.

Do not introduce dynamic unknown SEPA versions in this slice. hbci4java can
construct synthetic `SepaVersion` values for valid but unknown URNs, but the
current Rust public type stores static version metadata. Dynamic unknown
versions require a separate public API decision.

## Consequences

CAMT autodetection now matches the observable upstream parser tests for
namespace-less and invalid-namespace documents. The port remains deliberately
known-version based until a later ADR decides whether and how to represent
unknown but syntactically valid SEPA versions.
