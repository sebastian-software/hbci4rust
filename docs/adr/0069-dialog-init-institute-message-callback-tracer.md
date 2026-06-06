# ADR 0069: Dialog Init Institute Message Callback Tracer

## Status

Accepted

## Context

hbci4java collects `KIMsg` institute messages during dialog initialization after
parsing the dialog-init response and updating institute/user parameter data.

For every collected `HBCIInstMessage`, hbci4java calls the callback with:

- reason `HAVE_INST_MSG`;
- the message display string;
- data type `TYPE_NONE`;
- no expected response data.

ADR 0067 introduced `HbciInstMessage`.

ADR 0068 introduced `HbciInstMessage::collect_from_values(...)`.

The Rust handler already has an async callback API and a `HaveInstMsg` reason,
but dialog initialization did not yet emit institute-message callbacks.

## Decision

During `HbciHandler::init(...)`, after:

- validating the dialog-init response;
- updating dialog context;
- updating BPD/UPD passport data;
- updating UPD accounts;

collect institute messages from the parsed flat value map using the
`DialogInitRes.KIMsg` base.

For each message, call the configured callback with:

- `CallbackReason::HaveInstMsg`;
- `message.to_string()`;
- `CallbackDataType::None`;
- `current_value: None`.

Do not change `HbciExecStatus::messages`. It remains a return-code message list
for executed job messages.

Do not add persistent storage for institute messages in this slice.

## Consequences

Dialog initialization now exposes bank institute messages in the same behavioral
place as hbci4java: a callback event rather than a status return object.

The callback remains async-first and does not use Java's `StringBuffer`
response channel.

Replay tests cover real `HIKIM` segments flowing through parser, handler, and
callback.

Remaining work:

- decide whether dialog-init status should later retain institute messages in a
  structured status hierarchy;
- decide whether application-facing docs should recommend displaying
  `HaveInstMsg` events directly;
- revisit callback ordering if the full dialog engine introduces additional
  init-stage events.

## Links

- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `docs/adr/0067-institute-message-tracer.md`
- `docs/adr/0068-institute-message-collection-tracer.md`
- Upstream: `org.kapott.hbci.manager.HBCIDialog`
- Upstream: `org.kapott.hbci.callback.HBCICallback#HAVE_INST_MSG`
