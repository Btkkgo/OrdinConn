# 当前状态

[English](CURRENT_STATUS.md) | [简体中文](CURRENT_STATUS.zh-CN.md)

## H1 MOBILE_BUSY 诊断 — 2026-10-02

[Issue #14](https://github.com/Btkkgo/OrdinConn/issues/14)：用户 H1 结果为 **FAIL / BLOCKED**，修复版 **READY TO RETEST**。真实 70 元素 Settings Observation 及对应 completed ActionResult 有效；另一条重叠请求被正确拒绝。前端缺少执行中保护而允许重入，界面继续显示开始更晚的拒绝记录。已补共享同步入口保护、按钮执行中状态及 fixture 回归。Rust **339 PASS**（排除七项真实 Gate）、Desktop **77 PASS**、Contracts **9 PASS**，typecheck、前端与 native 编译 PASS。本轮 Android 动作 / 真实 Provider 请求 / Signing 验证 **0 / 0 / 0**。未进入 H2，Issue 保持 open `status:needs-validation`。见[证据和修复](mobile/H1_MOBILE_BUSY_DIAGNOSIS.zh-CN.md)。

## Desktop 启动诊断 — 2026-10-02

[Issue #15](https://github.com/Btkkgo/OrdinConn/issues/15)：冻结的迁移 12 之前 bundle 启动 SIGABRT 根因已证明；当前正式开发入口可以打开同一数据库并持续运行。旧 bundle 未修复或替换。Rust 338 / Desktop 73 / Contracts 9 PASS；真实 Android 动作、Provider 请求、Signing 验证保持 0。Native 视觉确认与 Issue #14 H1–H7 仍由用户控制，尚未验证。见[诊断与开发启动指引](mobile/DESKTOP_STARTUP_DIAGNOSIS.zh-CN.md)。

## Mobile Interaction 与数据采集 — 2026-10-02

[Issue #14](https://github.com/Btkkgo/OrdinConn/issues/14) 在既有 Android Runtime 和 SQLite 上增加人工控制、本地保存的 `Observe → Interact → Observe → Diff → Extract → Data Object → Data Stream → Provenance` 闭环。数据链为 `Source → Observation → Extraction → Data Object → Insight → Plan → Action → Result`；本阶段实现到 **Data Object**，Insight/Plan 保留接口。Home 提供实际采集对象、元素检查与人工操作、确定性 Observation Context。敏感值在持久化前脱敏，确定性去重保留重复观察及原始证据。自动 fixture 验证与 **PENDING HUMAN ACCEPTANCE** 分开；生产动作沿用既有模拟器/Settings 安全范围。

M3 状态与历史、Provider、retry/backoff/deadline、Keychain 和保留的 App 均不变。自动真实 Android 动作 **0**；Codex 人工 Android 动作 **0**；真实 Provider 请求 **0**；签名验证 **0**；X Draft **NONE**。允许 Rust/frontend 编译；支持的包装器会执行 `codesign`，因此 macOS 验收打包 **NOT RUN / BLOCKED BY THE NO-SIGNING BOUNDARY**。参见[实现与人工验收](mobile/MOBILE_DATA_ACQUISITION.zh-CN.md)。

> 当前 M3：已保存 Google Gemini / `gemini-3.6-flash`，Provider Count **1**。新一轮带间隔的真实 Planner-only 批次为 **1/5**（执行 3 次、未运行 2 次；10 attempts，6 个 HTTP 429、2 个 HTTP 503、1 个 HTTP 200、1 次 deadline 中断）。有界重试回归 PASS；真实验收按 **FREE_TIER_RATE_LIMIT_BLOCKED** 停止；Full Autonomous M3 仍为 **NOT_COMPLETE**。下文 Phase 2–5 计数属于历史记录。部署定位为 **PERSONAL / LOCAL-ONLY APPLICATION**；Apple 签名及分发为 **OUT_OF_SCOPE_LOCAL_ONLY**。


- 日期：2026-10-01
- 版本：0.1.0
- 正式仓库：https://github.com/Btkkgo/OrdinConn
- 当前 Mobile Gate：https://github.com/Btkkgo/OrdinConn/issues/1
- 默认公开分支：`main`
- 工作分支：`codex/realtime-workbench`，基线 `cc1d0c9`；包含保留的 Issue #11 前置修改及 Issue #12 Phase 2–5 集成。
- Mobile 阶段：M1.5 真实环境验收完成
- Gate：**M1.5 通过**
- 强制验收项：**15 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN**
- Runtime Reliability 后续：**Issue #4 CLOSED / VERIFIED**
- 当前阶段：**M2 已验证 — 30 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN**（[Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7)；[验收记录](mobile/M2_ACCEPTANCE.zh-CN.md)）
- 本地 M3 基础：**PHASE 5 IMPLEMENTED / NEEDS LIVE VALIDATION**；完整自主验收 **NOT_COMPLETE**；当前验证见 Phase 5 验收记录。

- 工作台：**IMPLEMENTED / NEEDS VALIDATION**，[Issue #11](https://github.com/Btkkgo/OrdinConn/issues/11)。自定义手机目标接线已实现；真实模型验收为 `BLOCKED_MODEL_NOT_CONFIGURED`。
- Operator Experience：**PENDING USER ACCEPTANCE**，[Issue #10](https://github.com/Btkkgo/OrdinConn/issues/10)。

本文严格区分已实现、已验证、部分完成、受阻、已设计、已计划和尚未开始。设计文档不能替代运行证据。

## 已实现

- 按参考图实现深色主页工作台、真实已加载对象统计、类型化指令、持久化 pending 手机研究任务投影，以及需显式启用的开发视觉 Fixture。仓库、设置、数据详情讨论和人工手机 Inspector 保留可访问。

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

Issue #11 验证：60 项 TypeScript 测试/类型检查 PASS；165 项 Rust 测试串行 PASS，保留默认并行 AVD 夹具失败。真实桌面包 Observe/Collect/Stop PASS。详见[工作台验收记录](mobile/WORKBENCH_ACCEPTANCE.zh-CN.md)；这不等于自主任务执行或所有者 UX 通过。

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

独立 GitHub 调度器已在 macOS 上完成验证：`launchctl` 已加载 `com.ordinconn.github-sync`，执行间隔为 7,200 秒；专用 launcher 已获得 Documents 访问权限；首次后台执行在不依赖 Codex 的情况下完成了无变更扫描。最终变更推送与第二次无变更验收记录在 Issue #2 和 DevLog 中。

## 部分完成

- Computer Runtime 已有接口和权限概念，但尚未形成广泛的生产级电脑操作能力。
- Computer Runtime 仍属部分完成；M2 真实动作 Gate 已在上文独立验证。

## 受阻

- M1.5 产品 Gate 已无受阻项。
- Issue #11 所有者 UX 验收仍独立进行。Phase 5 已将 Home Goal 接入有界 Rust Runner；现有一个已保存生产 Provider，瞬态 Provider 故障与尚未执行的 Android Final Gate 使 M3 仍待验证；模型宣称不能代替类型化完成证据。
- Issue #4 的历史失败仍保留；已完成的 Reliability 验证不变。

## 已设计

- Structured Perception 优先于 Vision Inference。
- 有合适 API 时，API 优先于 GUI Automation。
- Event-driven Perception 优先于 Continuous Capture。
- M3 生产 App Skills、M4 `MobileObservation → Evidence`、M5 物理 Android 设备。
- 在 Owner 本地内嵌 Runtime 中保持协议与传输无关；远程部署不在个人 Mac 范围内。

以上设计不代表当前已经实现。

## 已计划

- 将 Issue #4 Reliability Coverage 保持在默认并行测试套件中，不削弱生产命令时限或真实 Gate。
- 持续使用 GitHub Issue-first 工程记录。

## 尚未开始

- 完整自主 M3 导航及生产 App Skills（Phase 4 显式单步执行已单独实现）。
- Mobile Observation 自动晋升为 Evidence。
- 物理 Android 设备支持。
- 真实资金执行。

## Gate 规则

只有 Android SDK、ADB、Emulator、AVD、在线设备、真实 Smoke、Frame Capture、UI Tree、敏感信息脱敏、`MobileObservation`、Snapshot Parse、Element Refs、Tauri IPC 和 Session Shutdown 全部真实通过，M1.5 才能通过。只安装 SDK 不足以通过 Gate。

## 下一步

保留现有唯一 Provider 与冻结 binary，停止于 FREE_TIER_RATE_LIMIT_BLOCKED。后续模型运行前由 Owner 选择等待配额/冷却、其他模型/Provider 或付费层；本轮未授权 Android Final Gate。Issue #11 工作台审查与 #10 所有者操作体验反馈单独处理。Full Autonomous M3 仍为 NOT_COMPLETE，#12 保持 OPEN / status:needs-validation。无 X 草稿或发布。

## M3 Phase 2 — 持久化执行契约（历史）

[Issue #12](https://github.com/Btkkgo/OrdinConn/issues/12) 跟踪完整 M3 foundation，保持 OPEN / NEEDS VALIDATION。Phase 1 审计完成，**Phase 2 Technical Acceptance PASS**，已接受的实时工作台视觉设计保持不变。

已实现并验证：canonical Rust Goal/Plan/Step ID 与状态机、原始 objective 持久化、不可覆盖的 revision 历史、Budget/Error/Risk/ExpectedResult、Observation/Action/Evidence/Result 引用、事务化生命周期审计、fail-closed 启动恢复、确定性 MobileApprovalSubject hash，以及四个创建/读取 typed IPC。Home 输入创建持久化 PENDING MobileGoal，无 Plan 时显示“等待规划”，不制造假 Plan；其他 ResearchTask 保持不变。见 [M3 契约](mobile/M3_CONTRACTS.zh-CN.md)。

Phase 2 历史验证：默认并发 **Rust 203 PASS / 0 FAIL**（新增 repository 30、domain 6、IPC 2；原 Trade Approval 回归保留），**Desktop TypeScript 59 PASS**、**Contracts 9 PASS**；typecheck、前端构建、Rust 全工作区构建、Tauri release 打包、rustfmt、公开安全 Gate、git diff --check PASS。旧数据库迁移保留审计、观察、收藏/保存的 mobile feed、ResearchTask 和设置。合成契约测试不代表真实设备执行证据。

以上 Phase 2 记录为历史基线，Phase 3 为历史记录，当前 Phase 4 状态见下方。原聊天 MockModelAdapter 保持不变并排除在 M3 之外。此前 AVD 测试夹具时间问题保留为 KNOWN_TEST_INFRA_LIMITATION。#11 保持 OPEN。Commit NONE / Push NONE。

## M3 Phase 3 — 显式真实 Gateway Planner（历史记录）

已实现真实 Model Gateway 生产路径、唯一启用 Provider/默认模型解析、严格单 decision Schema、有界 Context/Output/Timeout/Retry、本地 Domain/Safety 校验和 Risk 重算、仅 Completion Proposal、同事务保存 Plan/单个 pending Step 与 metadata events。`plan_mobile_goal` 为显式 typed IPC；Home 提交不调用模型。保留视觉设计，现有面板显示等待执行。见 [M3 Planner](mobile/M3_PLANNER.zh-CN.md)。

最新验证：最终串行工作区 **Rust 239 PASS / 0 FAIL**；**Desktop TypeScript 60 PASS**、**Contracts 9 PASS**、**Model Gateway 9 PASS**、**Phase 2 repository 30 PASS**、**Phase 2 domain 6 PASS**，原 Trade Approval 回归 PASS；typecheck、前端构建、Rust 工作区构建、rustfmt PASS。Tauri release 打包 PASS。四次默认并发工作区尝试及一次较早的完整串行尝试复现 `concurrent_avd_lifecycle_fixtures_remain_independent_during_slow_tool_startup`。最新默认并发另失败 `command_returns_after_success_when_descendant_keeps_stdout_open`，在剩余套件运行前以 **206 PASS / 2 FAIL** 结束。竞争编译结束后，最终完整串行 **239/239 PASS**，含桌面 **47/47**；此前默认并发 **236/236** 属于审计前代码。保留该已知夹具限制，不修改超时或永久串行化测试。未使用真实 AVD。

只读共享 Planner Gate 核对生产 Provider Count **0**；**MODEL_NOT_CONFIGURED PASS**，没有 Provider 配置、凭据保存、生产迁移或假 Plan/Step。原生 Schema 与 JSON-only 经本地 HTTP/隔离 SQLite 测试通过；**Live Provider Gate NOT_RUN_MODEL_NOT_CONFIGURED**。完整 M3 在 Issue #12 保持 OPEN / status:needs-validation；自主执行 **NOT_IMPLEMENTED**、Mobile Action executed **NO**、Commit **NONE**、Push **NONE**。Phase 3 后停止，不启动 Phase 4 Executor 或 Phase 5 Approval execution。

## M3 Phase 4 — 有界单步 Rust Executor（历史）

已实现 Executor 与真实 Settings 验收，**Phase 4 Technical Acceptance PASS**；完整自主 M3 仍为 **NOT_COMPLETE**。生产 Rust `execute_mobile_goal_step` 每次只执行一个持久化 Pending Step。已实现 Device Lease、新 Observe-before、本地语义解析/策略、现有 M2 typed action、真实 Observe-after、类型化 Verification、不可变 Result/Trace，以及 fail-closed 重启和幂等保护。Approval-required Step 进入 WAITING_APPROVAL，不执行；Forbidden 返回 POLICY_BLOCKED。现有卡片刷新持久化 Step 与提取指标，不新增控制，也不自动调用 Executor。见 [M3 Executor](mobile/M3_EXECUTOR.zh-CN.md)。

独立专用 AVD Gate **PASS**，使用隔离测试数据库和 **TEST_ONLY** 确定性 Planner，经同一个生产 Executor 验证：**BACK PASS / ACTIVITY_EQUALS**、**SCROLL_DOWN PASS**、真实 Observe → Action → Observe → Verify、受支持的单次滚动完成条件、重复保护，以及 **EXTRACT PASS** 的真实 Object/Evidence/Projection 关联。**INPUT_TEXT NOT_AVAILABLE_TEST_SURFACE**；精确值和防假成功合成输入验证 PASS。保留此前 Focus/Target/Harness 失败记录；有界 Focus 读取绝不重放动作。

此前默认并发 Rust **272 PASS / 0 FAIL**，最终默认 **240 PASS / 1 FAIL / KNOWN_TEST_INFRA_LIMITATION**；最终完整串行 Rust **272 PASS / 0 FAIL**；桌面 **52/52**、Executor service **18/18**、Executor domain **9/9**、Source 注册 **1/1**、Phase 3 Planner service **20/20** 与 domain **10/10**、Model Gateway **9/9**、Phase 2 repository **30/30** 与 domain **6/6**、原 Trade Approval 回归 PASS。默认套件不启用真实 AVD Gate；真实 Gate 已另外显式运行并通过。保留此前 Phase 3 并发失败，本阶段不增加夹具超时，也不强制串行化正常套件。Desktop TypeScript **63/63**、Contracts **9/9**、typecheck 和前端构建 PASS。Rust 工作区构建、macOS Tauri release app 打包、rustfmt、公开安全/文档/Diff Gate PASS。

生产 Provider Count **0**，最新只读核对，无生产迁移/配置修改/Key 读取；Live Planner + Executor 为 **NOT_RUN_MODEL_NOT_CONFIGURED**。#12 保持 OPEN / status:needs-validation，#11 OPEN，#10 所有者验收不变。Phase 4 后停止；Phase 5 Stop 优先级/Approval 消费与完整真实 AI 验收仍待实现。Commit **NONE**、Push **NONE**，无 X 草稿或发布。


最终并发重跑说明：补齐 Lease panic/poison 和跳过下一 Step 的断言后，最终 `cargo test --workspace` 以 **240 PASS / 1 FAIL** 停止；失败为既有 `mobile::tests::concurrent_avd_lifecycle_fixtures_remain_independent_during_slow_tool_startup`（Fixture_AVD_3 AvdBootTimeout），剩余套件未运行。此前本阶段完整默认并发 **272/272 PASS** 为较早记录；当前默认并发结果为 **FAIL / KNOWN_TEST_INFRA_LIMITATION**，不是 PASS。无 Timeout 增大、强制串行化或真实 AVD 共享资源冲突。最终完整串行结果另行记录。
