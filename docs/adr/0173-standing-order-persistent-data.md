# ADR 0173: Standing Order Persistent Data

## Status

Accepted

## Context

hbci4java passports expose `setPersistentData(String,Object)` and
`getPersistentData(String)` as a loose runtime cache. `GVDauerSEPAList` uses it
after parsing a standing-order list response: when the order has an `orderid`,
it stores all response properties below the current result header under
`dauer_{orderid}`, excluding `SegHead.*` and the `orderid` field itself.

Later standing-order edit/delete jobs read that snapshot to prepare follow-up
requests. The Rust port does not yet have a comparable persistent-data surface.

## Decision

Add a Rust-native persistent-data map to `PinTanPassportData`:

- Store values as `BTreeMap<String, Properties>`, not as arbitrary objects.
- Expose narrow `persistent_data`, `set_persistent_data`,
  `get_persistent_data`, and `remove_persistent_data` helpers on
  `PinTanPassport`.
- Populate `dauer_{orderid}` from `DauerSEPAListRes2` results using the same
  filter hbci4java uses: include fields under the response root, strip the root
  prefix, skip `SegHead.*`, and skip fields ending in `.orderid`.

Do not emulate arbitrary Java object storage. Do not wire edit/delete jobs in
this slice; they will consume the stored snapshot when those jobs are ported.

## Consequences

The standing-order list result now leaves the same follow-up cache that
hbci4java relies on, while keeping the Rust storage format typed and
serializable.

The map is intentionally broad enough for later `termueb_*`, `termlast_*`, and
VoP-related persistent data, but only `dauer_*` is introduced now.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVDauerSEPAList.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/passport/AbstractHBCIPassport.java`
- `docs/adr/0171-standing-order-list-job.md`
