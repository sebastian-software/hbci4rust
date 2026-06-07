# ADR 0168: TAN Media List Job

## Status

Accepted

## Context

The v1 scope is PinTAN/HBCI-Plus. hbci4java exposes the `TANMediaList` high-level job through
`GVTANMediaList`, which renders `HKTAB` and parses `HITAB` into `GVRTANMediaList`. The job is
important for PinTAN because some TAN methods require a named TAN medium, and hbci4java also copies
active media names from the response into the passport UPD state.

The current Rust port has the `TANMediaList` name in the PinTAN registry, but queued job rendering
and result extraction are not implemented yet. The protocol resources already contain
`TANMediaList1` through `TANMediaList4` in `hbci-plus.xml`, with version 4 being the newest
HBCI-Plus shape and the first request version that includes `mediacategory`.

## Decision

Port `TANMediaList` as a Java-near high-level job:

- keep the public job name `TANMediaList`;
- add original-near constraints `mediatype -> TANMediaList4.mediatype` with default `0` and
  `mediacategory -> TANMediaList4.mediacategory` with default `A`;
- render `TANMediaList4` (`HKTAB` version 4) for HBCI-Plus custom messages;
- parse `TANMediaListRes4` (`HITAB` version 4) into a Rust result structure mirroring
  `GVRTANMediaList`'s observable fields;
- expose the parsed result through `HbciJobResultData`;
- copy active media names (`status == "1"` and non-empty `medianame`) into the Rust PinTAN
  passport's `tan_media_names`, preserving hbci4java's UPD side effect.

Older response versions can be added later when real fixtures require them. The first slice stays
on version 4 because it matches the current HBCI-Plus maximum and the Java defaults that mention
both media type and media category.

## Consequences

This broadens the PinTAN job surface with a practical management job and makes later TAN-media
selection flows better grounded in live-bank metadata. It also introduces the first non-saldo,
non-transaction-result job result variant, so future job ports can reuse the same pattern.

Open follow-up work:

- parse `TANMediaListRes1` through `TANMediaListRes3` if replay fixtures expose older banks;
- decide whether a dedicated TAN-media refresh dialog helper is needed, like hbci4java's
  `HBCIDialogTanMedia`;
- add process-specific SCA reference handling for `HKTAB` if banks require a TAN challenge for
  TAN-media list retrieval.
