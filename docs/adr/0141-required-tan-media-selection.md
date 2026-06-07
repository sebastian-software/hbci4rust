# ADR 0141: Required TAN Media Selection

## Status

Accepted

## Context

hbci4java resolves the TAN medium in
`AbstractPinTanPassport.getTanMedia(int segVersion)`. The behavior is tied to
the selected two-step security mechanism:

- HKTAN segment versions below 3 do not send a TAN medium;
- for HKTAN version 3 and newer, BPD parameter `needtanmedia=2` means the TAN
  medium must be sent;
- hbci4java passes the cached UPD value `tanmedia.names` to callback reason
  `HBCICallback.NEED_PT_TANMEDIA` with message
  `*** Enter the name of your TAN media`;
- if the callback returns text, that value is used;
- if a medium is required but no text is available, hbci4java sends `noref`.

The Rust port already stores an optional selected `tan_media` and sets it on
explicit process-1 HKTAN jobs, but it does not yet consider `needtanmedia`,
`tanmedia.names`, callback selection, or the `noref` fallback.

## Decision

Add a small original-near TAN-media selection layer:

- store `tan_media_names` in `PinTanPassportData` as the Rust-side equivalent of
  UPD property `tanmedia.names`;
- import pipe-separated `DialogInitRes.UPD.tanmedia.names` values when present;
- expose whether the current selected security mechanism requires a TAN medium;
- keep the existing selected `tan_media` if present;
- for required media, ask the configured async callback with reason
  `NeedPtTanMedia`, data type `Text`, message
  `*** Enter the name of your TAN media`, and the pipe-joined media list as
  current value;
- persist a non-blank callback response as the selected `tan_media`;
- use `noref` when media is required but no selected/callback value exists;
- add an async process-1 HKTAN helper that performs this callback selection;
- keep the existing synchronous process-1 helper and make it use only stored
  media or `noref`.

Do not port the TAN media list job/result parser or dialog-level TAN media
refresh workflow in this slice.

## Consequences

Process-1 HKTAN preparation can now satisfy banks that require `tanmedia`
without hard-coding a medium in the passport beforehand.

Remaining work:

- port `GVTANMediaList` and `GVRTANMediaList`;
- refresh TAN media names from the dedicated TAN media dialog;
- wire media selection into full automatic HKTAN queue patching;
- port final TAN entry and SCA challenge callbacks.

## Links

- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.getTanMedia`
- Upstream: `org.kapott.hbci.manager.HBCIUser.UPD_KEY_TANMEDIA`
- Upstream: `org.kapott.hbci.callback.HBCICallback.NEED_PT_TANMEDIA`
