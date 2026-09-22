# Mobile Action Protocol

[English](MOBILE_ACTION_PROTOCOL.md) | [简体中文](MOBILE_ACTION_PROTOCOL.zh-CN.md)

M1 仅观察。其 Tool Capability Surface 提供 Observation、Frame、UI Tree 与 Element Inspection；Tap、Swipe、Type、Back、Home、Open App 与 Search 保持不可用。

M1.5 增加 Environment Discovery、Existing-AVD Listing/Start、Diagnostic 与 Real-runtime Acceptance Gate。它们属于 Host Lifecycle Capability，不是 Agent Navigation Action。

所有者现已通过 [Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7) 授权 M2。平台无关的动作请求、策略决定、脱敏回执和验证契约正在实施。每个 Action 必须引用当前 Snapshot，每个已接受 Device Action 后必须产生新 Observation。默认只有 `VERIFIED` 可以推进 Task。在此检查点，生产设备路径仍仅观察。

保留的 Verification Result：

- `VERIFIED`
- `NO_CHANGE`
- `UNEXPECTED_STATE`
- `TARGET_MISSING`
- `PERMISSION_REQUIRED`
- `LOGIN_REQUIRED`
- `CAPTCHA_BLOCKED`
- `SENSITIVE_FIELD_BLOCKED`
- `FINANCIAL_ACTION_BLOCKED`
- `STALE_OBSERVATION`
- `APP_CRASHED`
- `DEVICE_OFFLINE`
- `INTERRUPTED`

React 永远不会直接调用 ADB 或 Platform Device API。

## M2 Entry Gate

在 `M1_5_ACCEPTANCE.md` 中的真实 Capture Gate 与独立 Desktop Acceptance Item 通过前，M2 始终受阻。Unit Fixture 与 Parser Test 不能代替该 Evidence。M1.5 已通过 15/15 强制项；所有者随后另行给出由 Issue #7 跟踪的明确 M2 指令。该授权只允许受限实施，不代表真实导航已通过验收。Search 和自动动作循环继续不在范围内。

2026-09-20 第一次 Run 因 SDK/ADB/Emulator 前置条件缺失而停止。之后的真实 Android 16 ARM64 Acceptance 已通过；项目在所有者新指令前按要求停在 M2 之前。
