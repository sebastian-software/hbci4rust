# V1 Readiness

Snapshot date: 2026-06-07.

This page tracks readiness for the scoped v1 PinTAN/HBCI-Plus port. It does not
measure the full hbci4java repository, because chipcard, PCSC, CTAPI, DDV,
RDH/RAH/RSA key-file live support, Java passport import/export, and arbitrary
lowlevel jobs are intentionally outside v1.

## Working Estimate

Current v1 readiness is roughly 75% +/- 5%.

That estimate is deliberately lower than the source-surface audit numbers:

- static high-level job coverage is 67 of 68 upstream `GV*.java` classes, with
  only the intentional `GVTemplate` boundary missing;
- normalized typed result coverage is 23 of 24 upstream `GVR*.java` shapes, with
  only the intentional `WPStammData` boundary missing.

The remaining distance is mostly runtime confidence, replay breadth, public API
hardening, and release documentation rather than missing job names.

## Evidence Matrix

| Area | Evidence | State | Remaining Work |
| --- | --- | --- | --- |
| Scope and baseline | ADRs 0001, 0003, 0005; `scripts/fetch-upstream.sh` | Accepted and pinned to hbci4java `hbci4j-core-4.1.11` / `3b7ce667c73724daa1c836ed7333ed090c21a831` | Keep scope stable unless a new ADR changes v1. |
| License and attribution | ADR 0002; ADR 0249; ADR 0253; `LICENSE`; `NOTICE`; `docs/reference/packaging.md`; `docs/reference/upstream-header-review.md` | LGPL-2.1-or-later assumption, upstream attribution points, and pinned-baseline header recheck are recorded | Rerun header/package review if the upstream baseline or copied artifacts change before release. |
| Crate bootstrap | `Cargo.toml`; `README.md`; `.github/workflows/ci.yml`; `cargo fmt --check`; `cargo clippy --all-targets`; `cargo test` | Single Rust 2024 crate with offline CI and test suite | Keep package metadata and release gates current as the v1 surface changes. |
| Protocol resources | `resources/protocol/`; `tests/protocol_resources.rs`; `tests/protocol_wire.rs`; `tests/protocol_message.rs`; `docs/reference/malformed-bank-responses.md`; ADR 0255 | Original XML/DTD resources are loaded, parsed, and covered for named malformed-response classes | Add deterministic replays or limitation entries for any new malformed bank-response behavior. |
| Foundational types | `src/tools.rs`; `src/error.rs`; `src/callback.rs`; `src/manager/*`; `tests/tools.rs`; `tests/status.rs`; `tests/account_crc.rs`; `tests/bank_info.rs` | Core helpers, callback codes, status types, bank info, and account CRC behavior are pinned with tests | Keep adding upstream fixture cases when Java behavior is ambiguous. |
| Offline SEPA/CAMT/SWIFT parity | `src/sepa/`; `src/swift/`; `tests/sepa.rs`; `tests/swift.rs`; `tests/structures.rs`; upstream fixture copies; `docs/reference/parser-generator-goldens.md`; ADR 0254 | Main offline parser/generator behavior is covered with original-near fixtures, observable parity tests, and explicit v1 limitations | Add Java goldens or limitation entries for any new risky parser/generator behavior before release acceptance depends on it. |
| PinTAN passport and storage | `src/passport/pintan.rs`; `src/passport/storage.rs`; `tests/passport.rs`; `docs/reference/passport-storage-security.md`; ADRs 0008 and 0251 | Rust-native encrypted PinTAN storage, KDF/AEAD/envelope review, envelope validation, and runtime PIN/TAN handling are present | Add migration/versioning tests when the first persisted format revision is introduced. |
| Communication boundary | `src/comm/mod.rs`; `src/comm/replay.rs`; replay tests in `tests/bootstrap.rs` | Async `CommClient`, default HTTPS client, and replay client exist | Add bank-specific replay fixtures for more SCA variants. |
| Handler and dialog runtime | `src/manager/handler.rs`; dialog and PinTAN tests in `tests/bootstrap.rs`; connection lifecycle tests in `tests/runtime_callbacks.rs`; `docs/reference/error-reporting.md`; `docs/reference/malformed-bank-responses.md`; ADRs 0154-0167, 0238-0245, 0250, and 0255 | Dialog init/close, signed messages, TAN processes, SCA callbacks, QR/photoTAN callback emission, connection callbacks, failed process-2 submission retry state, process-2 transport retry state, decoupled status-request rendering, decoupled refresh polling, status/error reporting review, malformed-response evidence, and replayed execution paths are substantially covered | Increase replay breadth and document new bank-specific error cases as they appear. |
| High-level jobs | `src/gv/mod.rs`; `scripts/audit-job-coverage.sh`; `docs/architecture/job-coverage.md`; `docs/reference/unsupported-surfaces.md` | 67/68 upstream job classes covered; only `GVTemplate` is intentionally out of scope | Keep audit current after any registry or upstream-baseline change. |
| Typed results | `src/gv_result/mod.rs`; `scripts/audit-result-coverage.sh`; `docs/architecture/result-coverage.md`; `docs/reference/unsupported-surfaces.md` | 23/24 normalized upstream result shapes covered; only `WPStammData` is intentionally out of scope | Add typed result details if replay fixtures expose currently raw-only fields. |
| Java-to-Rust mapping | `docs/reference/java-to-rust-mapping.md`; `docs/reference/public-api.md`; `docs/reference/migration-examples.md`; `docs/reference/error-reporting.md`; `docs/reference/unsupported-surfaces.md`; `tests/public_api.rs`; ADRs 0241, 0247, 0248, 0250, and 0252 | Major concepts, v1 boundaries, crate-root API groups, handler flow, PinTAN execution guidance, callback reasons, status/error inspection, checked balance-request shape, high-risk statement/SEPA migration examples, and unsupported surfaces are documented | Keep examples current as live-bank observations or migration questions expose unclear job shapes. |
| Optional live smoke | `tests/live_bank.rs`; `docs/reference/live-bank-tests.md`; ADR 0236 | Ignored, env-gated PinTAN dialog init/close hook exists outside CI | Run manually against selected banks and convert observations into replay fixtures. |
| Release hardening | This page; `docs/architecture/release-checklist.md`; `docs/reference/packaging.md`; `docs/reference/passport-storage-security.md`; `docs/reference/unsupported-surfaces.md`; `docs/reference/upstream-header-review.md`; `docs/reference/parser-generator-goldens.md`; `docs/reference/malformed-bank-responses.md`; `docs/rustification/README.md`; porting plan; ADRs 0246, 0249, 0251, 0252, 0253, 0254, and 0255 | Remaining work is visible, package metadata/package-list review, storage security review, public unsupported-surface reference, upstream header recheck, parser/generator golden policy, and malformed-response evidence are recorded, and v1 is not declared complete | Complete final release-candidate gates and any remaining explicit limitation docs. |

## Recheck Commands

```sh
cargo fmt --check
cargo clippy --all-targets
cargo test
cargo test -- --list
cargo test --test live_bank -- --ignored
scripts/audit-job-coverage.sh
scripts/audit-result-coverage.sh
```

The live-bank test command is safe by default: without
`HBCI4RUST_LIVE_ENABLE=1`, it exits without opening a network connection.

## Completion Bar

V1 can be called complete only when:

- the regular offline gates pass;
- job and result audits still show only the intentional v1 exclusions;
- representative PinTAN replay fixtures cover init, close, one-step TAN,
  process-1, process-2, SCA exemption, and error paths;
- public API docs and Java-to-Rust mapping are sufficient for a Java user to
  migrate common PinTAN workflows;
- live-bank smoke observations, if any, have been converted into deterministic
  replay fixtures or explicit documented limitations;
- the blocking items in `docs/architecture/release-checklist.md` are checked or
  explicitly documented as limitations.
