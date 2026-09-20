# English

## Goal

Establish a real Android Emulator environment and complete M1.5 validation.

## Scope

Install or locate the required official Android tooling, create one dedicated safe AVD, execute the observe-only production path, and verify capture, redaction, persistence, typed IPC, and shutdown. M2 actions are outside this Issue.

## Current State

Closed as verified. M1.5 is **PASS** and M2 is **NOT STARTED**. The separate intermittent process-fixture risk remains open in Issue #4.

## Result — 2026-09-20

- Android SDK: PASS
- ADB: PASS
- Emulator: PASS
- AVD: PASS — `OrdinConn_M1_5`
- Online Android Emulator: PASS
- Frame Capture: PASS
- UI Tree: PASS
- Sensitive Redaction: PASS
- Snapshot Parse / Element Refs: PASS
- MobileObservation: PASS
- Tauri IPC / Session Shutdown: PASS
- M1.5 Gate: PASS
- M2: NOT STARTED

The real Android 16 / API 36 ARM64 capture gate passed all ten checks, including honest `WORKSPACE_PROJECTION`. A separate packaged-desktop smoke passed the real Rust → Tauri command/event → React path for status, allowlist error, session start, observation, and stop. The temporary sensitive-node test app and artifacts were removed after validation. Intermittent default-concurrency process-fixture timeouts remain separately tracked in Issue #4.

## Validation

The final verification recorded 125 passing Rust workspace tests, 28 passing serial desktop tests, 31 passing TypeScript tests, typecheck, Vite build, Tauri bundle, rustfmt, Clippy, the real ten-check capture gate, real password-node redaction, and packaged GUI acceptance.

## Completed Work

- Install or locate Android SDK.
- Verify the ADB executable.
- Verify the Emulator executable.
- Create one safe AVD.
- Launch the Android Emulator.
- Confirm an online emulator through ADB.
- Run the M1 real smoke.
- Capture a real frame.
- Dump a real UIAutomator hierarchy.
- Validate sensitive-node redaction.
- Generate a real `MobileObservation`.
- Verify Snapshot Parse, Element Refs, Tauri IPC, and Session Shutdown.
- Budget the intermittent process-fixture concurrency failure in Issue #4 without weakening the production gate.

## Acceptance Criteria

Every mandatory M1.5 item ran against the real production path. Workspace projection and real GUI IPC are recorded as separate evidence so the helper test is not overstated. Mock or fixture output was not used as a substitute. M1.5 may be marked passed; M2 still requires a separate explicit owner instruction.

## Current Boundary

This Issue does not authorize M2 navigation actions, real-money operations, unrestricted capture, private application access, or collection of sensitive fields.

# 中文

## 目标

建立真实 Android Emulator 环境并完成 M1.5 验证。

## 范围

安装或定位所需官方 Android Tool；创建一个专用安全 AVD；运行 Observe-only Production Path；验证 Capture、Redaction、Persistence、Typed IPC 与 Shutdown。M2 Action 不属于本 Issue。

## 当前状态

已作为 Verified 关闭。M1.5 为**通过**，M2 为**尚未开始**。独立的间歇 Process-fixture 风险继续在 Issue #4 中保持 Open。

## 结果 — 2026-09-20

- Android SDK：PASS
- ADB：PASS
- Emulator：PASS
- AVD：PASS — `OrdinConn_M1_5`
- Online Android Emulator：PASS
- Frame Capture：PASS
- UI Tree：PASS
- Sensitive Redaction：PASS
- Snapshot Parse / Element Refs：PASS
- `MobileObservation`：PASS
- Tauri IPC / Session Shutdown：PASS
- M1.5 Gate：PASS
- M2：NOT STARTED

真实 Android 16 / API 36 ARM64 Capture Gate 已通过全部十项检查，包括准确命名的 `WORKSPACE_PROJECTION`。另一次打包桌面 Smoke 通过真实 Rust → Tauri Command/Event → React 链路，覆盖 Status、Allowlist Error、Session Start、Observation 与 Stop。临时 Sensitive-node Test App 与 Artifact 在验证后已清理。Default-concurrency Process-fixture 间歇 Timeout 单独记录在 Issue #4。

## 验证

最终验证记录 125 个 Rust Workspace Test 通过、28 个 Serial Desktop Test 通过、31 个 TypeScript Test 通过，以及 Typecheck、Vite Build、Tauri Bundle、rustfmt、Clippy、真实十项 Capture Gate、真实 Password-node Redaction 与 Packaged GUI Acceptance 通过。

## 已完成工作

- 安装或定位 Android SDK。
- 验证 ADB Executable。
- 验证 Emulator Executable。
- 创建一个安全 AVD。
- 启动 Android Emulator。
- 通过 ADB 确认 Online Emulator。
- 运行 M1 Real Smoke。
- 捕获真实 Frame。
- Dump 真实 UIAutomator Hierarchy。
- 验证 Sensitive-node Redaction。
- 生成真实 `MobileObservation`。
- 验证 Snapshot Parse、Element Refs、Tauri IPC 与 Session Shutdown。
- 在 Issue #4 中记录间歇 Process-fixture Concurrency Failure，不削弱 Production Gate。

## 验收标准

所有 M1.5 必做项均在真实 Production Path 上执行。Workspace Projection 与真实 GUI IPC 作为独立 Evidence 记录，避免夸大 Helper Test。Mock 或 Fixture Output 未替代真实验收。M1.5 可标记为通过；M2 仍需项目所有者另行明确授权。

## 当前边界

本 Issue 不授权 M2 Navigation Action、真实资金操作、Unrestricted Capture、Private Application Access 或 Sensitive-field Collection。
