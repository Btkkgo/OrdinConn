# 开发时间线

[English](DEVELOPMENT_TIMELINE.md) | [简体中文](DEVELOPMENT_TIMELINE.zh-CN.md)

## 2026-09-20 — Mobile Intelligence M1.5 真实验证

**目标：** 完成 Production Android Capture Gate，并另行证明打包应用的 Rust → Tauri → React 链路，同时不进入 M2。

**变更：** 建立一个专用 Android 16 ARM64 AVD；修复 Subprocess Pipe Handling、Android 16 Focus Detection 与 Malformed-node Safety；增加显式 Typed Stop Command。

**问题：** 真实 Integration 暴露了 Pipe Backpressure、特定版本 Window Diagnostic、非法 Platform Bounds，以及把 Helper-level Projection 夸大为 GUI IPC 的 Acceptance Label。

**方案：** 在有界 Process Execution 内持续读取 Output；读取完整 Window Dump；对非法 Sensitive/Financial Node Fail Closed；把 Helper Evidence 命名为 `WORKSPACE_PROJECTION`；另行验证打包 GUI IPC。

**结果：** 十项 Production Capture Gate 与独立打包桌面验收均通过。Sensitive-node Redaction 通过。M1.5 为**通过**；M2 为**尚未开始**。间歇性 Process-fixture 并发超时继续记录在 Issue #4。

**测试：** 最终验证记录 125 个 Rust Workspace Test 通过、28 个 Serial Desktop Test 通过、31 个 TypeScript Test 通过，以及 Typecheck、Vite Build、Tauri Bundle、rustfmt、Clippy、真实 Emulator Capture、真实 Password-node Redaction 与打包 GUI Acceptance 通过。

**决策：** 将 Issue #1 以 Verified 关闭，Issue #4 保持 Open，进入 M2 前必须得到用户的单独明确授权。

**Codex 笔记：** Lower-level Serialization 与真实 GUI IPC 是不同 Evidence Boundary；现在二者都被准确命名并独立测试。

## 2026-09-20 — 开源初始化

**目标：** 将 GitHub 建立为公开工程事实来源，同时不暴露 Secret、Private Path、Commit Email 或未验证完成声明。

**变更：** 创建正式公开仓库、Issue Taxonomy 与 Template、双语入口文档、规范 Status 与 ADR 结构、Working-tree/Git-history Security Gate、仅文档 Sync 与人工 X Draft 流程。

**状态：** 仓库初始化记录在 [Issue #2](https://github.com/Btkkgo/OrdinConn/issues/2)。X 自动发布保持禁用。

本时间线来自当前 Git History 和仓库文档，不重建私密对话。

## 2026-09-12 — V0.1 Financial-agent 基础

**目标：** 建立 Local-first Financial Intelligence Workflow。

**变更：** Commit `3899f60` 重建基础；`3b30617` 完成 Dashboard 可读性与 Interaction。

**问题：** Financial Intelligence 需要严格区分 Model Inference、Evidence、Signal、Approval 与 Execution。

**方案：** 分离 Transport-independent Rust Domain、Application Service、Typed Tauri IPC 与 React Presentation。Paper Execution 绑定精确、一次性的 Approval Capability。

**结果：** 仓库包含 End-to-end Demo Flow、SQLite Current State、Append-only Audit Event、Localization 与紫/黑/黄 Desktop Shell。

**测试：** 当前测试继续覆盖 Evidence Gate、Approval Consumption、Recovery、IPC Error Redaction 与 Closed-loop Paper Execution。

**决策：** V0.1 不执行真实资金操作。

**Codex 笔记：** Implementation Plan 将工作拆成可独立测试的 Domain、Persistence、Host 与 UI Boundary。

## 2026-09-12 — Core Intelligence Collection

**目标：** 用公开数据 Collection 与 Strategy Pipeline 替代仅 Demo 的 Intelligence Input。

**变更：** `da537a5` 记录架构；`f1b90f5` 实现 Collector 与 Orchestration；`c71ce1a` 让 Feed Schema Drift Fail Closed；`aa356e6` 定义下一阶段 Continuous Intelligence。

**问题：** Raw Source Format 会漂移，Duplicate Content 可能看起来像独立确认，不完整 History 会产生虚假置信度。

**方案：** 规范化 Source Record、为 Schema 建立 Fingerprint、去重 Observation、保留 Evidence Role，并将 Strategy Readiness 与 Evidence Validation 分离。

**结果：** REST、WebSocket、Feed 与 HTML Collection Path 进入共享 Registry 与 Evidence Pipeline。

**测试：** 当前覆盖 Normalization、Rate Limiting、Schema Drift、Feed Parsing、HTML Extraction、Deduplication 与 Publication Rejection。

**决策：** 当 Threshold 或 Readiness Gate 未通过时，0 个 Published Real Signal 是有效结果。

**Codex 笔记：** Deterministic Fixture 让 External-source Behavior 可测试，同时不会被表述成 Live Evidence。

## 2026-09-13 — Continuous Intelligence 与 Derivatives

**目标：** 增加 Long-running Scheduling、Rolling Baseline、公开 Derivatives Input 与持久化 Evidence Cluster。

**变更：** Commit `71b5023` 增加 Bounded Scheduler、Restart-restored Bucket、Binance USD-M Futures Normalization、Derivatives Strategy 与 Cluster Persistence。

**问题：** Live Input 可能迟到、乱序、过期，或在 Baseline 可用前到达。

**方案：** 显式 Readiness State、Bounded History、Deterministic Time Bucket、Capped Reconnect/Backoff 与 Rejected Run Audit Record。

**结果：** Strategy Run 保留 Input Snapshot、Metric、Reason Code、Parameter、Timestamp 与 Provenance。

**测试：** 当前覆盖 Scheduler Single-flight、Backoff、Rolling Metric、Readiness、Derivatives Classification 与 Cluster Semantic。

**决策：** Readiness Failure 产生可审计 Run，但不产生普通 Candidate。

**Codex 笔记：** Soak 与 Live Check 保持 Opt-in，并与 Deterministic Suite 分开。

## 2026-09-20 — Mobile Intelligence M0/M1

**目标：** 增加安全、仅观察的 Android Collection Surface，并接入 Home、Warehouse 与 Settings。

**变更：** `9a1a62d` 定义 Contract；`3a32b4d` 增加 Observe-only Runtime；`1d51c93` 集成 Workspace 与 UI。

**问题：** Phone Screen 本身不是有用的 Market Evidence，Raw UI Tree 可能含 Sensitive Field。

**方案：** 引入 Bounded Frame、Semantic Snapshot、Scoped Element Reference、Privacy Class、Sensitive-node Redaction、Allowlisted Application 与独立 `MobileObservation` Type。

**结果：** Code 可以投影已观察 Mobile State，但不会自动把它晋升为 Evidence 或 Signal。

**测试：** 当前覆盖 Snapshot Construction、Redaction、Frame Retention、Observation Deduplication、Audit Payload Safety、Persistence、Feed Projection 与 UI Contract。

**决策：** M1 仅观察；Tap、Swipe、Type、Launch、Navigation、Login、Posting、Messaging 与 Ordering 都保持禁用。

**Codex 笔记：** Reference Video 只用于 Interaction Structure；未经确认的机制保留为 Reference Behavior，不会被虚构成 Architecture。

## 2026-09-20 — Mobile Intelligence M1.5 首次 Gate

**目标：** 在实现 Navigation 前，用真实 Android Emulator 证明 Production Path。

**变更：** `c4f2d66` 增加 Environment Diagnostic、AVD Lifecycle Check、Bounded Command、Persisted Capture Projection 与十项 Real-runtime Harness。

**问题：** 当时本机没有 Android SDK、ADB、Emulator、AVD 或 Online Device。External Tool Call 也需要显式 Time Bound 与一致 SDK Selection。

**方案：** 分别报告每个前置条件，阻止依赖检查；Diagnostics 与 Observation 使用同一 SDK；终止 Timeout Child Process；IPC Acceptance 需要 Persistence/Event/Workspace Projection。

**结果：** 3 个前置条件失败，7 个检查受阻。M2 未进入。

**测试：** 在当时 Product Baseline 上，126 个 Rust Test 与 31 个 TypeScript Test 通过；Typecheck、Frontend Build 与 macOS Desktop Bundle 也通过。Real Android Smoke 因环境缺失而如实失败。

**决策：** 十项真实检查全部通过前，不编写 M2 Code。

**Codex 笔记：** Independent Review 发现 Command-timeout、SDK-selection、IPC 与 Shutdown Gap；Regression Test 在修复前确实失败，修复后通过。

本条保留第一次受阻检查点。它已被上方真实验证条目取代，不应被理解为当前 M1.5 状态。
