# Changelog

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md)

All notable public changes to OrdinConn will be recorded here from the open-source initialization forward. Earlier development history remains available in Git, but no unreconstructed release history is claimed.

## Unreleased

### Added

- M3 bounded Planner/Executor integration with persistent budgets, Stop priority, bound Approval and owner-verified completion. Live-model acceptance remains BLOCKED_MODEL_NOT_CONFIGURED; Full Autonomous M3 is NOT_COMPLETE. See [Phase 5](docs/mobile/M3_PHASE5.md).
- Public issue-first development workflow and GitHub templates.
- English and Chinese project entry documentation.
- Public status, DevLog, ADR, security, and Codex field-note structure.
- Fail-closed working-tree and Git-history security gate.
- Documentation-only scheduled GitHub synchronization.
- Manual X draft workflow with automatic publication disabled.
- Bilingual Build in Public language policy, paired core documentation, and bilingual Issue records.

## Mobile Interaction and Data Acquisition — 2026-10-02

[Issue #14](https://github.com/Btkkgo/OrdinConn/issues/14) adds a manual, local-only `Observe → Interact → Observe → Diff → Extract → Data Object → Data Stream → Provenance` loop on the existing Android Runtime and SQLite database. The lifecycle is `Source → Observation → Extraction → Data Object → Insight → Plan → Action → Result`; this stage implements through **Data Object**, with Insight/Plan reserved as interfaces. Home uses actual collected objects, an element inspector/manual controls and deterministic Observation Context. Sensitive values are redacted before persistence; deterministic deduplication retains repeated sightings and original evidence. Automated fixtures are verified separately from **PENDING HUMAN ACCEPTANCE**. Production actions retain the existing emulator/Settings safety surface.

M3 status/history, providers, retry/backoff/deadline, Keychain and the retained app are preserved. Automated real Android actions **0**; manual Android actions by Codex **0**; real Provider requests **0**; signing verification **0**; X Draft **NONE**. Rust/frontend compilation is allowed; macOS acceptance packaging is **NOT RUN / BLOCKED BY THE NO-SIGNING BOUNDARY** because the supported wrapper invokes `codesign`. See [implementation and owner acceptance](docs/mobile/MOBILE_DATA_ACQUISITION.md).
