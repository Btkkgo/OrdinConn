# 公开路线图

[English](README.md) | [简体中文](README.zh-CN.md)

路线图条目代表方向，不等于完成声明。

## 当前 Gate

M1.5 已在专用 `OrdinConn_M1_5` Android 16 ARM64 AVD 上通过。Production Capture Gate 和另行执行的打包桌面 Rust → Tauri → React 验收均已通过。M2 仍为**尚未开始**，且需要新的明确指令才能进入。

完成的 Gate 记录在 [GitHub Issue #1](https://github.com/Btkkgo/OrdinConn/issues/1)。间歇性 Process-fixture 并发超时风险单独记录在 [Issue #4](https://github.com/Btkkgo/OrdinConn/issues/4)。

## M2 — 验证式导航

只有 M1.5 通过后，才允许加入最小安全 Action Set：有界 Tap、Swipe、Type、Back/Home 和 Application Navigation，并具备 Preconditions、Policy Check、Action Receipt、Post-action Observation 与 Verification。

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
