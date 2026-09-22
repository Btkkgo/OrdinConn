# 当前状态

[English](CURRENT_STATUS.md) | [简体中文](CURRENT_STATUS.zh-CN.md)

- 日期：2026-09-22
- 版本：0.1.0
- 正式仓库：https://github.com/Btkkgo/OrdinConn
- 当前 Mobile Gate：https://github.com/Btkkgo/OrdinConn/issues/1
- 默认公开分支：`main`
- 产品源码基线：`f8705c5`
- Mobile 阶段：M1.5 真实环境验收完成
- Gate：**M1.5 通过**
- 强制验收项：**15 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN**
- Runtime Reliability 后续：**Issue #4 CLOSED / VERIFIED**
- 当前阶段：**M2 已验证 — 30 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN**（[Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7)；[验收记录](mobile/M2_ACCEPTANCE.zh-CN.md)）

本文严格区分已实现、已验证、部分完成、受阻、已设计、已计划和尚未开始。设计文档不能替代运行证据。

## 已实现

- 基于 Tauri、React 和类型化 IPC 的本地优先桌面应用。
- 市场、Evidence、Signal、Strategy、Model、Agent、Approval、Execution、Connector 和 Tool 的 Rust 领域模块。
- Model Gateway、OpenAI-compatible Adapter 和确定性 Mock Adapter。
- Connector Registry、公开数据采集、调度、滚动历史、确定性策略和 Evidence Cluster。
- Evidence-gated Signal、Agent Report、精确 Approval Capability 和仅 Paper Execution。
- Mobile M0/M1 的设备会话、Screen Frame、语义 UI Snapshot、Element Ref、隐私分类和 `MobileObservation` 契约。
- Android observe-only 代码路径：工具发现、已有 AVD 检查、在线 Emulator 发现、受限 Frame Capture、UI Tree Dump、敏感节点脱敏、持久化、事件和 Workspace 投影。
- M2 的六种人工 Emulator Action、失败即拒绝策略、有界 ADB 适配器、动作后验证、脱敏 SQLite Receipt/Audit，以及类型化 Tauri/React 控件。
- 公开工程文档、Issue Template、人工 X Draft 和 Fail-closed 公共仓库安全门。
- 独立于 Codex/ChatGPT 的 macOS 两小时 GitHub 同步 LaunchAgent；它通过专用 TCC 身份和 Application Support runner 工作。

## 已验证

在产品源码基线 `c4f2d66` 上，最近一次完整记录为：126 个 Rust 测试通过、31 个 TypeScript 测试通过、Rust 格式与 Clippy 通过、TypeScript Typecheck 与 Vite Build 通过，并成功生成 macOS Tauri App Bundle。

在 Issue #1 修正分支上，最终默认并行 Rust Workspace 重跑通过 125 个测试；31 个 TypeScript 测试、TypeScript Typecheck、Vite Build、macOS Tauri Bundle、rustfmt 以及禁止 Warning 的 Clippy 也全部通过。

真实 Android Smoke 已在专用 `OrdinConn_M1_5` AVD 上通过全部 10 项检查。该 AVD 使用 Pixel 8 设备配置和 Android 36 Google APIs ARM64 镜像；在线设备报告 Android 16 / API 36、1080×2400、420 dpi。最终生产重跑捕获了 188,909 bytes PNG，解析出 70 个已脱敏 UI 元素和 70 个 Snapshot Ref，生成 `MobileObservation`，通过持久化、审计和 Workspace Projection，并完成逻辑 Session Shutdown。

类型化 IPC 另有真实 GUI 证据：打包 Tauri 应用先在空白名单下显示受控错误；允许 `com.android.settings` 后，React UI 显示 `Observing`、`emulator-5554`、包名、`VERIFIED`、真实画面和 70 个 UI 元素。新增 Stop Session Command 让界面返回 `Disconnected`；同一 Session 持久化完整的 start/snapshot/observation/end 事件序列，且用户拥有的 AVD 仍保持在线。

独立的临时本地测试 App 产生了 1 个真实 `password=true` 节点。UIAutomator 没有输出测试明文，OrdinConn 记录了 1 次 Redaction，序列化 Capture 不含测试值。验收后已卸载 App 并清理临时产物。

开源初始化阶段，公开日志脱敏、同步脚本、调度器渲染、文档校验、plist、TypeScript 测试与 Typecheck、Vite Build 和 Tauri Bundle 均通过。本次修正的第一次默认并行 Workspace 尝试暴露 2 个间歇性 AVD lifecycle Fixture 失败；修正另一个无关的 Gate 标签断言后，完整默认并行 Workspace 重跑通过，28 个 Desktop 测试也全部串行通过。该不确定性被记录在 Issue #4，没有因后续重跑变绿而删除。

M1.5 Stage Close 验证再次复现 Issue #4：第一次新鲜 Default-parallel Workspace Run 的 Desktop Test 为 24/28，通过 24 项、失败 4 个 Process/AVD Timing-sensitive Fixture。紧接着的 Serial Desktop Run 为 28/28 PASS，下一次完整 Default-parallel Workspace Rerun 为 125/125 PASS。在当时的检查点 Issue 保持 Open；它不推翻已经独立完成的 M1.5 15/15 真实验收。

Issue #4 Reliability 分支上，改动前十次 Default-parallel Desktop 运行的第一次再次由同样四个测试造成 24/28，随后九次热态运行通过。受控四夹具冷启动回归在原一秒成功预算下失败；将有界的测试专用成功预算与保持不变的 30/50ms 超时检查分开后通过。随后 Default-parallel Desktop 20/20 次通过（每次 29/29）、完整 Workspace 10/10 次通过（每次 136 项测试），八线程 Desktop 29/29 通过。真实 Android Smoke 首次正确拒绝白名单外的冷启动 Launcher；在专用 AVD 上打开 Settings 后，原样 Smoke 十项全部通过。生产 Deadline 和 Runtime Code 均未改动。Rust Formatting、禁止 Warning 的 Clippy、31 个 TypeScript 测试、Typecheck、Rust/Vite Build 与 Tauri Bundle 均通过。

PR #6 已将验证过的 Tree 以 `52e73ca` 合并到 `main`；合并后的完整 Workspace 重跑及隔离公开历史安全门通过。Issue #4 作为 Verified 关闭。此前保留的仅本地隐私备份已在 M2 开工前依所有者明确授权删除；本地全引用安全门现已通过。

Issue #7 分支已在专用 AVD 的真实 Settings 流程覆盖 Tap、Swipe、Back、Type、Home、OpenApp、动作后 Observation、Receipt 持久化与 Audit Event。真实过期 Ref、错误 Package、未授权 App、临时密码字段 Type 均在 ADB 输入前被阻断。中英文金融与敏感文案通过合成 Fixture 阻断，没有连接金融 App。最终复核进一步收紧同 Activity 的 UI Tree 执行前校验、动作前原子持久的 Pending Receipt/Audit Intent、分动作验证、动态 Home Package、Session ID/预算轮换及固定的 Settings 生产动作面；这些修改后真实 M2 Smoke 再次通过。最终打包版 Tauri UI 完成人工 Inspector Tap → 类型化 IPC → `executed · VERIFIED`，显示不同的前后 Snapshot ID；Stop Session 返回 Disconnected，AVD 保持在线。M1.5 十项 Smoke、默认并行 Desktop 44/44、完整 Rust Workspace、30 个 Desktop 与 5 个 Contracts TypeScript 测试、Formatting、Clippy、Typecheck、Rust/Vite Build 和最终 macOS Bundle 均通过。确切证据与边界见双语 [M2 验收记录](mobile/M2_ACCEPTANCE.zh-CN.md)。

PR #8 首个 Merge Commit 未通过合并后公开历史身份门，使 M2 当时为 29 PASS / 1 FAIL。一次授权的隐私修复以 noreply Author/Committer 元数据把 `af843783` 替换为未签名的 `5a19c25`；Tree、按顺序排列的 Parents、Message、时间戳与产品文件完全相同。随后公开可达的 `main` 身份检查通过。旧对象仍可从 GitHub 访问，PR #8 也仍引用它；这一缓存状态与公开可达历史 Gate 分开处理。最终 30/30 结论以成对验收记录和完成的安全检查为依据，不以合并前技术结果单独推断。

PR #9 新增永久身份门，并以 noreply 身份受控本地合并为 `5bdf6b3`；GitHub 已识别该合并。合并后的工作区、暂存快照、可达历史、本地全引用、仓库安全、公开文档及 Diff Gate 均通过。Issue #7 为 CLOSED / `status:verified`。GitHub 没有为 PR #9 报告已配置的 CI 检查；上述本地 Gate 是本次验证证据。

独立 GitHub 调度器已在 macOS 上完成验证：`launchctl` 已加载 `com.ordinconn.github-sync`，执行间隔为 7,200 秒；专用 launcher 已获得 Documents 访问权限；首次后台执行在不依赖 Codex 的情况下完成了无变更扫描。最终变更推送与第二次无变更验收记录在 Issue #2 和 DevLog 中。

## 部分完成

- Computer Runtime 已有接口和权限概念，但尚未形成广泛的生产级电脑操作能力。
- Computer Runtime 仍属部分完成；M2 真实动作 Gate 已在上文独立验证。

## 受阻

- M1.5 产品 Gate 已无受阻项。
- 产品验收项已无受阻。Issue #4 的历史 Timeout Failure 保留在记录中；Reliability 分支上的重复默认并行验证现已通过。

## 已设计

- Structured Perception 优先于 Vision Inference。
- 有合适 API 时，API 优先于 GUI Automation。
- Event-driven Perception 优先于 Continuous Capture。
- M3 生产 App Skills、M4 `MobileObservation → Evidence`、M5 物理 Android 设备。

以上设计不代表当前已经实现。

## 已计划

- 将 Issue #4 Reliability Coverage 保持在默认并行测试套件中，不削弱生产命令时限或真实 Gate。
- 持续使用 GitHub Issue-first 工程记录。

## 尚未开始

- Mobile M3 生产 App Skills 与任何自主导航。
- Mobile Observation 自动晋升为 Evidence。
- 物理 Android 设备支持。
- 真实资金执行。

## Gate 规则

只有 Android SDK、ADB、Emulator、AVD、在线设备、真实 Smoke、Frame Capture、UI Tree、敏感信息脱敏、`MobileObservation`、Snapshot Parse、Element Refs、Tauri IPC 和 Session Shutdown 全部真实通过，M1.5 才能通过。只安装 SDK 不足以通过 Gate。

## 下一步

由所有者实际体验一次 M2 Verified Navigation：Observe → Select Element → Tap/Swipe/Type → Receipt → Post Observation → Verification。停在 M2；未经所有者新请求，不进入 M3，也不创建或发布 X 内容。
