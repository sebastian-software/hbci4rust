# ADR 0129: Secmech QR Matrix Parser Parity

## Status

Accepted

## Context

hbci4java contains small PinTAN security-mechanism helpers in the `manager`
package:

- `QRCode` extracts QR image bytes and cleaned challenge text from HHD/CHLGUC
  challenge payloads;
- `MatrixCode` extracts server-provided photoTAN image bytes from a compact
  binary payload;
- `HHDVersion` detects chipTAN, QR, photoTAN, and decoupled variants from
  BPD security-mechanism properties.

These helpers are in the v1 PinTAN/HBCI-Plus scope and are listed in the
offline-domain phase of the port plan. They do not require live bank access.

The upstream tests check parser boundaries and fixture byte lengths, not QR or
photoTAN image rendering.

## Decision

Add Rust-cased equivalents under `manager`:

- `QrCode`;
- `MatrixCode`;
- `HhdVersion`;
- `HhdVersionType`.

Keep the parsers original-near:

- `MatrixCode` reads the first two bytes as the Java decimal-byte length field,
  then copies the MIME type and remaining image bytes;
- `QrCode` first tries the same direct image payload shape for long HHD data,
  then falls back to CHLGUC/CHLGTEXT extraction from the challenge text;
- CHLGUC payloads are decoded with standard Base64 after removing the same
  whitespace characters as hbci4java;
- the MIME type is set to `image/png` when QR image bytes start with the PNG
  signature;
- invalid input returns `HbciErrorKind::InvalidArgument` instead of throwing.

Use the existing locked `base64` crate explicitly for Base64 decoding. Do not
add QR generation, image decoding, or rendering dependencies in this slice.

Copy only the three upstream security-mechanism fixtures used by the QR and
Matrix tests. Do not vendor all security-mechanism test resources.

## Consequences

The Rust port can represent the first QR/photoTAN parser behaviors needed by
later SCA/TAN flows while keeping byte-level tests offline.

The HHD version detector now covers the upstream test cases for QR 1.3/1.4,
HHD 1.3/1.4, photoTAN, and decoupled methods.

Remaining work:

- extend QR/photoTAN callback fixtures when more bank-specific payload shapes
  appear.

## Links

- `src/manager/secmech.rs`
- `tests/secmech.rs`
- `tests/fixtures/hbci4java/secmech/`
- Upstream: `org.kapott.hbci.manager.QRCode`
- Upstream: `org.kapott.hbci.manager.MatrixCode`
- Upstream: `org.kapott.hbci.manager.HHDVersion`
- Upstream: `org.kapott.hbci4java.secmech.TestQRCode`
- Upstream: `org.kapott.hbci4java.secmech.TestMatrixCode`
- Upstream: `org.kapott.hbci4java.secmech.TestHHDVersion`
