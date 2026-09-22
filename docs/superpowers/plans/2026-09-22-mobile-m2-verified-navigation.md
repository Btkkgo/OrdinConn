# Mobile M2 Verified Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

[English](2026-09-22-mobile-m2-verified-navigation.md) | [简体中文](2026-09-22-mobile-m2-verified-navigation.zh-CN.md)

**Goal:** Add six bounded, manual Android Emulator navigation actions with fail-closed policy, sanitized receipts, fresh post-action observation, and deterministic verification.

**Architecture:** `mobile-runtime` owns platform-neutral action contracts and policy. The existing Tauri `MobileHost` owns ADB execution and in-memory capture state; `ordinconn-app` owns receipts and audit persistence. React only invokes typed IPC after a human click.

**Tech Stack:** Rust, Android ADB/UIAutomator, SQLite/sqlx, Tauri, React/TypeScript.

**Spec:** [Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7) and the owner-authorized M2 brief in this task. The brief is the binding source for the complete acceptance matrix.

## Global Constraints

- Only Tap by current Element Ref, bounded directional Swipe, safe Type, Back, Home, and allowlisted Open App.
- Emulator only; maximum 20 executed actions per session; no arbitrary coordinates, ADB/shell/intent API, physical devices, credentials/OTP, financial operations, autonomous Agent action loop, or M3.
- `GenericAndroidSkill::v1()` stays observe-only. Navigation capability does not itself execute an action.
- Type has at most 256 Unicode scalar values, no control/newline/shell-unsafe characters, and is never persisted or logged in plaintext. Receipts record length and SHA-256 only.
- Every attempted action produces a sanitized receipt; every executed action must be followed by fresh production observation. `NO_CHANGE` is not `VERIFIED`.
- Default tests never touch an emulator. Real tests require `ORDINCONN_MOBILE_M2_SMOKE=1` and the dedicated `OrdinConn_M1_5` AVD.
- Public records are English-first/Chinese-second; no X draft or publication; M3 remains not started.

## Review Focus

- A stale snapshot ref after another observation must block without issuing an ADB input command; Task 2 tests this.
- A foreground app changing between preflight and input must block before the input; Task 2 tests this.
- A password element whose visible text is redacted but whose class remains editable must still block Type; Task 1 tests this.
- An ADB input error or failed post-capture must not be recorded as verified; Task 2 tests this.
- Unicode and shell-metacharacter Type text must be rejected before ADB; Task 1 tests this.

---

### Task 1: Domain contracts and fail-closed policy

**Files:** Create `crates/mobile-runtime/src/action.rs`; modify `crates/mobile-runtime/src/lib.rs`; test in `action.rs`.

**Interfaces:** Consume `MobileDeviceSession`, `MobileUiSnapshot`, `MobileObservation`, `VerificationResult`. Produce `MobileActionRequest`, `MobileActionTarget`, `MobileActionPolicy`, `MobileActionDecision`, `MobileActionReceipt`, `evaluate_action`, `verify_action`.

- [ ] Write tests constructing a connected emulator session, current snapshot, and six typed targets. Assert allow for safe cases and deny codes for inactive/physical/budget/package/foreground/stale/sensitive/missing/disabled/noneditable/invalid Type/invalid Swipe/invalid Open App. Assert serialized request and receipt omit Type plaintext.
- [ ] Run `cargo test -p mobile-runtime action::tests -- --nocapture`; expect RED because action module and types do not yet exist.
- [ ] Implement the exact public enums/structs and ordered checks. Use `#[serde(skip_serializing, skip_deserializing)]` for Type plaintext only if an IPC carrier is separate; otherwise make the action request non-serializable for plaintext and provide a typed transport request whose Debug redacts text. Validate without constructing arbitrary command fragments.
- [ ] Rerun the named tests; expect PASS. Run `cargo fmt --all -- --check` and `cargo clippy -p mobile-runtime --all-targets -- -D warnings`; expect PASS.
- [ ] Commit `feat(mobile): add bounded M2 action policy Refs #7`.

### Task 2: Android adapter, receipt, and post-observation

**Files:** Modify `apps/desktop/src-tauri/src/mobile.rs`; test in its module.

**Interfaces:** Consume Task 1 policy and request; produce `MobileHost::execute_action` returning a sanitized receipt and optional fresh `MobileCapture`.

- [ ] Write fixture tests with fake ADB: stale ref and changed foreground issue no input command; safe action emits one bounded input; post-capture failure yields non-verified receipt. Guard real smoke with `ORDINCONN_MOBILE_M2_SMOKE=1`.
- [ ] Run `cargo test -p ordinconn-desktop mobile_action -- --nocapture`; expect RED for missing execution method.
- [ ] Keep the last capture and action count under one mutex. Recheck device/foreground before command, resolve Tap center from the current ref, invoke only fixed ADB input verbs, then call the production capture path. Back/Home use fixed keycodes; Open App resolves launcher for an allowlisted package. Never set `VERIFIED` without post-capture evidence.
- [ ] Rerun named tests and `cargo clippy -p ordinconn-desktop --all-targets -- -D warnings`; expect PASS.
- [ ] Commit `feat(mobile): execute guarded emulator navigation Refs #7`.

### Task 3: SQLite receipts and sanitized audit events

**Files:** Modify `crates/ordinconn-app/src/mobile_intelligence.rs`, existing migrations in `crates/ordinconn-app/src`, and `crates/ordinconn-app/tests/mobile_intelligence.rs`.

**Interfaces:** Consume Task 1 receipt and Task 2 optional post-capture; produce `AppRuntime::record_mobile_action` and latest receipt in `MobileWorkspaceData`.

- [ ] Test executed and blocked receipt persistence with four event types `mobile.action_requested`, `mobile.action_blocked`, `mobile.action_executed`, `mobile.action_verified`. Assert no Type plaintext in DB, event payload, or serialized workspace.
- [ ] Run `cargo test -p ordinconn-app mobile_action -- --nocapture`; expect RED for missing record method/schema.
- [ ] Add minimal schema and atomic receipt/event writes; keep prior M1.5 snapshot persistence intact.
- [ ] Rerun named tests and `cargo clippy -p ordinconn-app --all-targets -- -D warnings`; expect PASS.
- [ ] Commit `feat(mobile): persist sanitized action receipts Refs #7`.

### Task 4: Typed IPC and manual React controls

**Files:** Modify `apps/desktop/src-tauri/src/commands.rs`, `lib.rs`, `packages/contracts/src/index.ts`, `apps/desktop/src/runtime/client.ts`, `apps/desktop/src/pages/MobileHomePage.tsx`, `apps/desktop/src/components/MobileDeviceView.tsx`, both locale dictionaries, and focused tests.

**Interfaces:** Consume Task 2/3 result; produce `execute_mobile_action` returning receipt and updated `MobileWorkspaceData`; React uses only this typed command.

- [ ] Test the typed request/result and UI disabled state without live ADB. Assert action UI is manual and Type input clears after submission.
- [ ] Run `npm test -- --run`; expect RED for missing client/action UI.
- [ ] Add command and typed client, then minimal controls for selected Element Ref, Swipe, Type, Back, Home, and allowlisted Open App. Rust revalidates all UI-gated conditions.
- [ ] Run `npm test -- --run`, `npm run typecheck`, and `npm run build`; expect PASS.
- [ ] Commit `feat(mobile): add manual verified-navigation controls Refs #7`.

### Task 5: Real acceptance, regressions, and bilingual closeout

**Files:** Create `docs/mobile/M2_ACCEPTANCE.md` and `.zh-CN.md`; update bilingual Current Status, Roadmap, Mobile Intelligence, DevLog, and relevant Problems/Field Notes; update Issue #7.

**Interfaces:** Consume all previous tasks and fresh real AVD results; produce an evidence matrix with truthful PASS/FAIL/BLOCKED/NOT RUN.

- [ ] Run the dedicated AVD Settings Tap/Back/Swipe/Type/Home/Open App flow plus stale/wrong-app/disallowed/sensitive/financial negative gates using `ORDINCONN_MOBILE_M2_SMOKE=1`. Record each actual result without treating fixtures as real evidence.
- [ ] Rerun M1.5 real smoke and Issue #4 parallel regression; record exact command, test count, and failures.
- [ ] Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, desktop default-parallel tests, `npm test -- --run`, `npm run typecheck`, `npm run build`, `npm run desktop:build`, public-doc validation, `scripts/security/check-public-repo.sh`, and `git diff --check`.
- [ ] Synchronize English/Chinese documents and Issue results. Only if all mandatory gates PASS: create bilingual PR, merge to main, and close Issue. Otherwise leave branch and Issue open with exact blockers. Never create or publish X content in this task.
