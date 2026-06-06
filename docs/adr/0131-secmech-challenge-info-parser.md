# ADR 0131: Secmech Challenge Info Parser

## Status

Accepted

## Context

hbci4java uses `ChallengeInfo` to load `challengedata.xml`, map business
transaction segment codes to HHD challenge classes, and select challenge
parameters for TAN/SCA requests.

The data is used by PinTAN/HBCI-Plus flows even though v1 excludes chipcard,
PCSC, CTAPI, DDV, RDH/RAH/RSA key-file live support, and Java passport import.
The deterministic upstream tests cover parsing and parameter formatting before
the values are written into an `HKTAN` segment.

The relevant upstream behavior is:

- unknown job codes return no challenge data;
- known jobs may have a challenge class but no parameters;
- HHD 1.2, 1.3, and 1.4 have separate challenge classes and parameter lists;
- `Wrt` values are formatted through hbci4java's syntax layer, for example
  `100` becomes `100,` and `100.50` becomes `100,5`;
- blank parameter types do not apply `AN` escaping because escaping happens
  later when the HBCI segment is rendered;
- `Date` values use compact `YYYYMMDD` rendering and invalid dates fail;
- optional parameter conditions compare security-mechanism properties such as
  `needchallengevalue`.

## Decision

Add a Rust `ChallengeInfo` parser in `manager::secmech`.

Keep the parser original-near:

- parse the upstream `challengedata.xml` shape with `quick-xml`;
- preserve job codes, `challengeinfo spec` names, classes, parameter order,
  paths, parameter types, and optional conditions exactly;
- expose `ChallengeJob`, `ChallengeHhdVersion`, and `ChallengeParam` so tests
  and later runtime integration can mirror hbci4java's nested object model;
- use `HhdVersion::challenge_version()` to choose the XML spec;
- return `HbciErrorKind::Protocol` for malformed XML and
  `HbciErrorKind::InvalidArgument` for invalid formatted values.

Copy the pinned upstream `challengedata.xml` into
`tests/fixtures/hbci4java/secmech/challengedata.xml` for offline parity tests.
The production resource-loading decision remains separate from this parser
slice because the v1 port still needs the surrounding `HKTAN` runtime path.

Do not port `ChallengeInfo.applyParams(...)` yet. That method writes the parsed
data into Java `HBCIJob`/`HKTAN` objects and is best ported together with the
PinTAN dialog and HKTAN message integration.

## Consequences

The Rust port can now verify the deterministic `ChallengeInfoTest` parsing,
class, condition, and formatting behavior against the original XML fixture.

Later PinTAN work can consume the parsed model without reparsing the XML shape
or rediscovering the challenge-parameter rules.

Remaining work:

- decide whether `challengedata.xml` is checked in as a production resource or
  generated/copied during a resource-sync step;
- integrate challenge class and challenge parameters into HKTAN request
  generation;
- port the upstream DEG/HKTAN position-preservation test once HKTAN rendering
  is in scope.

## Links

- `src/manager/secmech.rs`
- `tests/secmech.rs`
- `tests/fixtures/hbci4java/secmech/challengedata.xml`
- Upstream: `org.kapott.hbci.manager.ChallengeInfo`
- Upstream: `org.kapott.hbci4java.secmech.ChallengeInfoTest`
