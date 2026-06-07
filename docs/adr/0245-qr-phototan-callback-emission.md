# ADR 0245: QR And PhotoTAN Callback Emission

## Status

Accepted

## Context

ADR 0129 ported the QR/photoTAN security-mechanism helpers:

- `MatrixCode` for photoTAN image payloads;
- `QrCode` for QR-code payloads;
- `HhdVersion` and `HhdVersionType` detection.

The runtime still emits the generic `NeedPtTan` callback for those SCA
challenges. hbci4java is more specific when it can safely validate the payload:

- `HBCICallback.NEED_PT_PHOTOTAN = 33`;
- `HBCICallback.NEED_PT_QRTAN = 34`.

hbci4java deliberately does not rely only on TAN-mechanism metadata. It emits
the photoTAN or QR callback only if the stored HHD-UC data can be parsed by the
matching helper. Otherwise it falls back to the generic TAN callback.

## Decision

Add Rust-cased callback reasons:

- `CallbackReason::NeedPtPhotoTan`;
- `CallbackReason::NeedPtQrTan`.

Map them to and from the original hbci4java constants `33` and `34`.

During SCA TAN collection:

- detect the current HHD type from the current security mechanism;
- for `PhotoTan`, emit `NeedPtPhotoTan` only when
  `MatrixCode::try_parse(hhd_uc)` succeeds;
- for `QrCode`, emit `NeedPtQrTan` only when
  `QrCode::try_parse(hhd_uc, formatted_challenge_message)` succeeds;
- pass the original HHD-UC data as `current_value`, matching hbci4java's raw
  payload-in-callback behavior;
- still expect the callback response value to contain the final TAN;
- keep invalid or missing specialized payloads on the existing `NeedPtTan`
  path.

Do not add image decoding, rendering, or UI abstractions in this slice. The
library exposes the original-near callback payload and keeps applications in
control of presentation.

## Consequences

Applications can distinguish QR/photoTAN flows without parsing TAN-method names
themselves, while existing generic TAN callbacks still work for unknown or
malformed payloads.

Tests must pin numeric callback constants, parser-gated emission, raw HHD-UC
payload delivery, and fallback to `NeedPtTan` when parser validation fails.

## Links

- ADR 0129: Secmech QR Matrix Parser Parity
- ADR 0143: SCA TAN Callback Helper
- ADR 0243: Decoupled PinTAN Callback Code Mapping
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/callback/HBCICallback.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/passport/HBCIPassportPinTan.java`
