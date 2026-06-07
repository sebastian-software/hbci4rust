# Java To Rust Mapping

This page maps common hbci4java concepts to their current Rust equivalents.
The Rust port stays original-near for protocol behavior, job names, parameter
keys, and tests, while using Rust-cased public types and async control flow.
For a crate-root export review, see `docs/reference/public-api.md`.
For high-risk per-job examples, see `docs/reference/migration-examples.md`.
For error and status inspection, see `docs/reference/error-reporting.md`.

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

The Rust v1 flow is async and keeps Java job names and parameter keys:

```rust
let mut handler = HbciHandler::new("300", passport);
let mut job = handler.new_job("SaldoReq")?;
job.try_set_param("my.iban", iban)?;
handler.try_add_to_queue(job)?;
let status = handler.execute_with_tan2step().await?;
```

Use `init().await` and `close().await` for explicit dialog lifecycle control
around live-bank dialogs. `execute_with_tan2step().await` is the v1 public
entry point closest to hbci4java's hidden PinTAN choreography for a queued
business job.

`execute().await` remains public, but it is the lower-level single-message
primitive. Use it for replay tests, explicit choreography helpers, and cases
where the queue already contains the exact FinTS message shape to send.

| Rust handler method | Java-near use |
| --- | --- |
| `init().await` | Open a FinTS dialog and import BPD/UPD metadata. |
| `try_add_to_queue(job)` | Verify constraints and enqueue one Java-named business job. |
| `execute_with_tan2step().await` | Execute queued work with selected one-step or two-step PinTAN handling. |
| `execute().await` | Send the current queue as one signed `CustomMsg` without extra TAN dispatch. |
| `close().await` | Send dialog end and reset the dialog context. |

For live PinTAN clients, the usual shape is:

```rust
use hbci4rust::{HbciHandler, HbciResult, PinTanPassport};

async fn load_balance(passport: PinTanPassport, iban: &str) -> HbciResult<()> {
    let mut handler = HbciHandler::new("300", passport);

    handler.init().await?;

    let mut job = handler.new_job("SaldoReq")?;
    job.try_set_param("my.iban", iban)?;
    handler.try_add_to_queue(job)?;

    let status = handler.execute_with_tan2step().await?;
    handler.close().await?;

    if !status.success {
        // Inspect status.error_string(), global_return_values,
        // segment_return_values, job_results, and known return-code helpers.
    }

    Ok(())
}
```

Tests can replace network I/O with `ReplayCommClient` and then inspect the
recorded FinTS request bodies.

## Error And Status Inspection

Java exceptions map to `HbciError` only when the Rust operation cannot produce
the normal result value. Bank-side return codes stay in status objects:

```rust
let status = handler.execute_with_tan2step().await?;

if status.is_invalid_pin() {
    let value = status.invalid_pin_code().expect("checked above");
    eprintln!("authentication failed: {value}");
}

if !status.success {
    eprintln!("{}", status.error_string());
}
```

This is intentionally close to hbci4java's split between thrown exceptions and
`HBCIExecStatus`/`HBCIRetVal` inspection.

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

Important v1 PinTAN reasons:

| Reason | Typical response |
| --- | --- |
| `NeedPtPin` | Return the dialog PIN. |
| `NeedPtTan` | Return the TAN for the current SCA challenge. |
| `NeedPtSecMech` | Return the selected TAN mechanism id. |
| `NeedPtTanMedia` | Return the selected TAN medium name. |
| `NeedPtPhotoTan` | Display/parse the raw photoTAN HHD-UC payload in `current_value`, then return the TAN. |
| `NeedPtQrTan` | Display/parse the raw QR-TAN HHD-UC payload in `current_value`, then return the TAN. |
| `NeedPtDecoupled` | Inform the user to approve the decoupled order externally; return value is ignored. |
| `NeedPtDecoupledRetry` | Inform the user that approval is still pending; `current_value` contains the BPD wait hint in seconds. |
| `NeedConnection` / `CloseConnection` | Observe transport lifecycle; usually return an empty accepted response. |
| `HaveInstMsg` | Display or log institute messages. |

Callbacks may receive `current_value` with challenge data, HHD/QR/photoTAN
payloads, selected defaults, or lifecycle hints depending on the reason.

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
