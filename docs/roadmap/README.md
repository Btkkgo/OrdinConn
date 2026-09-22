# Public Roadmap

[English](README.md) | [简体中文](README.zh-CN.md)

Roadmap entries are directions, not completion claims.

## Current gate

M1.5 and M2 passed against the dedicated `OrdinConn_M1_5` Android 16 ARM64 AVD. M2's 30-item acceptance includes real bounded navigation, safety negatives, sanitized receipts, and a packaged-desktop Rust → Tauri → React manual Tap. See [Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7) and the [M2 acceptance record](../mobile/M2_ACCEPTANCE.md). M3 has not started.

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
- Standalone and remote host options that reuse the same protocols

Computer Runtime roadmap research is tracked in [GitHub Issue #3](https://github.com/Btkkgo/OrdinConn/issues/3).
