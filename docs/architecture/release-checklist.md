# V1 Release Checklist

This checklist turns the ADR 0237 readiness matrix into an operational release
pass for the scoped PinTAN/HBCI-Plus v1 port. It is not a claim that v1 is
complete today. Before a v1 release candidate, every blocking item below must
either be checked with current evidence or moved into an explicit documented
limitation through a new ADR.

## Scope And Baseline

- [x] v1 scope is FinTS PinTAN / HBCI-Plus only.
- [x] Chipcard, PCSC, CTAPI, DDV, RDH, RAH, RSA key-file live support, Java
  passport import/export, and arbitrary lowlevel jobs are outside v1.
- [x] Upstream baseline is hbci4java tag `hbci4j-core-4.1.11` at
  `3b7ce667c73724daa1c836ed7333ed090c21a831`.
- [x] The upstream reference is fetched into `target/reference/`, not vendored.
- [x] Any future baseline or scope change is recorded in a new ADR before code
  changes rely on it.

Evidence:

```sh
scripts/fetch-upstream.sh
```

- ADR 0258

## Required Offline Gates

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets`
- [x] `cargo test`
- [x] `cargo test -- --list`
- [x] `scripts/audit-modern-scope.sh`
- [x] `scripts/audit-job-coverage.sh`
- [x] `scripts/audit-result-coverage.sh`
- [x] `git diff --check`

The release candidate must record the exact output summary of these commands.
Clippy warnings are visible during porting, but a v1 release candidate should
not ship with new warnings unless a dedicated ADR accepts the warning.

The command set can be run together with:

```sh
scripts/run-release-candidate-checks.sh
```

Use `scripts/run-release-candidate-checks.sh --package` for the final packaging
pass after the last release-candidate commit.

Final local evidence is captured by running
`CARGO_NET_OFFLINE=true scripts/run-release-candidate-checks.sh --package` on
the release-candidate commit. The runner writes the exact per-command summary
and full logs under `target/release-gates/`.

## Source Surface Coverage

- [x] Static high-level job registry covers all in-scope upstream `GV*.java`
  classes except the intentional `GVTemplate` lowlevel boundary and the
  unsupported `COR1`, DTAUS bulk, classic direct-debit, classic domestic
  transfer/account-transfer, classic scheduled-transfer, and classic
  standing-order jobs.
- [x] Normalized typed result coverage covers all in-scope upstream `GVR*.java`
  shapes except the intentional `WPStammData` lowlevel boundary.
- [x] Coverage audit docs are current after the final release-candidate commit.
- [x] Publicly documented unsupported surfaces match the audit exclusions.

Evidence:

- `docs/architecture/job-coverage.md`
- `docs/architecture/result-coverage.md`
- `docs/reference/java-to-rust-mapping.md`
- `docs/reference/unsupported-surfaces.md`
- ADR 0252
- ADR 0265
- ADR 0266
- ADR 0267
- ADR 0268
- ADR 0269
- ADR 0270

## Protocol And Offline Parity

- [x] Original protocol XML/DTD resources are loaded from `resources/protocol/`.
- [x] FinTS wire parsing/rendering tests cover quoted delimiters, binary
  blocks, segment resolution, sequence validation, and value extraction.
- [x] SEPA/CAMT/SWIFT/MT940 fixtures cover the current original-near parser and
  generator behavior.
- [x] Risky parser/generator behavior has Java golden artifacts or an explicit
  limitation entry.
- [x] Any malformed-bank-response behavior added before v1 has deterministic
  replay or fixture coverage.

Evidence:

- `tests/protocol_resources.rs`
- `tests/protocol_wire.rs`
- `tests/protocol_message.rs`
- `tests/sepa.rs`
- `tests/swift.rs`
- `tests/structures.rs`
- `tests/bootstrap.rs`
- `tests/status.rs`
- `tests/secmech.rs`
- `tests/runtime_callbacks.rs`
- `tests/fixtures/hbci4java/`
- `docs/reference/parser-generator-goldens.md`
- `docs/reference/malformed-bank-responses.md`
- ADR 0254
- ADR 0255

## PinTAN Runtime Replay Breadth

- [x] Dialog init, execution, and close are replay-tested.
- [x] Signed HBCI-Plus/PinTAN messages are replay-tested.
- [x] One-step TAN, process-1, and process-2 flows are replay-tested.
- [x] SCA exemption, TAN media selection, decoupled polling, QR-TAN, and
  photoTAN callback emission are replay-tested.
- [x] Failed process-2 submission state and process-2 transport retry state are
  replay-tested.
- [x] Error/reporting behavior is reviewed for user-facing API clarity.
- [x] Additional bank-specific SCA variants discovered during live smoke tests
  are converted into offline replay fixtures or documented as limitations.

Evidence:

- `tests/bootstrap.rs`
- `tests/runtime_callbacks.rs`
- `tests/live_bank.rs`
- `tests/status.rs`
- `tests/public_api.rs`
- `docs/reference/error-reporting.md`
- `docs/reference/live-bank-tests.md`
- ADR 0250
- ADR 0257

## Public API And Migration Docs

- [x] Public Rust names use Rust casing while job names and property keys stay
  Java-compatible.
- [x] Callback reasons expose original hbci4java code mappings where ported.
- [x] Common PinTAN runtime setup and a balance-request migration shape are
  documented.
- [x] API docs are reviewed for the crate-root exported v1 types.
- [x] At least one migration example is checked against the current public API.
- [x] Per-job migration examples are added for the highest-risk transfer and
  statement workflows.

Evidence:

- `README.md`
- `src/lib.rs`
- `docs/reference/public-api.md`
- `docs/reference/java-to-rust-mapping.md`
- `docs/reference/migration-examples.md`
- `tests/public_api.rs`
- `tests/bootstrap.rs`

## Passport Storage And Security

- [x] v1 uses the Rust-native encrypted PinTAN passport format.
- [x] Storage tests cover roundtrips of persistent data.
- [x] Runtime PIN caching and clearing are tested.
- [x] Release candidate reviews KDF, AEAD, and envelope metadata against ADR
  0008 and current dependency docs.
- [x] A persisted-format migration test is added before the first storage
  format revision.

Evidence:

- `src/passport/storage.rs`
- `src/passport/pintan.rs`
- `tests/passport.rs`
- `tests/fixtures/passport/pintan-v1-envelope.json`
- `docs/reference/passport-storage-security.md`
- ADR 0008
- ADR 0251
- ADR 0256

## License, Packaging, And Metadata

- [x] Repo-level LGPL-2.1-or-later assumption is recorded.
- [x] `LICENSE` and `NOTICE` exist.
- [x] Upstream header inconsistencies are rechecked before publishing.
- [x] `Cargo.toml` package metadata is reviewed for crate publication.
- [x] Current `cargo package --list` output is reviewed and documented.
- [x] Generated or copied upstream artifacts are documented with attribution.
- [x] Final release-candidate package checks are rerun after the last release
  commit.

Evidence:

- `LICENSE`
- `NOTICE`
- `Cargo.toml`
- `docs/reference/packaging.md`
- `docs/reference/upstream-header-review.md`
- `resources/protocol/README.md`
- `tests/fixtures/hbci4java/README.md`
- ADR 0002
- ADR 0249
- ADR 0253

The final packaging pass can be run with:

```sh
scripts/run-release-candidate-checks.sh --package
```

## Optional Live Smoke

- [x] Live PinTAN smoke testing is ignored and environment-gated.
- [x] The live test command exits safely without credentials by default.
- [x] Manual live-bank observations, if any, are recorded without credentials.
- [x] Live observations are converted into deterministic replay fixtures or
  explicit limitations before they influence acceptance.

Evidence:

```sh
cargo test --test live_bank -- --ignored
```

- `docs/reference/live-bank-tests.md`
- ADR 0257

## Release Declaration

Do not declare v1 complete until:

- all required offline gates pass on the release-candidate commit;
- all source-surface audits show only intentional v1 exclusions;
- all blocking unchecked items above are resolved;
- any remaining limitations are documented in ADRs and public docs;
- `docs/architecture/v1-readiness.md` is updated with the release-candidate
  evidence snapshot.
