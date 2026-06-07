# Java To Rust Mapping

This page maps common hbci4java concepts to their current Rust equivalents.
The Rust port stays original-near for protocol behavior, job names, parameter
keys, and tests, while using Rust-cased public types and async control flow.

## Naming Rule

Rust public types use Rust casing:

```text
HBCIHandler -> HbciHandler
HBCIJob -> HbciJob
HBCICallback -> HbciCallback
HBCIException -> HbciError
```

Original job names and parameter keys stay unchanged:

```text
new_job("SaldoReq")
try_set_param("my.iban", "DE...")
try_set_param("src.number", "1234567890")
```

Lowlevel segment names and response content keys also stay close to hbci4java
and the original XML resources, for example `Saldo7`, `CustomMsg5`,
`content.KTV.iban`, and `CustomMsgRes.GVRes.SaldoRes7`.

## Core Types

| hbci4java | hbci4rust | Notes |
| --- | --- | --- |
| `HBCIHandler` | `HbciHandler` | Async handler; the library does not own a Tokio runtime. |
| `HBCIJob` / `HBCIJobImpl` | `HbciJob` | String-parameter job object with Java job names. |
| `HBCIJobResult` | `HbciJobResult` | Per-job status, raw data, and optional typed result. |
| `HBCIExecStatus` | `HbciExecStatus` | Overall execution status and message/job return values. |
| `HBCIMsgStatus` | `HbciMsgStatus` | Message-level global and segment status. |
| `HBCIRetVal` | `HbciReturnValue` | Return code, text, params, segment reference, element. |
| `HBCICallback` | `HbciCallback` | Async event/response callback trait. |
| `HBCIPassportPinTan` | `PinTanPassport` | Rust-native PinTAN passport only in v1. |
| `HBCIUtils` params | `set_param` / `get_param` | Global parameter helpers kept for original-near behavior. |
| `Comm` classes | `CommClient` | Trait with default HTTPS and replay implementations. |

## Handler Flow

hbci4java's synchronous flow:

```java
HBCIJob job = handler.newJob("SaldoReq");
job.setParam("my.iban", iban);
job.addToQueue();
HBCIExecStatus status = handler.execute();
```

Current Rust flow:

```rust
let mut handler = HbciHandler::new("300", passport);
let mut job = handler.new_job("SaldoReq")?;
job.try_set_param("my.iban", iban)?;
handler.try_add_to_queue(job)?;
let status = handler.execute().await?;
```

Use `init().await` and `close().await` for explicit dialog lifecycle control
when a test or client needs it. `execute().await` sends queued jobs through the
current signed PinTAN `CustomMsg` path.

## Jobs And Parameters

The registry is static for v1:

- use `JobRegistry::pintan()` or `HbciHandler::new_job(...)`;
- valid names are listed in `PINTAN_JOB_NAMES`;
- `scripts/audit-job-coverage.sh` verifies coverage against upstream `GV*.java`
  classes.

Setters:

| Rust method | Use |
| --- | --- |
| `set_param` | Permissive Java-like setter. |
| `try_set_param` | Checked setter for known, non-empty params. |
| `try_set_param_int` | Integer convenience shape. |
| `try_set_param_date` | ISO date input normalized before storage. |
| `try_set_indexed_param` | Java-style indexed repeated fields. |
| `set_param_account` | Account-structured helper. |
| `set_param_value` | Amount/currency helper. |

`verify_constraints()` resolves defaults and lowlevel parameter destinations.
`try_add_to_queue(...)` calls this before queuing. Account CRC callbacks can be
run through `try_add_to_queue_with_account_checks(...).await`.

## Results

Generic result data is always available through `HbciJobResult`:

- `global_return_values`;
- `return_values`;
- `result_data`;
- `raw_response`.

Typed results are stored in `HbciJobResultData`. The names are Rust-cased and
sometimes intentionally shared where hbci4java has several compatible result
classes:

```text
GVRLastSEPA / GVRLastCOR1SEPA / GVRLastB2BSEPA -> LastSepa
GVRDauerLastList -> DauerList
GVRDauerLastNew -> DauerNew
```

`scripts/audit-result-coverage.sh` verifies normalized result coverage against
upstream `GVR*.java` classes.

## Passport And Storage

v1 supports Rust-native PinTAN passport data:

```text
HBCIPassportPinTan -> PinTanPassport + PinTanPassportData
```

The storage format is Rust-native and encrypted. Java passport import/export is
out of v1 scope. Runtime PIN caching is held in `PinTanPassport`; persistent BPD,
UPD, account metadata, TAN mechanisms, and short-lived SCA state are updated
from replayed or live FinTS responses.

## Callback And TAN Flow

Callbacks are async:

```rust
#[async_trait]
impl HbciCallback for MyCallback {
    async fn handle(&self, event: CallbackEvent) -> HbciResult<CallbackResponse> {
        /* return PIN, TAN, or selections */
    }
}
```

The Java `StringBuffer` and `ThreadSyncer` callback response style is not
ported. Callback reason and data-type codes remain close to hbci4java and are
covered by callback code mapping tests.

## Communication And Tests

`CommClient` is the transport boundary:

- `DefaultCommClient` sends async HTTPS requests;
- `ReplayCommClient` records requests and returns deterministic offline
  responses;
- CI tests stay offline and do not depend on real bank credentials.

Replay tests inspect actual FinTS wire payloads, including signed `CustomMsg`
requests.

Optional live smoke hooks are documented in `docs/reference/live-bank-tests.md`.
They are ignored, env-gated, and not part of CI acceptance.

## V1 Boundaries

The following hbci4java concepts are intentionally outside v1:

- chipcard, PCSC, CTAPI, DDV;
- RDH, RAH, RSA key-file live support;
- Java passport import/export;
- `GVTemplate` and arbitrary `newLowlevelJob(...)` public API;
- `GVRWPStammData`, because upstream documents it as requiring lowlevel
  `WPStammList`.

## Coverage References

- `docs/architecture/job-coverage.md`
- `docs/architecture/result-coverage.md`
- `scripts/audit-job-coverage.sh`
- `scripts/audit-result-coverage.sh`
