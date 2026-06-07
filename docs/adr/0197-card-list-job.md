# ADR 0197: CardList Job

## Status

Accepted

## Context

`GVCardList` queries information about cards issued for an account. It is a
small account-bound query job and returns hbci4java's `GVRCardList`.

For HBCI 300 the protocol XML provides `CardList1` and `CardList2`. Version 1
uses `KTV2`; version 2 uses `KTV3` and adds `cardbez` in the response. The Java
result class does not expose `cardbez`, and `GVCardList.extractResults` maps
only card type, card number, next card number, owner, validity dates, card
limit, and comment.

## Decision

Port `CardList` as an original-near query job.

- Use `CardList2` / `HKAZK` version 2 for HBCI 300 requests and
  `CardListRes2` / `HIAZK` version 2 for responses.
- Keep Java-compatible frontend parameters: `my.country`, `my.blz`,
  `my.number`, and `my.subnumber`.
- Render the account as national `KTV3` under `KTV`, matching the HBCI 300
  segment definition.
- Add `GvrCardList` and `GvrCardInfo` result types shaped after
  `GVRCardList.CardInfo`.
- Preserve `cardbez` in raw `content.*` result data, but do not add it to the
  structured card info type in this slice because hbci4java's public result
  object does not expose it.
- Verify account CRC for `my`, matching hbci4java's `checkAccountCRC("my")`.

## Consequences

This adds another small PinTAN-compatible information query while keeping the
public structured result close to hbci4java. Later work can revisit repeated
response-segment collection once the handler gains broader multi-result
mapping.
