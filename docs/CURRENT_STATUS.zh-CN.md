# 当前状态

- 日期：2026-09-20
- 版本：0.1.0
- 正式仓库：https://github.com/Btkkgo/OrdinConn
- 当前 Mobile Gate：https://github.com/Btkkgo/OrdinConn/issues/1
- 默认公开分支：`main`
- 产品源码基线：`c4f2d66`
- Mobile 阶段：M1/M1.5 环境验证
- Gate：**M1.5 未通过**
- 下一阶段：**M2 尚未开始**

本文严格区分已实现、已验证、部分完成、受阻、已设计、已计划和尚未开始。设计文档不能替代运行证据。

## 已实现

- 基于 Tauri、React 和类型化 IPC 的本地优先桌面应用。
- 市场、Evidence、Signal、Strategy、Model、Agent、Approval、Execution、Connector 和 Tool 的 Rust 领域模块。
- Model Gateway、OpenAI-compatible Adapter 和确定性 Mock Adapter。
- Connector Registry、公开数据采集、调度、滚动历史、确定性策略和 Evidence Cluster。
- Evidence-gated Signal、Agent Report、精确 Approval Capability 和仅 Paper Execution。
- Mobile M0/M1 的设备会话、Screen Frame、语义 UI Snapshot、Element Ref、隐私分类和 `MobileObservation` 契约。
- Android observe-only 代码路径：工具发现、已有 AVD 检查、在线 Emulator 发现、受限 Frame Capture、UI Tree Dump、敏感节点脱敏、持久化、事件和 Workspace 投影。
- 公开工程文档、Issue Template、人工 X Draft 和 Fail-closed 公共仓库安全门。

## 已验证

在产品源码基线 `c4f2d66` 上，最近一次完整记录为：126 个 Rust 测试通过、31 个 TypeScript 测试通过、Rust 格式与 Clippy 通过、TypeScript Typecheck 与 Vite Build 通过，并成功生成 macOS Tauri App Bundle。

真实 Android Smoke 输出了完整的 10 项报告：3 项前置检查失败，7 项依赖检查受阻。这是经过验证的失败结果，不是 Mobile Integration 通过。

开源初始化阶段，公开日志脱敏、同步脚本、调度器渲染、文档校验、plist、TypeScript 测试与 Typecheck、Vite Build 和 Tauri Bundle 均通过。最新默认并行 Desktop Rust Unit Run 有两项 1 秒 AVD 生命周期测试超时；同一组 20 个测试串行运行全部通过。

## 部分完成

- Computer Runtime 已有接口和权限概念，但尚未形成广泛的生产级电脑操作能力。
- Mobile Observation 已有生产代码路径和 Fixture，但没有真实 Emulator 验收。
- Build in Public 同步能力已建立；由于 macOS `Documents` 隐私控制拒绝独立 LaunchAgent 访问当前目录，实际两小时触发由 Codex heartbeat 承担。

## 受阻

- Android SDK 实际路径：**未检测到**。
- ADB：**失败**。
- Emulator：**失败**。
- AVD 枚举与启动：**失败**。
- 在线 Android Emulator：**无**。
- 真实 Frame Capture、UIAutomator Dump、UI Tree Parse、敏感节点脱敏验证、`MobileObservation`、Snapshot Parse、Element Refs、Tauri IPC 和 Session Shutdown：**受阻**。

## 已设计

- Structured Perception 优先于 Vision Inference。
- 有合适 API 时，API 优先于 GUI Automation。
- Event-driven Perception 优先于 Continuous Capture。
- M2 验证式导航、M3 生产 App Skills、M4 `MobileObservation → Evidence`、M5 物理 Android 设备。

以上设计不代表当前已经实现。

## 已计划

- 建立真实 Android SDK、ADB、Emulator 和安全 AVD 环境。
- 在真实环境上原样重跑 M1.5 Gate。
- 在不削弱真实 Gate 的前提下解决并行 AVD Test Timeout。
- 持续使用 GitHub Issue-first 工程记录。

## 尚未开始

- Mobile M2 Action 与 Verified Navigation。
- 面向第三方 App 的生产级 App Skills。
- Mobile Observation 自动晋升为 Evidence。
- 物理 Android 设备支持。
- 真实资金执行。

## Gate 规则

只有 Android SDK、ADB、Emulator、AVD、在线设备、真实 Smoke、Frame Capture、UI Tree、敏感信息脱敏、`MobileObservation`、Snapshot Parse、Element Refs、Tauri IPC 和 Session Shutdown 全部真实通过，M1.5 才能通过。只安装 SDK 不足以通过 Gate。

## 下一步

建立真实 Android Emulator 环境并完成 M1.5 的全部验收项；在此之前不进入 M2。
