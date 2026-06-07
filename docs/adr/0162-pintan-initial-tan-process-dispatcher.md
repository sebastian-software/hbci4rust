# ADR 0162: PinTAN Initial TAN Process Dispatcher

## Status

Accepted

## Context

The port now has explicit helpers for process variant 1 (`TAN2Step.process=1`), process variant 2
step 1 (`TAN2Step.process=4`), and process variant 2 step 2 (`TAN2Step.process=2`). hbci4java reads
the selected TAN mechanism's BPD `process` field and maps it through `KnownTANProcess`: variant
`1` starts with process code `1`, variant `2` starts with process code `4`, and missing or unknown
variant codes default to variant 2.

Without an initial dispatcher, callers of the Rust port must duplicate this BPD decision.

## Decision

Add initial HKTAN helpers that choose the first HKTAN job from the current PinTAN mechanism:

- `HbciHandler::new_tan2step_initial_job(&self, task: &HbciJob, challenge_info:
  Option<&ChallengeInfo>) -> HbciResult<HbciJob>`;
- `HbciHandler::new_tan2step_initial_job_with_tan_media_selection(&mut self, task: &HbciJob,
  challenge_info: Option<&ChallengeInfo>) -> HbciResult<HbciJob>`;
- when current BPD `process` is exactly `1`, delegate to the process-1 helper;
- for `process=2`, unknown, empty, or absent values, delegate to the process-2 step-1 helper;
- preserve the existing process-1 order-hash, order-account, challenge-parameter, and TAN-media
  behavior;
- preserve the existing process-2 step-1 minimal field behavior.

## Consequences

The handler now has an original-near front door for the first HKTAN of a TAN-protected task. This
keeps automatic queue patching as a later step while reducing duplicated process-selection logic in
tests and future callers.
