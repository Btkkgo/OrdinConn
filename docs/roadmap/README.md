# Public Roadmap

[English](README.md) | [简体中文](README.zh-CN.md)

> Current M3: saved Google Gemini / `gemini-3.6-flash`, Provider Count **1**. The new paced Planner-only batch passed **1/5** (3 calls executed, 2 not run; 10 attempts, 6 HTTP 429, 2 HTTP 503, 1 HTTP 200 and 1 deadline-interrupted attempt). Bounded retry regression PASS; real acceptance stopped as **FREE_TIER_RATE_LIMIT_BLOCKED**; Full Autonomous M3 remains **NOT_COMPLETE**. Earlier Phase 2–5 counts below are historical. Deployment is **PERSONAL / LOCAL-ONLY APPLICATION**; Apple signing/distribution is **OUT_OF_SCOPE_LOCAL_ONLY**.


Roadmap entries are directions, not completion claims.

## Current gate

M1.5 and M2 passed against the dedicated `OrdinConn_M1_5` Android 16 ARM64 AVD. M2's 30-item acceptance includes real bounded navigation, safety negatives, sanitized receipts, and a packaged-desktop Rust → Tauri → React manual Tap. See [Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7) and the [M2 acceptance record](../mobile/M2_ACCEPTANCE.md). M3 Phase 2/3 foundations and Phase 4 bounded single-step execution are locally implemented; see [M3 Executor](../mobile/M3_EXECUTOR.md). Full autonomous M3 is NOT_COMPLETE: live provider availability/rate limiting requires validation; Phase 5 Stop/Approval integration is implemented.

The completed gate is recorded in [GitHub Issue #1](https://github.com/Btkkgo/OrdinConn/issues/1). Intermittent process-fixture concurrency timeouts remain tracked separately in [Issue #4](https://github.com/Btkkgo/OrdinConn/issues/4).

## M2 — Verified navigation

Verified on an emulator after M1.5: bounded tap, swipe, type, back/home, and application navigation with preconditions, policy checks, action receipts, post-action observation, and verification. This does not authorize autonomous App Skills or physical devices.

## M3 — Production App Skill

Implement one narrowly scoped, allowlisted App Skill with deterministic navigation rules, recovery, budgets, and auditability.

## M4 — Evidence promotion

Add an explicit `MobileObservation -> Evidence` policy through Connector Registry, then allow eligible Evidence to enter Strategy and `SignalCandidate` evaluation.

## M5 — Physical Android devices

Generalize the verified runtime from Emulator to explicitly authorized real devices without weakening permissions, privacy, or audit controls.

## Longer-term runtime direction

- Broader desktop Accessibility and event-driven perception
- Explicit Work Sessions and application allowlists
- Additional model adapters behind Model Gateway
- Additional public sources behind Connector Registry
- Local embedded runtime improvements within the personal-use boundary

Computer Runtime roadmap research is tracked in [GitHub Issue #3](https://github.com/Btkkgo/OrdinConn/issues/3).

## Mobile Interaction and Data Acquisition — 2026-10-02

[Issue #14](https://github.com/Btkkgo/OrdinConn/issues/14) adds a manual, local-only `Observe → Interact → Observe → Diff → Extract → Data Object → Data Stream → Provenance` loop on the existing Android Runtime and SQLite database. The lifecycle is `Source → Observation → Extraction → Data Object → Insight → Plan → Action → Result`; this stage implements through **Data Object**, with Insight/Plan reserved as interfaces. Home uses actual collected objects, an element inspector/manual controls and deterministic Observation Context. Sensitive values are redacted before persistence; deterministic deduplication retains repeated sightings and original evidence. Automated fixtures are verified separately from **PENDING HUMAN ACCEPTANCE**. Production actions retain the existing emulator/Settings safety surface.

M3 status/history, providers, retry/backoff/deadline, Keychain and the retained app are preserved. Automated real Android actions **0**; manual Android actions by Codex **0**; real Provider requests **0**; signing verification **0**; X Draft **NONE**. Rust/frontend compilation is allowed; macOS acceptance packaging is **NOT RUN / BLOCKED BY THE NO-SIGNING BOUNDARY** because the supported wrapper invokes `codesign`. See [implementation and owner acceptance](../mobile/MOBILE_DATA_ACQUISITION.md).
