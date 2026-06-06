# ADR 0130: Secmech Flicker Code Parser Renderer

## Status

Accepted

## Context

hbci4java's `FlickerCode` parses HHDuc/challenge text into the optical
chipTAN flicker payload and renders the normalized transmission string.
`FlickerRenderer` then maps that string into repeated 5-bar bit frames and
drives a timing thread.

The v1 Rust scope excludes chipcard/passport integration, but optical TAN
helpers are still relevant to PinTAN/HBCI-Plus SCA flows because HITAN can
deliver flicker, QR, and photoTAN challenges as TAN media payloads. ADR 0129
already ported QR, Matrix, and HHD version detection.

The upstream `FlickerTest` mostly checks deterministic parsing/rendering:

- HHD 1.4, HHD 1.3, embedded CHLGUC, Sparda three-digit LDE, and explicit
  HHD-version fallback cases;
- ASC versus BCD encoding;
- Luhn and XOR checksums;
- the renderer's generated 5-bar frame sequence.

## Decision

Add `FlickerCode`, `FlickerCodeVersion`, `FlickerDataElement`,
`FlickerStartCode`, `FlickerEncoding`, and `FlickerRenderer` to
`manager::secmech`.

Keep the parser and renderer original-near:

- parse HHD 1.4 first, then HHD 1.4 with three-character LDE for the Sparda
  fallback, then HHD 1.3;
- when an explicit `HhdVersion` is supplied, map it to the internal
  `FlickerCodeVersion` first;
- preserve CHLGUC/CHLGTEXT extraction by trimming spaces and prepending `0` to
  embedded HHD 1.3 challenge text;
- preserve BCD/ASC render-length rules, Luhn checksum, XOR checksum, and
  low-six-bit length extraction;
- return `HbciErrorKind::InvalidArgument` for invalid parse/render data instead
  of Java unchecked exceptions.

For `FlickerRenderer`, port the deterministic bit-frame mapping but not the
threaded sleep/timing loop. Expose `frames_for_iterations(...)`, which returns
the same paint frames the Java renderer would emit for a fixed number of full
transmissions.

## Consequences

The Rust port now covers the deterministic `FlickerTest` behavior without
introducing GUI, thread, or timing concerns into the library API.

Later PinTAN callback integration can choose whether to expose the rendered
flicker string, the synchronous frame stream, or an application-level renderer
adapter.

Remaining work:

- integrate `FlickerCode::try_parse(...)` into HITAN/SCA challenge processing;
- decide whether an async/timed renderer belongs in the crate or in examples;
- port `ChallengeInfo` and the remaining security-mechanism metadata tests.

## Links

- `src/manager/secmech.rs`
- `tests/secmech.rs`
- `docs/adr/0129-secmech-qr-matrix-parser-parity.md`
- Upstream: `org.kapott.hbci.manager.FlickerCode`
- Upstream: `org.kapott.hbci.manager.FlickerRenderer`
- Upstream: `org.kapott.hbci4java.secmech.FlickerTest`
