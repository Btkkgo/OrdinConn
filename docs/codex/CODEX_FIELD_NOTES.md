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
