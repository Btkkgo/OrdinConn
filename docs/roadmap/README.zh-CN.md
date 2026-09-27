# 公开路线图

[English](README.md) | [简体中文](README.zh-CN.md)

> 当前 Phase 5：有界 Planner/Executor 集成已实现；真实模型验收 **BLOCKED_MODEL_NOT_CONFIGURED**（Production Provider Count **0**），Full Autonomous M3 **NOT_COMPLETE**。下文较早的 Phase 2–4 验证记录保留为历史证据；最新验证见 [Phase 5 收口记录](../mobile/M3_PHASE5.zh-CN.md)。


路线图条目代表方向，不等于完成声明。

## 当前 Gate

M1.5 与 M2 已在专用 `OrdinConn_M1_5` Android 16 ARM64 AVD 上通过。M2 的 30 项验收包含真实受限导航、安全负例、脱敏 Receipt，以及打包桌面 Rust → Tauri → React 人工 Tap。详见 [Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7) 和 [M2 验收记录](../mobile/M2_ACCEPTANCE.zh-CN.md)。M3 Phase 2/3 基础与 Phase 4 有界单步执行已在本地实现，见 [M3 Executor](../mobile/M3_EXECUTOR.zh-CN.md)。完整自主 M3 为 NOT_COMPLETE：Provider 0 阻塞真实模型验收；Phase 5 Stop/Approval 已集成。

完成的 Gate 记录在 [GitHub Issue #1](https://github.com/Btkkgo/OrdinConn/issues/1)。间歇性 Process-fixture 并发超时风险单独记录在 [Issue #4](https://github.com/Btkkgo/OrdinConn/issues/4)。

## M2 — 验证式导航

M1.5 通过后，最小安全 Action Set 已在 Emulator 上验证：有界 Tap、Swipe、Type、Back/Home 和 Application Navigation，具备 Preconditions、Policy Check、Action Receipt、Post-action Observation 与 Verification。这不授权自主 App Skills 或物理设备。

## M3 — 生产级 App Skill

实现一个范围狭窄、加入 Allowlist 的 App Skill，具备确定性 Navigation Rule、Recovery、Budget 和 Auditability。

## M4 — Evidence 晋升

通过 Connector Registry 增加显式 `MobileObservation -> Evidence` Policy，再允许符合条件的 Evidence 进入 Strategy 和 `SignalCandidate` 评估。

## M5 — 物理 Android 设备

将已验证 Runtime 从 Emulator 扩展到明确授权的真实设备，同时不削弱 Permission、Privacy 或 Audit Control。

## 长期 Runtime 方向

- 更广泛的 Desktop Accessibility 与 Event-driven Perception
- 显式 Work Session 与 Application Allowlist
- Model Gateway 后的更多 Model Adapter
- Connector Registry 后的更多公开数据源
- 复用相同 Protocol 的 Standalone 与 Remote Host

Computer Runtime 路线图研究记录在 [GitHub Issue #3](https://github.com/Btkkgo/OrdinConn/issues/3)。
