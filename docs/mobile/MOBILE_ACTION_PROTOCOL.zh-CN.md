# Mobile Action Protocol

[English](MOBILE_ACTION_PROTOCOL.md) | [简体中文](MOBILE_ACTION_PROTOCOL.zh-CN.md)

M1 仅观察。其 Tool Capability Surface 提供 Observation、Frame、UI Tree 与 Element Inspection；Tap、Swipe、Type、Back、Home、Open App 与 Search 保持不可用。

M1.5 增加 Environment Discovery、Existing-AVD Listing/Start、Diagnostic 与 Real-runtime Acceptance Gate。它们属于 Host Lifecycle Capability，不是 Agent Navigation Action。

M2 已在 [Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7) 范围内通过 30 项真实 Emulator 与打包 GUI Gate；见双语[验收记录](M2_ACCEPTANCE.zh-CN.md)。平台无关 Action Request、Policy Decision、脱敏 Receipt、Verification Contract、Typed IPC 和人工控件均已按各自边界实现与验证。即使用户将其他包加入白名单，生产 Host 也只对 Android Settings/Settings Intelligence 执行非逃离动作。每个 Action 引用当前 Snapshot，非逃离输入还会重查实时 UI Tree；每个已派发 Device Action 后都要求新 Observation。默认只有 `VERIFIED` 可以推进 Task。仍没有 Agent 自动导航。

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

在 `M1_5_ACCEPTANCE.md` 中的真实 Capture Gate 与独立 Desktop Acceptance Item 通过前，M2 始终受阻。M1.5 已通过 15/15 强制项；所有者随后另行给出由 Issue #7 跟踪的明确 M2 指令。现在真实 M2 导航与负向 Gate 已通过。通用 Search Action 与自动动作循环仍不在范围内；在允许的 Settings 搜索字段中人工 Tap/Type 属于受限动作集。

2026-09-20 第一次 Run 因 SDK/ADB/Emulator 前置条件缺失而停止。之后的真实 Android 16 ARM64 Acceptance 已通过；项目在所有者新指令前按要求停在 M2 之前。
