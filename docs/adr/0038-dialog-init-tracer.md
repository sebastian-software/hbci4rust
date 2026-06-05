# ADR 0038: Dialog Init Tracer

## Status

Accepted

## Context

ADR 0037 can import UPD accounts from a flat `DialogInitRes.UPD` value map, but
the handler still did not send or parse dialog initialization traffic.

hbci4java starts a user dialog by rendering `DialogInit` from the original
protocol tables. Its data population is split across `AbstractRawHBCIDialogInit`
and `HBCIDialogInit`: bank identification goes into `Idn.KIK`, the customer id
and system id go into `Idn`, and BPD/UPD version, language, product name, and
product version go into `ProcPrep`.

For the current PinTAN-only tracer we do not yet have signed PinTAN dialog
state, SCA/TAN handling, BPD/UPD version persistence, or dialog-id lifecycle
management.

## Decision

Change `HbciHandler::init` from a callback-only placeholder into a narrow async
`DialogInit` tracer.

The tracer:

- requires a PinTAN passport host, bank code, and user id or customer id;
- renders `DialogInit` through the original `hbci-*.xml` message tree;
- sets `MsgHead.dialogid = 0`, `MsgHead.msgnum = 1`, and `MsgTail.msgnum = 1`;
- sets `Idn.KIK.country` from the passport country, defaulting to `DE` for the
  v1 German PinTAN scope when the field is empty;
- sets `Idn.KIK.blz` from the passport bank code;
- sets `Idn.customerid` from `customer_id`, falling back to `user_id`;
- sets `Idn.sysid = 0` and `Idn.sysStatus = 0`;
- sets `ProcPrep.BPD = 0`, `ProcPrep.UPD = 0`, and `ProcPrep.lang = 0`;
- identifies the product as `hbci4rust` with the Cargo package version truncated
  to the XML `prodVersion` size limit;
- sends the rendered message through the configured `CommClient`;
- parses HTTP-success responses as `DialogInitRes` using the existing wire
  parser and message mapper;
- imports returned UPD accounts into the Rust-native PinTAN passport by calling
  `PinTanPassport::update_accounts_from_values`.

`HbciHandler::init` now takes `&mut self`, because dialog initialization mutates
the passport account cache.

## Consequences

The handler now has a replay-testable first dialog step that stays close to the
original Java message names and XML paths.

This creates a concrete bridge from handler runtime to protocol rendering,
transport abstraction, response mapping, and passport account state.

The tracer intentionally remains incomplete:

- it does not sign or encrypt `DialogInit`;
- it does not attach HKTAN, process SCA challenges, or choose TAN media;
- it does not store BPD/UPD version numbers, system ids, dialog ids, or message
  counters;
- it does not expose a rich initialization status object;
- it treats HTTP error status as a network error and does not yet interpret
  FinTS return codes for initialization failure policy.

Those gaps are follow-up tracer slices before live-bank PinTAN parity.

## Links

- `src/manager/handler.rs`
- `src/passport/pintan.rs`
- `tests/bootstrap.rs`
- `resources/protocol/hbci-300.xml`
- Upstream: `org.kapott.hbci.dialog.AbstractRawHBCIDialogInit`
- Upstream: `org.kapott.hbci.dialog.HBCIDialogInit`
