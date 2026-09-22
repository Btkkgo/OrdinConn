# Current Status

[English](CURRENT_STATUS.md) | [简体中文](CURRENT_STATUS.zh-CN.md)

- Date: 2026-09-22
- Version: 0.1.0
- Official repository: https://github.com/Btkkgo/OrdinConn
- Current Mobile gate: https://github.com/Btkkgo/OrdinConn/issues/1
- Default public branch: `main`
- Product source baseline: `f8705c5`
- Mobile stage: M1.5 real-environment validation complete
- Gate: **M1.5 PASS**
- Mandatory acceptance: **15 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN**
- Runtime reliability follow-up: **Issue #4 CLOSED / VERIFIED**
- Current phase: **M2 VERIFIED — 30 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN** ([Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7); [acceptance](mobile/M2_ACCEPTANCE.md))

This document separates implementation, verification, partial work, blocked work, design, plans, and work that has not started. Written intent is never counted as runtime evidence.

## Implemented

- Local-first Tauri desktop shell with React and typed IPC.
- Transport-independent Rust crates for market, Evidence, Signal, strategy, model, agent, approval, execution, connector, and tool domains.
- Model Gateway with OpenAI-compatible and deterministic mock adapters.
- Connector Registry, public REST/WebSocket/feed/HTML collectors, scheduling, bounded rolling history, deterministic strategies, and Evidence clusters.
- Evidence-gated Signals, contextual Agent reports, exact approval capabilities, and Paper Execution only.
- Mobile M0/M1 domain contracts for device sessions, frames, semantic UI snapshots, element references, privacy classification, and `MobileObservation`.
- Observe-only Android adapter code for tool discovery, existing-AVD inspection, online-emulator discovery, bounded frame capture, UI-tree dump, sensitive-node redaction, persistence, events, and workspace projection.
- M2's six manual emulator actions, fail-closed policy, bounded ADB adapter, post-action verification, sanitized SQLite receipts/audit, and typed Tauri/React controls.
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

For the open-source initialization, public-log sanitizer, allowlisted Git sync, scheduler rendering, document validation, plist lint, TypeScript tests/typecheck, Vite build, and Tauri bundle passed. During the corrective run, the first default-parallel workspace attempt exposed two intermittent AVD lifecycle fixture failures; after an unrelated gate-label assertion was corrected, the complete default-parallel workspace rerun passed and all 28 desktop tests passed serially. The nondeterminism was tracked in Issue #4 rather than being erased by the green rerun.

The M1.5 stage-close verification reproduced Issue #4 again: the first fresh default-parallel workspace run passed 24 of 28 desktop tests and failed four process/AVD timing-sensitive fixtures. The immediate serial desktop run passed 28/28, and the next complete default-parallel workspace rerun passed 125/125. At that checkpoint the issue remained open; it did not invalidate the separately completed 15/15 real M1.5 acceptance.

On the Issue #4 reliability branch, the first of ten pre-change default-parallel desktop runs again failed the same four tests at 24/28, while nine warm runs passed. A controlled four-fixture cold-start regression failed with the original one-second success budget and passed after separating bounded test-only success budgets from the unchanged 30/50 ms timeout checks. Default-parallel desktop then passed 20/20 runs (29/29 each), full workspace passed 10/10 runs (136 tests per run), and an eight-thread desktop run passed 29/29. The first real Android smoke correctly rejected an unallowlisted cold-boot Launcher; after opening Settings on the dedicated AVD, the unchanged smoke passed 10/10 checks. Production deadlines and runtime code did not change. Rust formatting, Clippy with warnings denied, 31 TypeScript tests, typecheck, Rust/Vite builds, and the Tauri bundle passed.

PR #6 merged this verified tree into `main` as `52e73ca`; a post-merge full workspace run and an isolated public-history security gate passed. Issue #4 is closed as verified. The formerly retained local-only privacy backup was deleted under explicit owner authorization before M2 work; the full local all-ref security gate now passes.

M2's Issue #7 branch has a real dedicated-AVD Settings flow covering Tap, Swipe, Back, Type, Home, OpenApp, post-action observations, receipt persistence, and audit events. Real stale-ref, wrong-package, unallowlisted-app, and temporary password-field Type attempts were blocked before ADB input. Synthetic English/Chinese financial and sensitive targets were blocked without connecting a financial app. Final review tightened same-activity UI-tree preflight, atomic write-ahead Pending Receipt/audit intent, action-specific verification, dynamic Home-package detection, session identity/budget rotation, and a fixed Settings-only production action surface; the real M2 smoke passed again after these changes. The final packaged Tauri UI completed manual inspector Tap through typed IPC and showed `executed · VERIFIED` with distinct pre/post Snapshot IDs; Stop Session returned to Disconnected while the AVD remained online. The M1.5 ten-check smoke, default-parallel desktop 44/44, complete Rust workspace, 30 desktop and five contracts TypeScript tests, formatting, Clippy, typecheck, Rust/Vite builds, and final macOS bundle passed. See the paired [M2 acceptance record](mobile/M2_ACCEPTANCE.md) for exact evidence and limits.

The first PR #8 merge commit failed the post-merge public-history identity gate, leaving M2 at 29 PASS / 1 FAIL. One authorized privacy repair replaced `af843783` with unsigned `5a19c25` using noreply Author/Committer metadata; the tree, ordered parents, message, timestamps, and product files remained identical. Reachable public `main` identity then passed. The old object remains GitHub-accessible and referenced by PR #8; this cache condition is separate from the public reachable-history gate. The final 30/30 statement depends on the paired acceptance record and the completed security checks, not on the pre-merge technical result alone.

The independent GitHub scheduler is verified on macOS: `launchctl` loaded `com.ordinconn.github-sync` with a 7,200-second interval, the dedicated launcher received Documents access, and its first background run completed a no-change scan without Codex involvement. The final change/push and second no-change acceptance are recorded in Issue #2 and the DevLog.

## Partial

- Computer Runtime interfaces and permission concepts exist, but broad production computer operation is not implemented.
- Computer Runtime remains partial; M2's real action gate is separately verified above.

## Blocked

- No M1.5 product gate remains blocked.
- No product acceptance item is blocked. The historical Issue #4 timeout failures remain documented; repeated default-parallel verification on the reliability branch now passes.

## Designed

- Structured perception before visual inference.
- API access before GUI automation when an appropriate API exists.
- Event-driven perception before continuous capture.
- M3 production App Skills, M4 `MobileObservation → Evidence` promotion, and M5 physical Android devices.
- Standalone or remote runtime hosts that reuse the same domain protocols.

Designed items are not current product capabilities.

## Planned

- Keep Issue #4 reliability coverage in the default-parallel test suite without weakening production command deadlines or the real acceptance gate.
- Continue issue-first public engineering records in GitHub.

## Not Started

- Mobile M3 production App Skills and any autonomous navigation.
- Automatic promotion of mobile observations to Evidence.
- Physical Android device support.
- Real-money execution.

## Gate Rule

M1.5 can pass only after Android SDK, ADB, Emulator, AVD, online device, real smoke, frame capture, UI tree, sensitive redaction, `MobileObservation`, Snapshot Parse, Element Refs, Tauri IPC, and Session Shutdown are all exercised successfully. Installing an SDK alone is insufficient.

## Next

Run one owner-led M2 Verified Navigation experience check: Observe → Select Element → Tap/Swipe/Type → Receipt → Post Observation → Verification. Stop at M2; do not start M3 or create/publish X content without a new owner request.
