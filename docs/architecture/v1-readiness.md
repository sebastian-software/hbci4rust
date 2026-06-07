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
| License and attribution | ADR 0002; `LICENSE`; `NOTICE` | LGPL-2.1-or-later assumption recorded | Recheck before packaging/release if upstream headers create a distribution question. |
| Crate bootstrap | `Cargo.toml`; `README.md`; `cargo fmt --check`; `cargo clippy --all-targets`; `cargo test` | Single Rust 2024 crate with offline test suite | Add CI workflow if this repository is pushed to a remote host. |
| Protocol resources | `resources/protocol/`; `tests/protocol_resources.rs`; `tests/protocol_wire.rs`; `tests/protocol_message.rs` | Original XML/DTD resources are loaded and parsed | Broaden malformed-bank-response replay coverage as real cases appear. |
| Foundational types | `src/tools.rs`; `src/error.rs`; `src/callback.rs`; `src/manager/*`; `tests/tools.rs`; `tests/status.rs`; `tests/account_crc.rs`; `tests/bank_info.rs` | Core helpers, callback codes, status types, bank info, and account CRC behavior are pinned with tests | Keep adding upstream fixture cases when Java behavior is ambiguous. |
| Offline SEPA/CAMT/SWIFT parity | `src/sepa/`; `src/swift/`; `tests/sepa.rs`; `tests/swift.rs`; `tests/structures.rs`; upstream fixture copies | Main offline parser/generator behavior is covered with original-near fixtures | Add more Java golden artifacts for risky edge cases before release. |
| PinTAN passport and storage | `src/passport/pintan.rs`; `src/passport/storage.rs`; `tests/passport.rs`; ADR 0008 | Rust-native encrypted PinTAN storage and runtime PIN/TAN handling are present | Add migration/versioning tests when the first persisted format revision is introduced. |
| Communication boundary | `src/comm/mod.rs`; `src/comm/replay.rs`; replay tests in `tests/bootstrap.rs` | Async `CommClient`, default HTTPS client, and replay client exist | Add bank-specific replay fixtures for more SCA variants. |
| Handler and dialog runtime | `src/manager/handler.rs`; dialog and PinTAN tests in `tests/bootstrap.rs`; connection lifecycle tests in `tests/runtime_callbacks.rs`; ADRs 0154-0167, 0238, and 0239 | Dialog init/close, signed messages, TAN processes, SCA callbacks, connection callbacks, failed process-2 submission retry state, and replayed execution paths are substantially covered | Increase replay breadth and harden error/reporting behavior before calling v1 complete. |
| High-level jobs | `src/gv/mod.rs`; `scripts/audit-job-coverage.sh`; `docs/architecture/job-coverage.md` | 67/68 upstream job classes covered; only `GVTemplate` is intentionally out of scope | Keep audit current after any registry or upstream-baseline change. |
| Typed results | `src/gv_result/mod.rs`; `scripts/audit-result-coverage.sh`; `docs/architecture/result-coverage.md` | 23/24 normalized upstream result shapes covered; only `WPStammData` is intentionally out of scope | Add typed result details if replay fixtures expose currently raw-only fields. |
| Java-to-Rust mapping | `docs/reference/java-to-rust-mapping.md` | Major concepts and v1 boundaries are documented | Expand with per-job migration examples when the API stabilizes. |
| Optional live smoke | `tests/live_bank.rs`; `docs/reference/live-bank-tests.md`; ADR 0236 | Ignored, env-gated PinTAN dialog init/close hook exists outside CI | Run manually against selected banks and convert observations into replay fixtures. |
| Release hardening | This page; `docs/rustification/README.md`; porting plan | Remaining work is visible but v1 is not declared complete | Add release checklist, API docs pass, and crate packaging review. |

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
  replay fixtures or explicit documented limitations.
