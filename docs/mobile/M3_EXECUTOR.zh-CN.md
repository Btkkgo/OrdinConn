# M3 Phase 4 — 有界 Rust Mobile Executor

[English](M3_EXECUTOR.md) | [简体中文](M3_EXECUTOR.zh-CN.md)

> 当前 Phase 5：有界 Planner/Executor 集成已实现；真实模型验收 **BLOCKED_MODEL_NOT_CONFIGURED**（Production Provider Count **0**），Full Autonomous M3 **NOT_COMPLETE**。下文较早的 Phase 2–4 验证记录保留为历史证据；最新验证见 [Phase 5 收口记录](M3_PHASE5.zh-CN.md)。


## 范围与证据边界

Phase 4 每次显式调用只执行一个已持久化 Step。生产 Rust Executor 为真实实现；真实设备验收 Planner 为 **TEST_ONLY**。生产 Provider Count 为 **0**，Live Planner + Executor 为 **NOT_RUN_MODEL_NOT_CONFIGURED**，Full Autonomous M3 为 **NOT_COMPLETE**。Issue [#12](https://github.com/Btkkgo/OrdinConn/issues/12) 保持 OPEN / status:needs-validation；#11 保持 OPEN，#10 操作体验仍由所有者验收。

本阶段没有执行循环、模型回退、Provider 配置、Approval 消费、Phase 5 Stop 优先级、真钱操作或新增界面控制。Commit NONE / Push NONE，保留此前 Phase 2/3 与工作台的本地修改。

## 架构与设备独占

`execute_mobile_goal_step({ goalId })` → `AppRuntime` → `MobileGoalExecutor` → 现有 `MobileHost` → 现有 M2 typed action → 真实后置观察 → `MobileVerificationEngine` → 事务化 Step Result。

Core 与 Transport 解耦；桌面适配层仅提供现有 Host。React 通过类型化 DTO、现有卡片和 Runtime Events 读取持久化状态，提交 Goal 不触发规划或执行；Goal 刷新丢弃过期响应，提取后刷新真实 Workspace Projection。

Rust RAII Device Lease 保存设备、可选 Goal、Executor identity 和取得时间。单进程应用共享同一个 Lease Registry，并保护原有人工 M2 action IPC。即使调用 Future 消失，阻塞 Host 工作仍持有 Lease。持久化单次 Execution identity 和 Step UNIQUE 约束阻止重复 Claim；不同设备相互独立。错误与 panic 释放 Rust 所有权；含糊的 EXECUTING 持久化状态必须恢复处理，禁止重放。

所有显式真实 AVD Gate 还共用跨进程 OS advisory 文件锁，独立于正常并发合成测试运行；未启用时直接返回的测试不构成真实设备证据。

## Observe、Resolve、Act 与 Verify

Repository 在执行前拒绝终态或非活跃 Goal、非 Active Plan、非下一 Step、Step/Time/Failure/Stall Budget 耗尽，以及已有 Executing 或 Waiting Approval Step。Claim 与开始审计同事务提交，再进行外部操作；只有实际取得执行 Claim，PLANNING 才变为 RUNNING。

新 Observe-before 校验 Emulator/Session、白名单、前台 Package/Activity、公开隐私状态和 Snapshot 新鲜度。Step 引用绑定真实前置观察，Execution attempt 保留原规划观察。解析要求原页面结构仍适用；通过 Label/Resource identity 绑定目标，无标签容器只在完整页面结构未变时按唯一真实结构和 Bounds 重新绑定。Planner 坐标、Shell/ADB、URL 和 Intent 不能成为可执行 IPC 输入。

SCROLL_DOWN 对应向上 Swipe，SCROLL_UP 对应向下 Swipe；TAP_ELEMENT 解析当前启用目标。INPUT_TEXT 要求安全、Focused、Editable 目标及一致的 TEXT_EQUALS。BACK 使用 ACTIVITY_CHANGED、确切 ACTIVITY_EQUALS 或语义 ELEMENT_VISIBLE，不使用仅 PNG 变化。OBSERVE/EXTRACT 不修改设备；WAIT 有界且必须容纳于剩余 Runtime Budget。

Mutation 先保存原有 Pending Receipt/Audit intent，再发送恰好一次 typed action。Host 进度确认在实际 Action-completed 和 Observe-after-started 阶段保存审计。Activity 切换时，前台获取最多等待十秒，只重新读取 Focus，绝不重复动作。保留原逐命令时间上限，动作前及最终验证检查 Goal Deadline；这不等于 Phase 5 抢占式 Stop。

验证使用 App 结构、确切 Package/Activity、语义 Element、实际输入值或新持久化 Data Object，排除系统时钟、Snapshot/Ref ID 和仅截图变化。Mutation 不能仅靠 Receipt 通过，必须有相符的真实后置观察与回执链路。跨 Session/Device、错误值、错误 Activity 和歧义目标均 fail closed。验证失败使 Step/Goal FAILED，并停止剩余 Pending Steps；单 Step 成功后 Goal 仍为 RUNNING，直到另行验证受支持的完成条件。

## 输入与完成验证

输入脱敏前，精确比较同一唯一 Editable 控件实际值，不 Trim、不改变大小写。可信回执仅保留 `inputValueVerified`、长度/Hash 及 Snapshot/Tree 绑定。Domain 只在 Type 目标、预期值 Hash、长度和 Verified Receipt 均一致时接受证明。页面变化但实际值错误必定失败；Password/OTP/Seed/Private Key Surface 不可接受。

Completion 文本只是提议。当前支持显式绑定的单步用户目标：“Scroll down the current Settings page once” 或“向下滚动当前设置页面一次”。提议必须引用当前 Goal/Plan/Step 所属的已验证前后 Observation，所有必需 Step、Receipt 和 Execution attempt 均通过。任意自然语言目标、外部引用、敏感或不足证据不能完成 Goal。

## 提取与可追溯性

EXTRACT 经现有 Collector Source Registry 注册受限 `android://com.android.settings` Computer Source，只从真实 Observation 提取有界、脱敏的可见 Settings Label，排除 Editable 值和模型文本。通用 Settings 提取类型为 `mobile_observation_object`，归入 Other，不制造 News/Stock/Customer Feedback 标签或市场资产。

Source、不可变 Data Object、Observed ComputerUse Evidence、Step/Evidence link、最终 Result/Audit 同事务提交。Facts、Source Locator、Capture Time 必须与来源 Observation 相符。ID 链为 Goal → Plan → Step → Before/After Observation → Action Receipt → Step Result → Data Object → Evidence。失败原子回滚对象、证据和结果投影。Mobile Observation 去重不会把独立提取对象合并进其来源观察，重复对象 ID 仍会去重。

## Crash 与 Audit 安全

外部设备动作不置于数据库事务内。Claim/Start 与最终 Result/Audit 分别具有事务边界。动作之后审计失败保留未决 EXECUTING attempt，禁止自动重试。重启将活跃 Goal/Step 标记 INTERRUPTED_BY_RESTART，将 attempt 标记 interrupted。不可变 identity、每 Step 单一 Result、Write-ahead intent 保留链路，但不声称未记录 Crash 下的 Exactly-once。

## 真实设备验收

独立显式命令：

```sh
ORDINCONN_MOBILE_M3_EXECUTOR_SMOKE=1 cargo test -p ordinconn-desktop --lib real_phase4_executor_gate_is_explicitly_gated -- --nocapture
```

Gate 独占专用 `OrdinConn_M1_5` AVD，使用临时隔离数据库，仅导航公开 Settings 测试页面；确定性 Test Plan 经同一持久化 Step contract 输入，不初始化或迁移生产数据库。

最新真实 Gate：**PASS**。安全子页面 Tap 后 **BACK PASS / ACTIVITY_EQUALS**；**SCROLL_DOWN PASS**，真实 Before/Action/After/Typed Verify、重复保护及受支持 Goal 完成；**EXTRACT PASS**，真实 Object/Evidence/Projection Query。**INPUT_TEXT NOT_AVAILABLE_TEST_SURFACE**：未找到符合条件的安全 Focused Editable 页面。精确值及防假成功合成测试通过，不冒充真实输入 Gate。

此前集成尝试因瞬时 Focus 缺失、无标签目标和测试 Harness 重复插入 Snapshot 失败。已分别通过有界 Focus 观察、未变化页面的唯一目标绑定和 Harness 新观察修复，保留原动作安全边界及这些历史失败；Executor 没有重放失败动作。

## 验证

此前默认并发完整 Rust **272/272 PASS**，最终默认 **240 PASS / 1 FAIL / KNOWN_TEST_INFRA_LIMITATION**；最终完整串行 **272/272 PASS**。Executor service **18/18**、domain **9/9**、Source policy **1/1**、桌面 **52/52**、Phase 2 repository/domain **30/6**、Phase 3 Planner service/domain **20/10**、Model Gateway **9/9** 与原 Trade Approval 回归 PASS。正常套件不启用真实 Gate；已单独显式启用的 AVD Gate PASS。Desktop TypeScript **63/63**、Contracts **9/9**、typecheck、前端/Rust 构建、macOS Tauri release app 打包、rustfmt、公开安全/脱敏/文档及 Diff Gate PASS。此前 Phase 3 夹具失败仍为历史记录，本轮没有削弱默认并发。生产只读 Gate 再次确认 Provider **0 / MODEL_NOT_CONFIGURED**，无迁移、配置修改或 Secret 访问。Lease panic/poison 与跳过下一 Step 的 Claim 测试 PASS。保留生产 Planner/Gateway 和四张既有工作台 JPEG。

见 [Current Status](../CURRENT_STATUS.zh-CN.md)、[Planner](M3_PLANNER.zh-CN.md)、[契约](M3_CONTRACTS.zh-CN.md)及[当日日志](../devlog/2026-09-27.zh-CN.md)。Phase 4 后停止；Phase 5 与完整自主验收需要另行所有者指令。


最终并发重跑说明：补齐 Lease panic/poison 和跳过下一 Step 的断言后，最终 `cargo test --workspace` 以 **240 PASS / 1 FAIL** 停止；失败为既有 `mobile::tests::concurrent_avd_lifecycle_fixtures_remain_independent_during_slow_tool_startup`（Fixture_AVD_3 AvdBootTimeout），剩余套件未运行。此前本阶段完整默认并发 **272/272 PASS** 为较早记录；当前默认并发结果为 **FAIL / KNOWN_TEST_INFRA_LIMITATION**，不是 PASS。无 Timeout 增大、强制串行化或真实 AVD 共享资源冲突。最终完整串行结果另行记录。
