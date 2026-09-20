# Current Status

[English](CURRENT_STATUS.md) | [简体中文](CURRENT_STATUS.zh-CN.md)

- Date: 2026-09-20
- Version: 0.1.0
- Official repository: https://github.com/Btkkgo/OrdinConn
- Current Mobile gate: https://github.com/Btkkgo/OrdinConn/issues/1
- Default public branch: `main`
- Product source baseline: `f8705c5`
- Mobile stage: M1.5 real-environment validation complete
- Gate: **M1.5 PASS**
- Next phase: **M2 NOT STARTED**

This document separates implementation, verification, partial work, blocked work, design, plans, and work that has not started. Written intent is never counted as runtime evidence.

## Implemented

- Local-first Tauri desktop shell with React and typed IPC.
- Transport-independent Rust crates for market, Evidence, Signal, strategy, model, agent, approval, execution, connector, and tool domains.
- Model Gateway with OpenAI-compatible and deterministic mock adapters.
- Connector Registry, public REST/WebSocket/feed/HTML collectors, scheduling, bounded rolling history, deterministic strategies, and Evidence clusters.
- Evidence-gated Signals, contextual Agent reports, exact approval capabilities, and Paper Execution only.
- Mobile M0/M1 domain contracts for device sessions, frames, semantic UI snapshots, element references, privacy classification, and `MobileObservation`.
- Observe-only Android adapter code for tool discovery, existing-AVD inspection, online-emulator discovery, bounded frame capture, UI-tree dump, sensitive-node redaction, persistence, events, and workspace projection.
- Public engineering documentation, issue templates, manual X drafts, and a fail-closed public repository gate.
- A two-hour macOS LaunchAgent for public GitHub synchronization, using a dedicated TCC identity and an Application Support runner without requiring Codex or ChatGPT to be open.

## Verified

At product source baseline `c4f2d66`, the recorded full local verification was:

- 126 Rust tests passed.
- 31 TypeScript tests passed.
- Rust formatting and Clippy with warnings denied passed.
- TypeScript typecheck and the Vite production build passed.
- The macOS Tauri application bundle was produced.

On the corrective Issue #1 branch, the final default-parallel Rust workspace rerun passed 125 Rust tests. All 31 TypeScript tests, TypeScript typecheck, Vite build, macOS Tauri bundle, rustfmt, and Clippy with warnings denied also passed.

The explicit real Android smoke passed all ten checks against `OrdinConn_M1_5`, a Pixel 8 profile using the Android 36 Google APIs ARM64 image. The online device reported Android 16 / API 36 at 1080×2400 and 420 dpi. The final production rerun captured a 188,909-byte PNG, parsed 70 sanitized UI elements, generated 70 snapshot-bound references and a `MobileObservation`, passed persisted/audited workspace projection, and closed the logical session.

Typed IPC has separate real GUI evidence. In the packaged Tauri application, the empty allowlist produced a visible controlled error. After allowlisting `com.android.settings`, the React UI showed `Observing`, `emulator-5554`, the package name, `VERIFIED`, a real frame, and 70 UI elements. The new Stop Session command returned the UI to `Disconnected`; one session persisted the complete start/snapshot/observation/end event sequence, and the operator-owned AVD remained online.

A separate temporary local test application exposed one real `password=true` node. UIAutomator did not emit the test plaintext, OrdinConn recorded one redaction, and serialized capture data did not contain the test value. The app and temporary artifacts were removed after validation.

For the open-source initialization, public-log sanitizer, allowlisted Git sync, scheduler rendering, document validation, plist lint, TypeScript tests/typecheck, Vite build, and Tauri bundle passed. During the corrective run, the first default-parallel workspace attempt exposed two intermittent AVD lifecycle fixture failures; after an unrelated gate-label assertion was corrected, the complete default-parallel workspace rerun passed and all 28 desktop tests passed serially. The nondeterminism remains tracked in Issue #4 rather than being erased by the green rerun.

The independent GitHub scheduler is verified on macOS: `launchctl` loaded `com.ordinconn.github-sync` with a 7,200-second interval, the dedicated launcher received Documents access, and its first background run completed a no-change scan without Codex involvement. The final change/push and second no-change acceptance are recorded in Issue #2 and the DevLog.

## Partial

- Computer Runtime interfaces and permission concepts exist, but broad production computer operation is not implemented.

## Blocked

- No M1.5 product gate remains blocked.
- No product acceptance item is blocked. Issue #4 remains open because the desktop process-fixture tests showed intermittent timeout failures under default concurrency even though the final workspace rerun passed.

## Designed

- Structured perception before visual inference.
- API access before GUI automation when an appropriate API exists.
- Event-driven perception before continuous capture.
- M2 verified navigation with action receipts and post-action verification.
- M3 production App Skills, M4 `MobileObservation → Evidence` promotion, and M5 physical Android devices.
- Standalone or remote runtime hosts that reuse the same domain protocols.

Designed items are not current product capabilities.

## Planned

- Resolve the concurrent process-fixture timeouts in Issue #4 without weakening production command deadlines or the real acceptance gate.
- Continue issue-first public engineering records in GitHub.

## Not Started

- Mobile M2 action execution and verified navigation.
- Production App Skills for third-party mobile applications.
- Automatic promotion of mobile observations to Evidence.
- Physical Android device support.
- Real-money execution.

## Gate Rule

M1.5 can pass only after Android SDK, ADB, Emulator, AVD, online device, real smoke, frame capture, UI tree, sensitive redaction, `MobileObservation`, Snapshot Parse, Element Refs, Tauri IPC, and Session Shutdown are all exercised successfully. Installing an SDK alone is insufficient.

## Next

Complete owner review of the M1.5 evidence; do not start M2 until a new explicit instruction authorizes it.
