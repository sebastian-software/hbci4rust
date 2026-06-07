# Public API Reference

This document reviews the crate-root exports in `src/lib.rs` for the scoped v1
PinTAN/HBCI-Plus port. It is intentionally original-near: Java job names,
parameter keys, result concepts, and lowlevel protocol helpers stay visible so
existing hbci4java users can migrate incrementally.

For workflow examples, see `docs/reference/java-to-rust-mapping.md`.

## Primary PinTAN Path

These exports are the public entry points most applications should start with:

```text
HbciHandler
HbciJob
HbciJobResult
HbciJobResultData
HbciExecStatus
HbciDialogStatus
HbciMsgStatus
HbciInstMessage
HbciReturnValue
HbciStatus
HbciStatusCode
HbciError
HbciErrorKind
HbciResult
```

Java-near rules:

- construct jobs with Java names such as `new_job("SaldoReq")`;
- keep parameter keys such as `my.iban`, `src.iban`, and `_sepapain`;
- prefer checked setters such as `try_set_param(...)` for application input;
- use permissive `set_param(...)` when deliberately matching hbci4java's loose
  string-parameter behavior.

The public balance-request migration shape is covered by
`tests/public_api.rs`.

## Passport And Storage

v1 supports Rust-native PinTAN storage only:

```text
PinTanPassport
PinTanPassportData
PinTanScaState
PinTanScaUpdate
PassportStorage
UserSig
ONESTEP_TAN_METHOD_ID
TanMethodSelection
TanMethodOption
```

Out-of-v1 boundaries remain unchanged: no chipcard, PCSC, CTAPI, DDV,
RDH/RAH/RSA key-file live support, or Java passport import/export.

## Callback Surface

Callbacks are async and event/response based:

```text
HbciCallback
CallbackEvent
CallbackResponse
CallbackReason
CallbackDataType
```

The Java `StringBuffer` and `ThreadSyncer` callback response shape is not
ported. `CallbackReason::original_code()` and
`CallbackReason::from_original_code(...)` preserve the hbci4java integer codes
for ported reasons.

## Communication And Replay

Transport is explicit and async:

```text
CommClient
CommRequest
CommResponse
DefaultCommClient
ReplayCommClient
```

`DefaultCommClient` is the HTTPS implementation. `ReplayCommClient` is part of
the public v1 surface because deterministic offline replays are the main way to
pin original-near behavior without live credentials.

## Job Registry And Constraints

The v1 registry is static and PinTAN-scoped:

```text
JobRegistry
HbciJobConstraint
PINTAN_JOB_NAMES
```

`GVTemplate` and arbitrary lowlevel `newLowlevelJob(...)` are not exposed for
v1. The audit script records that as an intentional boundary.

## Typed Results And Structures

Typed results are re-exported from the crate root so applications do not need to
know the internal module layout:

```text
GvrAccInfo
GvrAccInfoAddress
GvrAccInfoEntry
GvrCardInfo
GvrCardList
GvrDauerList
GvrDauerListAussetzung
GvrDauerListEntry
GvrFestCond
GvrFestCondList
GvrFestList
GvrFestListEntry
GvrFestListProlong
GvrInfoList
GvrInfoListInfo
GvrInfoOrder
GvrInfoOrderInfo
GvrInstUebSepa
GvrKUms
GvrKUmsBTag
GvrKUmsLine
GvrKontoauszug
GvrKontoauszugEntry
GvrSaldoReq
GvrSaldoReqInfo
GvrStatus
GvrStatusEntry
GvrTanInfo
GvrTanList
GvrTanListEntry
GvrTanMediaInfo
GvrTanMediaList
GvrTermUeb
GvrTermUebEdit
GvrTermUebList
GvrTermUebListEntry
GvrVoP
GvrWPDepotList
GvrWPDepotUms
GvrWPDepotUmsEntry
GvrWPDepotUmsInstrument
Konto
KontoauszugFormat
Limit
Saldo
Value
VoPResult
VoPResultItem
VoPStatus
```

`WPStammData` remains out of the v1 typed-result surface because upstream uses
it through lowlevel template-style behavior.

## Protocol, Security-Mechanism, And Utility Helpers

These exports are intentionally available for original-near tests, replay
fixtures, and advanced callers that need FinTS details:

```text
DialogContext
KnownReturncode
AccountCrcAlgs
AppliedChallengeParams
BankInfo
BankInfoRegistry
ChallengeHhdVersion
ChallengeInfo
ChallengeJob
ChallengeParam
FlickerCode
FlickerCodeVersion
FlickerDataElement
FlickerEncoding
FlickerRenderer
FlickerStartCode
HbciVersion
HhdVersion
HhdVersionType
MatrixCode
OrderHashMode
PinTanSigHead
PinTanSignatureContext
QrCode
ParameterFinder
ParameterQuery
Properties
```

The crate also re-exports original-near helper functions:

```text
apply_pintan_sig_head
apply_pintan_sig_tail_from_head
apply_pintan_signature_shell
apply_pintan_user_sig_to_sig_tail
collect_pintan_segment_codes
collect_pintan_signature_range
done
get_param
has_text
init
join_strings
safe_filename
set_param
to_boolean
to_ins_code
to_parameter_code
```

`init`, `done`, `set_param`, and `get_param` deliberately mirror the global
`HBCIUtils` style. They are original-near compatibility helpers, not a new
configuration facade.

## Public Modules

The internal package-near modules remain public during v1:

```text
callback
comm
dialog
error
gv
gv_result
manager
passport
protocol
sepa
swift
tools
```

This keeps fixture work and Java-near parity checks straightforward while the
port is still hardening.

## Review Notes

- The crate-root export list is intentionally broad for v1.
- Every crate-root exported type name from `src/lib.rs` is listed above by
  role, together with the public modules.
- The primary application path remains `HbciHandler` plus Java-named
  `HbciJob`s.
- Replay, protocol, and security-mechanism helpers stay public until v1 parity
  is stable, because the test and fixture strategy depends on them.
- Any future narrowed facade belongs in `docs/rustification/` and must not
  change v1 parity acceptance without a new ADR.
