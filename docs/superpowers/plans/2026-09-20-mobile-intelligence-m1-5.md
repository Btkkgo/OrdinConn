# Mobile Intelligence M1.5 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make OrdinConn truthfully discover and diagnose an existing local Android SDK/AVD runtime, start an existing AVD with bounded readiness checks, and run the real M1 acceptance gate without adding M2 actions when the real runtime is unavailable.

**Architecture:** Keep Android process discovery and control inside the Tauri host adapter. Put serializable diagnostic value types in `mobile-runtime`, project them through the existing typed workspace IPC, and render them as a compact Settings diagnostic block. The M1.5 acceptance path reuses the production observation pipeline and reports each gate independently; a failed environment prerequisite closes the gate and prevents all M2 work.

**Tech Stack:** Rust, Tauri 2, React 19, TypeScript, Vitest, Android command-line tools when already installed.

**Spec:** User-provided execution specification (local attachment; not committed).

## Global Constraints

- Do not redo M0/M1, redesign the three-column Mobile UI, or change Evidence/Signal/Strategy architecture.
- Search `ANDROID_HOME`, `ANDROID_SDK_ROOT`, configured SDK, `~/Library/Android/sdk`, and then PATH; never require environment variables when an existing SDK is discoverable.
- Do not install Android Studio, system images, tools, apps, or AVDs; do not modify or delete existing AVDs.
- AVD launch must have bounded timeouts and must verify ADB online state plus `sys.boot_completed=1`.
- All formal UI copy uses English and Simplified Chinese locale keys.
- Do not implement any mobile control action unless all ten real M1.5 gates pass.
- Do not modify, stage, restore, or commit the user's existing `AGENTS.md` change. Do not push.

## Review Focus

- A configured SDK path may point either to the SDK root or directly to `adb`; detection must normalize both without inventing sibling tool paths from an `adb` file.
- Tool files may exist but be non-executable or return errors; readiness and version fields must remain truthful.
- `adb devices -l` can include physical, offline, unauthorized, and emulator devices; only online emulators satisfy the M1 gate.
- AVD metadata can be incomplete; list output must preserve the AVD name and represent unavailable profile/architecture as absent.
- A spawned emulator can hang or boot incompletely; bounded polling must return an explicit timeout and must not kill a pre-existing user emulator.

---

### Task 1: Android Environment Detector and AVD Lifecycle

**Files:**
- Modify: `apps/desktop/src-tauri/src/mobile.rs`
- Test: `apps/desktop/src-tauri/src/mobile.rs`

**Interfaces:**
- Consumes: optional configured SDK string plus process environment and command outputs.
- Produces: `AndroidEnvironmentDiagnostics`, `AndroidAvdInfo`, `AndroidDeviceInfo`, `MobileHost::environment_diagnostics`, `MobileHost::start_avd`, and `MobileHost::stop_session`.

- [ ] **Step 1: Write failing detector tests**

Add table-driven Rust tests that create a temporary SDK with executable `platform-tools/adb`, `emulator/emulator`, and command-line tools; assert configured root precedence, standard-path fallback, direct-ADB normalization, missing-tool status, AVD config metadata, online/offline device parsing, and non-emulator exclusion.

- [ ] **Step 2: Run detector tests and verify RED**

Run: `cargo test -p ordinconn-desktop android_environment`

Expected: compilation fails because the detector and diagnostic types do not exist.

- [ ] **Step 3: Implement minimal detector and parsers**

Implement a focused `AndroidEnvironmentDetector` that receives explicit candidates for tests and production candidates for runtime use. Execute discovered tools with `std::process::Command`, cap diagnostic stderr, parse `adb version`, `adb devices -l`, `emulator -list-avds`, `adb -s <id> emu avd name`, and `.android/avd/<name>.avd/config.ini` when available.

- [ ] **Step 4: Run detector tests and verify GREEN**

Run: `cargo test -p ordinconn-desktop android_environment`

Expected: all detector behavior tests pass.

- [ ] **Step 5: Write failing bounded-start and shutdown tests**

Use executable temporary shell fixtures that emulate an emulator process and ADB state transitions. Assert unknown AVD rejection, timeout, online-plus-boot completion, and idempotent logical session shutdown.

- [ ] **Step 6: Run lifecycle tests and verify RED**

Run: `cargo test -p ordinconn-desktop avd_lifecycle`

Expected: failures name the missing `start_avd`/`stop_session` behavior.

- [ ] **Step 7: Implement bounded lifecycle behavior**

Launch only an AVD returned by discovery, poll `adb devices` and `getprop sys.boot_completed` until a test-configurable deadline, never kill the emulator, and mark only the OrdinConn logical session ended on shutdown.

- [ ] **Step 8: Run lifecycle tests and full desktop Rust tests**

Run: `cargo test -p ordinconn-desktop`

Expected: all desktop Rust tests pass.

### Task 2: Typed IPC and Settings Diagnostics

**Files:**
- Modify: `crates/mobile-runtime/src/lib.rs`
- Modify: `crates/ordinconn-app/src/mobile_intelligence.rs`
- Modify: `packages/contracts/src/index.ts`
- Modify: `packages/contracts/src/index.test.ts`
- Modify: `apps/desktop/src-tauri/src/commands.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/runtime/client.ts`
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`
- Create: `apps/desktop/src/pages/SettingsPage.test.tsx`
- Modify: `apps/desktop/src/i18n/en.ts`
- Modify: `apps/desktop/src/i18n/zh-CN.ts`

**Interfaces:**
- Consumes: Task 1 diagnostic and lifecycle methods.
- Produces: `MobileWorkspaceDto.androidEnvironment`, `start_mobile_avd` IPC, and localized Settings diagnostics showing statuses, counts, and actual paths.

- [ ] **Step 1: Write failing contract and Settings tests**

Extend the contract fixture with literal diagnostic data and render Settings with missing and ready environments. Assert SDK/ADB/emulator status, AVD/device counts, and visible actual paths. Assert no navigation action capability is present.

- [ ] **Step 2: Run TypeScript tests and verify RED**

Run: `npm test --workspace @ordinconn/contracts && npm test --workspace @ordinconn/desktop`

Expected: type/test failures identify the absent diagnostic contract and UI.

- [ ] **Step 3: Implement typed projection and localized UI**

Add serializable diagnostics to the workspace, populate it on every `get_mobile_workspace` and observation response, expose only existing-AVD start through typed IPC, and add a compact Settings diagnostic grid and per-AVD start button. Preserve the existing Home three-column layout and observe-only labels.

- [ ] **Step 4: Run TypeScript tests and typecheck**

Run: `npm test --workspace @ordinconn/contracts && npm test --workspace @ordinconn/desktop && npm run typecheck`

Expected: all contract/UI tests and TypeScript checks pass.

- [ ] **Step 5: Run Rust integration tests**

Run: `cargo test -p mobile-runtime -p ordinconn-app -p ordinconn-desktop`

Expected: all affected Rust tests pass.

### Task 3: M1.5 Gate, Documentation, and Phase Stop

**Files:**
- Modify: `apps/desktop/src-tauri/src/mobile.rs`
- Modify: `docs/mobile/DEVICE_RUNTIME.md`
- Modify: `docs/mobile/MOBILE_ACTION_PROTOCOL.md`
- Modify: `docs/mobile/MOBILE_SECURITY.md`
- Create: `docs/mobile/M1_5_ACCEPTANCE.md`

**Interfaces:**
- Consumes: production environment diagnostics and the existing observation pipeline.
- Produces: an explicit ten-check M1.5 gate report and operator documentation that prevents M2 on any failed check.

- [ ] **Step 1: Write a failing gate-classification test**

Construct a missing environment and assert `ADB_READY`, `EMULATOR_READY`, and `DEVICE_ONLINE` fail while all downstream real-capture checks remain blocked rather than guessed or mocked.

- [ ] **Step 2: Run the gate test and verify RED**

Run: `cargo test -p ordinconn-desktop m1_5_gate`

Expected: compilation fails because the gate report does not exist.

- [ ] **Step 3: Implement the explicit gate report and real smoke harness**

The `ORDINCONN_MOBILE_SMOKE=1` test must call production detection and observation, verify frame bytes/hash, UI tree parse/hash, snapshot refs, MobileObservation, IPC-compatible serialization, and logical session shutdown. Missing prerequisites must be reported as failed/blocked checks and must not use fixtures.

- [ ] **Step 4: Run the real gate command on this machine**

Run: `ORDINCONN_MOBILE_SMOKE=1 cargo test -p ordinconn-desktop real_emulator_smoke -- --nocapture`

Expected on the currently detected machine: fail closed with missing Android SDK/ADB/emulator; no M2 task begins.

- [ ] **Step 5: Document detection, bounded launch, gate criteria, and stop reason**

Update the three existing mobile documents and add `M1_5_ACCEPTANCE.md` with exact commands, ten gate names, distinction between automated parser coverage and real-device evidence, and the observed local failure state.

- [ ] **Step 6: Run full verification**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace && npm test && npm run typecheck && npm run build && npm run desktop:build`

Expected: all static/unit/build/package checks pass. The separate real smoke remains a truthful failed acceptance prerequisite and is reported as such.

- [ ] **Step 7: Commit only M1.5-owned files**

Run: `git add <explicit M1.5 files> && git commit -m "feat: validate Android mobile runtime"`

Expected: one local commit; `AGENTS.md` remains modified and unstaged; no push.
