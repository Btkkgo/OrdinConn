# Current Status

[English](CURRENT_STATUS.md) | [简体中文](CURRENT_STATUS.zh-CN.md)

> Current Phase 5: bounded Planner/Executor integration is implemented. Live model acceptance is **BLOCKED_MODEL_NOT_CONFIGURED** (Production Provider Count **0**); Full Autonomous M3 is **NOT_COMPLETE**. Earlier Phase 2–4 validation records below remain historical evidence. See the [Phase 5 closeout record](mobile/M3_PHASE5.md) for current validation.


- Date: 2026-09-27
- Version: 0.1.0
- Official repository: https://github.com/Btkkgo/OrdinConn
- Current Mobile gate: https://github.com/Btkkgo/OrdinConn/issues/1
- Default public branch: `main`
- Working branch: `codex/realtime-workbench`, base `cc1d0c9`; includes retained Issue #11 prerequisites and Issue #12 Phase 2–5 integration.
- Mobile stage: M1.5 real-environment validation complete
- Gate: **M1.5 PASS**
- Mandatory acceptance: **15 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN**
- Runtime reliability follow-up: **Issue #4 CLOSED / VERIFIED**
- Current phase: **M2 VERIFIED — 30 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN** ([Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7); [acceptance](mobile/M2_ACCEPTANCE.md))
- Local M3 foundation: **PHASE 5 IMPLEMENTED / NEEDS LIVE VALIDATION**; full autonomous acceptance **NOT_COMPLETE**; current validation is recorded in the Phase 5 closeout.

- Workbench: **IMPLEMENTED / NEEDS VALIDATION**, [Issue #11](https://github.com/Btkkgo/OrdinConn/issues/11). Autonomous custom-goal wiring is implemented; live acceptance remains `BLOCKED_MODEL_NOT_CONFIGURED`.
- Operator Experience: **PENDING USER ACCEPTANCE**, [Issue #10](https://github.com/Btkkgo/OrdinConn/issues/10).

This document separates implementation, verification, partial work, blocked work, design, plans, and work that has not started. Written intent is never counted as runtime evidence.

## Implemented

- Reference-driven dark Home workbench, real loaded-object metrics, typed commands, persisted pending research-task projection, and an explicitly opted-in development visual fixture. Existing Warehouse, Settings, detail discussions, and manual mobile inspector remain accessible.

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

Issue #11 verification: 60 TypeScript tests/typecheck PASS; 165 Rust tests PASS serially, with retained default-parallel AVD fixture failures. Real packaged Observe/Collect/Stop PASS. See the [workbench acceptance record](mobile/WORKBENCH_ACCEPTANCE.md); this is not autonomous task execution or owner UX acceptance.

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

The independent GitHub scheduler is verified on macOS: `launchctl` loaded `com.ordinconn.github-sync` with a 7,200-second interval, the dedicated launcher received Documents access, and its first background run completed a no-change scan without Codex involvement. The final change/push and second no-change acceptance are recorded in Issue #2 and the DevLog.

## Partial

- Computer Runtime interfaces and permission concepts exist, but broad production computer operation is not implemented.
- Computer Runtime remains partial; M2's real action gate is separately verified above.

## Blocked

- No M1.5 product gate remains blocked.
- Issue #11 owner UX acceptance remains separate. Phase 5 now wires Home goals to the bounded Rust runner, but Provider 0 blocks live Planner + Executor acceptance. Model claims never replace typed completion evidence.
- Historical Issue #4 failures remain documented; its completed reliability verification is unchanged.

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

- Full autonomous M3 navigation and its production App Skills (Phase 4 explicit single-step execution is implemented separately).
- Automatic promotion of mobile observations to Evidence.
- Physical Android device support.
- Real-money execution.

## Gate Rule

M1.5 can pass only after Android SDK, ADB, Emulator, AVD, online device, real smoke, frame capture, UI tree, sensitive redaction, `MobileObservation`, Snapshot Parse, Element Refs, Tauri IPC, and Session Shutdown are all exercised successfully. Installing an SDK alone is insufficient.

## Next

Configure exactly one production provider/default model in existing Settings, then run the safe two-decision live model/Planner/Executor gate. Review the Issue #11 workbench and complete owner-led feedback in #10 separately. Full Autonomous M3 remains NOT_COMPLETE; #12 stays OPEN / status:needs-validation. No X draft or publication.

## M3 Phase 2 — persistent execution contracts (historical)

[Issue #12](https://github.com/Btkkgo/OrdinConn/issues/12) tracks the full M3 foundation and remains OPEN / NEEDS VALIDATION. Phase 1 audit is complete; **Phase 2 technical acceptance PASS**. The accepted realtime workbench visual design is preserved.

Implemented and verified: canonical Rust Goal/Plan/Step IDs and state machines, original-objective persistence, immutable revision history, budget/error/risk/expected-result contracts, observation/action/evidence/result references, transactional lifecycle audit, fail-closed restart recovery, deterministic MobileApprovalSubject hash, and four read/create typed IPC commands. Home input now creates a persistent PENDING MobileGoal and displays Waiting for planning without fabricating a Plan. Other ResearchTasks remain intact. See [M3 contracts](mobile/M3_CONTRACTS.md).

Phase 2 recorded validation: **203 Rust PASS / 0 FAIL** under default parallelism (30 new repository, 6 domain, 2 IPC tests; existing Trade Approval regression preserved); **59 Desktop TypeScript PASS**, **9 Contracts PASS**; typecheck, frontend build, Rust workspace build, Tauri release packaging, rustfmt, public security gate and git diff --check PASS. Legacy migration preserves audit, observations, saved/favorite mobile feed, ResearchTasks and settings. Synthetic contract tests are not real-device execution evidence.

The Phase 2 record above is historical. Phase 3 is historical; the current Phase 4 state is below; the existing chat MockModelAdapter remains unchanged and excluded from M3. Prior AVD fixture timing failures remain KNOWN_TEST_INFRA_LIMITATION. #11 stays OPEN. Commit NONE / Push NONE.

## M3 Phase 3 — explicit real Gateway planner (historical)

The real Model Gateway production path, unique enabled-provider/default-model resolution, strict single decision schema, bounded context/output/timeout/retry, local domain/safety validation and risk recalculation, completion-only proposals, transactional Plan/one pending Step and metadata events are implemented. `plan_mobile_goal` is explicit typed IPC; Home submission does not call a model. The visual design is preserved; the existing panel reports waiting for execution. See [M3 Planner](mobile/M3_PLANNER.md).

Fresh validation: **239 Rust PASS / 0 FAIL** on the final serial workspace run; **60 Desktop TypeScript PASS**, **9 Contracts PASS**, **9 Model Gateway PASS**, **30 Phase 2 repository PASS**, **6 Phase 2 domain PASS**, original Trade Approval regression PASS. Typecheck, frontend build, Rust workspace build and rustfmt PASS. Tauri release packaging PASS. Four default-parallel workspace attempts and one earlier full serial attempt reproduced `concurrent_avd_lifecycle_fixtures_remain_independent_during_slow_tool_startup`. The latest default-parallel attempt also failed `command_returns_after_success_when_descendant_keeps_stdout_open`, ending at **206 PASS / 2 FAIL** before remaining suites. Final full serial passed **239/239**, including desktop **47/47**, after competing compilation ended. The earlier default-parallel **236/236** was on pre-audit code. This retains the known fixture limitation without changing its timeout or permanently serializing tests. No real AVD was used.

Production Provider Count **0**, checked read-only through the shared planner gate: **MODEL_NOT_CONFIGURED PASS**, no provider setup, secrets, production migration or fake Plan/Step. Native Schema and JSON-only modes pass local HTTP/isolated SQLite tests; **Live Provider Gate NOT_RUN_MODEL_NOT_CONFIGURED**. Full M3 remains OPEN / status:needs-validation in Issue #12. Autonomous execution **NOT_IMPLEMENTED**, Mobile Action executed **NO**, Commit **NONE**, Push **NONE**. Stop at Phase 3; no Phase 4 executor or Phase 5 approval execution is started.

## M3 Phase 4 — bounded single-step Rust executor (historical)

**Phase 4 Technical Acceptance PASS** for the implemented executor and real Settings acceptance; full autonomous M3 remains **NOT_COMPLETE**. Production Rust `execute_mobile_goal_step` runs exactly one persisted pending Step. Device leases, fresh Observe-before, local semantic action/policy, existing typed M2 action, real Observe-after, typed Verification, immutable Result/trace and fail-closed restart/idempotency are implemented. Approval-required Steps become WAITING_APPROVAL without execution; forbidden Steps return POLICY_BLOCKED. The existing cards refresh persisted Step states and extraction metrics; no new controls or autonomous invocation were added. See [M3 Executor](mobile/M3_EXECUTOR.md).

Fresh independent dedicated-AVD gate **PASS**, using an isolated test database and **TEST_ONLY** deterministic planner through the production executor: **BACK PASS / ACTIVITY_EQUALS**, **SCROLL_DOWN PASS**, real Observe → Action → Observe → Verify, supported one-scroll completion, duplicate protection, **EXTRACT PASS** with real object/Evidence/projection links. **INPUT_TEXT NOT_AVAILABLE_TEST_SURFACE**; exact value/false-success synthetic input verification PASS. Earlier failed focus/target/harness attempts remain recorded; bounded focus reads never replay an action.

Earlier default-parallel Rust **272 PASS / 0 FAIL**; final default **240 PASS / 1 FAIL / KNOWN_TEST_INFRA_LIMITATION**; final full serial Rust **272 PASS / 0 FAIL**; desktop **52/52**, executor service **18/18**, executor domain **9/9**, source registration **1/1**, Phase 3 planner service **20/20** and domain **10/10**, Model Gateway **9/9**, Phase 2 repository **30/30** and domain **6/6**, original Trade Approval regression PASS. Default runs leave real AVD gates inactive; the independent enabled real gate passed separately. Prior Phase 3 concurrency failures remain historical, with no timeout increase or forced serialization in this phase. Desktop TypeScript **63/63**, Contracts **9/9**, typecheck and frontend build PASS. Rust workspace build, macOS Tauri release app packaging, rustfmt and public security/docs/diff gates PASS.

Production Provider Count **0**, freshly read-only checked without production migration/configuration/key access. Live Planner + Executor **NOT_RUN_MODEL_NOT_CONFIGURED**. #12 remains OPEN / status:needs-validation, #11 OPEN; #10 owner acceptance is unchanged. Stop after Phase 4. Phase 5 integration is implemented; full live AI acceptance remains blocked. Commit **NONE**, Push **NONE**; no X draft/publication.


Final parallel rerun: after expanding lease panic/poison and non-next-claim assertions, final `cargo test --workspace` stopped at **240 PASS / 1 FAIL** on the existing `mobile::tests::concurrent_avd_lifecycle_fixtures_remain_independent_during_slow_tool_startup` (Fixture_AVD_3 AvdBootTimeout); remaining suites were not run. The earlier complete Phase 4 default run passed **272/272**, but the final current default result is **FAIL / KNOWN_TEST_INFRA_LIMITATION**, not PASS. No timeout increase, forced global serialization or shared real AVD contention was introduced. Final complete serial results are recorded separately.
