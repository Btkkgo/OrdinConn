# Codex Field Notes

[English](CODEX_FIELD_NOTES.md) | [简体中文](CODEX_FIELD_NOTES.zh-CN.md)

These notes describe observed engineering workflow, not a product endorsement.

## Current stage — Mobile Intelligence M0 to M1.5

### What worked well

- Repository-first inspection kept the implementation aligned with the current product baseline rather than older concepts.
- A written phase gate stopped mobile navigation work when the real Android environment was missing.
- Small contract tests made privacy redaction, observation lineage, IPC projection, and shutdown behavior concrete.
- A fresh review found issues that a green unit suite alone had not exposed: subprocess bounds, environment consistency, and production-path persistence.

### What went wrong

- The first M1.5 pass treated some production acceptance behavior too indirectly.
- Tests could prove controlled ADB parsing while still leaving the real Emulator path unavailable.
- Large prompts mix product direction, architecture, UI, security, and acceptance criteria; without explicit phase boundaries they can encourage premature implementation.

### Better prompting pattern

Use `Read -> State the phase boundary -> Name observable acceptance checks -> Implement one boundary -> Run the exact checks -> Review -> Stop at the gate`.

Ask for the evidence format in advance. “Show the ten named gate results” is more useful than “make Android work.”

### Token / context lesson

Persist the plan and decisions in repository documents. Re-reading one focused task brief is cheaper and more reliable than carrying an entire product history in every prompt.

### Verification lesson

A completion statement is only as strong as the command that proves it. Fixture tests, a successful build, and a real-device smoke answer different questions and must be reported separately.

### My current view of Codex

Codex is strongest as a bounded engineering collaborator: it can inspect a large codebase, preserve explicit constraints, write tests, and iterate across layers. It is weaker when success depends on unavailable hardware, ambiguous external state, or a prompt that asks several future phases to appear complete at once. Independent review and real-environment evidence remain necessary.

## Issue #4 — A green rerun is not a deterministic test

### What Codex initially assumed

After a serial pass and a later green parallel run, the process fixtures could have been dismissed as transient machine noise. That was not evidence of a repair.

### What repeated testing showed

The first fresh default-parallel desktop run failed the same four process/AVD cases at 24/28, while nine warm reruns passed. Each case passed 20 isolated runs. A controlled four-fixture cold-start test then failed under the original one-second success budget and passed with a bounded test-only budget. The important distinction was test timing versus production deadline behavior, not merely serial versus parallel execution.

### What changed after evidence

Codex kept the short failure-path timeout assertions and all production command deadlines unchanged, added independent concurrent fixture coverage, and required repeated desktop/workspace runs plus a real Android smoke. The first smoke correctly rejected an unallowlisted Launcher; only the later Settings foreground run passed. Both outcomes belong in the record.

## Issue #7 — Real navigation changed the answer

Fake ADB tests established command shape, but the dedicated Android 16 AVD revealed two different boundaries: Settings Search is a separate system package, and Android task reuse made a generic changed-frame check insufficient for OpenApp. The first real attempts were reported as failures; only after target-package verification and a fixed clear-top launch did the full gated flow pass. A separate packaged-app UI check caught an expected stale-snapshot denial before a fresh inspector selection produced `executed · VERIFIED`.

The useful Codex pattern was to preserve each failure's exact evidence, add a small failing regression, and rerun the real device after the fix. The easy mistake would have been to equate a passing fixture, a successful bundle, or any changed screen with M2 acceptance. A second small privacy test caught untrusted rejected identifiers in receipts before public documentation was finalized. Context was managed with a paired plan, narrow test commands, and a concise progress ledger rather than repeatedly reloading the full task brief. M3 and X remained outside the authorized scope.

## Issue #12 — M3 Phase 5 evidence boundaries

Read-only provider discovery confirmed zero production providers. Codex completed the bounded integration and retained the live-model blocker instead of substituting localhost fixtures for acceptance. A new approved-action Stop regression failed before the fix and passed after it. Real input preparation exposed a keyboard tutorial and failed observations; exact target-value verification preserved those failures. Unique native dump ownership now prevents stale XML reuse. The default workspace passed 287 tests; final builds and real-device results are tracked in [Phase 5](../mobile/M3_PHASE5.md). No owner UX acceptance or Full Autonomous M3 completion is inferred. Existing work and five JPEGs were preserved; private traces stay outside the public repository.

## Mobile Interaction and Data Acquisition — 2026-10-02

[Issue #14](https://github.com/Btkkgo/OrdinConn/issues/14) adds a manual, local-only `Observe → Interact → Observe → Diff → Extract → Data Object → Data Stream → Provenance` loop on the existing Android Runtime and SQLite database. The lifecycle is `Source → Observation → Extraction → Data Object → Insight → Plan → Action → Result`; this stage implements through **Data Object**, with Insight/Plan reserved as interfaces. Home uses actual collected objects, an element inspector/manual controls and deterministic Observation Context. Sensitive values are redacted before persistence; deterministic deduplication retains repeated sightings and original evidence. Automated fixtures are verified separately from **PENDING HUMAN ACCEPTANCE**. Production actions retain the existing emulator/Settings safety surface.

M3 status/history, providers, retry/backoff/deadline, Keychain and the retained app are preserved. Automated real Android actions **0**; manual Android actions by Codex **0**; real Provider requests **0**; signing verification **0**; X Draft **NONE**. Rust/frontend compilation is allowed; macOS acceptance packaging is **NOT RUN / BLOCKED BY THE NO-SIGNING BOUNDARY** because the supported wrapper invokes `codesign`. See [implementation and owner acceptance](../mobile/MOBILE_DATA_ACQUISITION.md).
