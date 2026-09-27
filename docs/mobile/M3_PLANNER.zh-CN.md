# M3 Phase 3 — 真实 Model Gateway Planner

[English](M3_PLANNER.md) | [简体中文](M3_PLANNER.zh-CN.md)

> 当前 Phase 5：有界 Planner/Executor 集成已实现；真实模型验收 **BLOCKED_MODEL_NOT_CONFIGURED**（Production Provider Count **0**），Full Autonomous M3 **NOT_COMPLETE**。下文较早的 Phase 2–4 验证记录保留为历史证据；最新验证见 [Phase 5 收口记录](M3_PHASE5.zh-CN.md)。


> Phase 3 仅规划的历史范围。[Phase 4 Executor](M3_EXECUTOR.zh-CN.md) 现已实现显式单步执行，完整自主 M3 仍为 NOT_COMPLETE。

## 范围与架构

Phase 3 实现显式、受限规划，**不执行手机动作**。自主执行仍为 **NOT_IMPLEMENTED**；Phase 4 和 Phase 5 不在本轮范围。参见 [Phase 2 契约](M3_CONTRACTS.zh-CN.md)与[当前状态](../CURRENT_STATUS.zh-CN.md)。

`MobileGoal + 已保存 Observation + 有限执行历史 -> Model Gateway -> 不可信 JSON -> Rust 严格解码 -> Domain/Safety 校验 -> 同事务保存 Plan/单个 PENDING Step 或不执行的 Proposal`

Service 位于 `ordinconn-app/mobile_planner.rs`。Canonical decision、Schema、验证与单一版本化 `MOBILE_PLANNER_V1` Prompt 位于 `mobile-runtime/planner.rs`、`mobile_planner_v1.txt`。Provider 请求 JSON 留在 `model-gateway`。既有聊天 MockModelAdapter 不属于此链路；M3 不导入或回退到它。确定性 Test Adapter 只存在于 `#[cfg(test)]` 模块。

## 输入与隐私

Context 包含 Goal ID、最小化 objective、Goal 状态、最新 revision、step budget、全部已使用 Step 数、剩余预算、observation_missing、当前 Observation、最近 Step/失败码、允许动作、禁止能力和现有应用白名单。不向模型提供截图、完整数据库历史、凭据、Tool、文件系统、Tauri、shell 或 ADB 能力。

Observation 保留 observation/snapshot/session ID、采集时间、package/activity、屏幕尺寸，以及最多 **80** 个语义元素引用，不包含元素坐标。复用已有 Runtime 脱敏词表及标记，拦截敏感页面/目标。已保存 Observation 的 Privacy 必须为 PUBLIC 或 USER_ALLOWED；SENSITIVE 或未知/缺失分类在模型调用前拒绝。全部可编辑字段内容均不发送，包括没有 password 标记但可能含验证码的字段。安全可见标签最多 256 字符。安全 objective 保留完整的有界 Goal 文本（最多 1,000 字符），包括中文目标；只有 UI label 使用较短投影。疑似敏感 objective/metadata 保守脱敏；这可能省略无害的长文本或安全相关文本，不声称超出该 fail-closed 策略的语义隐私保证。

History 最多 **5** 个 Step，只包含状态/类型、错误码、Observation 引用，不含旧模型 reason 或输入内容。Context 序列化上限 **32 KiB**，完整历史计数仍用于预算。无 Observation 时明确标记，只能规划 OBSERVE/WAIT；本阶段不重新 Observe。未来 Executor 仍须校验设备身份、时效、前台、bounds、焦点和权限。

## Provider Resolution 与 MODEL_NOT_CONFIGURED

Settings 当前保存启用 Provider 及默认模型，没有独立 Planner 选择项，因此要求**恰好一个启用 Provider 且默认模型非空**。Provider 总数为 0 返回 `MODEL_NOT_CONFIGURED`；没有启用项、启用项多个或选中模型为空返回 `MODEL_NOT_SELECTED`。不支持的 Adapter/Capability 与服务失败使用固定 typed error，不随机/按字母选择、不硬编码厂商、不自动创建 Provider、不带开发 Key、不联网发现模型。

M3 复用 OpenAI-compatible Adapter 与安全凭据存储。有 credential reference 时按现有 Provider ID 查询 keyring；凭据值/reference、完整 Prompt、服务错误正文与原始模型输出均不写 audit。直接丢弃 Provider 错误正文，模型回复回显所提供凭据时拒绝持久化。

当前生产 Provider Count 为 **0**，通过只读 SQLite 核对。共享 `plan_mobile_goal` service 在读取 Goal 或写入前返回 `MODEL_NOT_CONFIGURED`。`mobile_planner_provider_gate` 不初始化 AppRuntime、不迁移、不读凭据、不调用模型。Typed IPC helper 另在隔离且已迁移数据库中使用既有 Goal 验证：没有假 Plan/Step，Goal 保持 PENDING。该 Gate 未启动应用。

## 严格输出 Schema 与本地验证

根对象固定为 `decision`、`action`、`completion`、`failure`，未使用项必须为 null。decision 只能是 `next_action`、`complete`、`cannot_proceed`；所有对象拒绝未知字段。原生 Schema 使用 root object、required 字段和嵌套 union，符合官方[结构化输出子集](https://developers.openai.com/api/docs/guides/structured-outputs)。

只允许一个候选动作：OBSERVE、SCROLL_DOWN、SCROLL_UP、TAP_ELEMENT、INPUT_TEXT、BACK、WAIT、EXTRACT；每个都要求非空有限 reason 和 Phase 2 Typed ExpectedResult。TAP/INPUT 引用必须来自当前 Observation；INPUT 还要求可编辑、启用、非敏感目标与 Runtime-safe 文本。Scroll 不接受坐标；BACK 必须使用兼容 UI/activity expectation。WAIT 为 **1–5,000 ms**。EXTRACT 只保存 extraction intent 与 NEW_DATA_OBJECT expectation，不生成 Data Object/Evidence。COMPLETE、STOP 不属于 Planner Action。

Rust 将 OBSERVE/WAIT/EXTRACT 重新计算为 READ_ONLY，将安全导航/输入计算为 REVERSIBLE。模型 risk 声明属于未知字段，直接拒绝。敏感、资金、消息、发布、注销、不可逆 intent/target fail closed。应用白名单未扩大，规划并不是执行许可或 Approval capability。

拒绝非法/空/Markdown JSON、多动作、任意动作名、额外安全字段、坐标、shell/ADB/JavaScript、伪造引用、危险输入、不兼容 expectation、无依据完成引用、超大输出以及 Provider tool/refusal/截断回复。不修复 JSON；原生 Schema 永远不能替代 Rust 本地验证。

## Structured Output 与边界

Capability 来自用户已有 Settings 配置，不按厂商名称猜测。`structured_output=true` 使用 strict `response_format=json_schema`；否则 `json_mode=true` 使用 `json_object`；两者皆无时只发送相同 JSON-only Prompt 和完整 Schema，不发送不受支持的 response_format。所有模式经过同一严格 Decoder 和 Domain/Safety 校验。Provider 拒绝其声明的原生能力时返回 MODEL_ERROR，不自动降级掩盖失败。

限制：本地 INVALID_MODEL_OUTPUT 最多 **2 次尝试**；跨全部尝试共 **30 秒**并使用 Gateway timeout；**2,048 output tokens**；**64 KiB HTTP response**；**16 KiB 候选 JSON**；**32 KiB Context**；**5** 个最近 Step；**80** 个元素；WAIT 最多 **5,000 ms**。Domain/Policy 错误不修复、不重试。在网络/凭据访问前检查预算、terminal/approval 状态、deadline，以及现有 pending/executing/waiting Step。应用 shutdown 取消请求 Future；完整 Stop 优先级仍属于 Phase 5。

## 持久化、状态与 Runtime Events

合法动作创建下一个单调 Plan revision，恰好一个 PENDING Step，全局 sequence 为 `steps_used + 1`。保留旧已验证 revision；只有无 pending/executing/waiting Step 时才能 supersede 旧 active plan。WAIT 时长、extraction intent 以可选字段扩展 canonical Step JSON，与旧记录兼容。Goal/Plan 保存原始 objective。

只有成功事务中才执行 PENDING -> PLANNING；active pending Step 等待未来 Executor 时 Goal 保持 PLANNING。本阶段不进入 RUNNING、不设置 started_at。现有面板通过 locale key 显示“规划已就绪，等待执行”，不新增按钮或修改布局。模型失败保留之前 Goal 状态，不造成新 Goal 无限 PLANNING。既有 restart recovery 继续 fail closed、不重放动作。

迁移 **0009** 增加不可变 `mobile_planner_decisions`，保存 Proposal/Outcome、版本/Provider metadata，以及 Goal/Plan/Step/Observation 外键；本轮只在隔离数据库应用迁移。Plan/Step、Outcome 和 `mobile.planner_succeeded` audit 同事务提交。Repository 在 BEGIN IMMEDIATE 中重新检查状态、预算、白名单、Observation 和 revision。并发/过期候选不能保存两个 pending Step；注入 audit 失败会回滚全部写入。

事件复用已有命名/Family：`mobile.planner_started`、`mobile.planner_succeeded`、`mobile.planner_failed` 及 Phase 2 Goal/Plan/Step events。Payload 只包含引用、Provider/model ID、Planner/Schema version、耗时和 Outcome/Error code，不包含 objective、Prompt、屏幕/输入文本、凭据、原始输出。模型尝试前的 preflight 拒绝不会产生 started event 或生产写入。

## Completion Proposal 与 Cannot Proceed

COMPLETE 要求有限 reason，以及来自当前 Observation 或最近 VERIFIED 历史的非空 supporting ID。只保存 **Completion Proposal**，绝不把 Goal 改为 COMPLETED，也不创建 COMPLETE Step。Phase 4 必须独立验证 completion criteria。

CANNOT_PROCEED 只接受 NEEDS_NEW_OBSERVATION、TARGET_NOT_FOUND、INSUFFICIENT_CONTEXT、GOAL_UNSUPPORTED、SAFETY_BLOCKED；保存 Proposal，不生成 Plan/Step 或虚构 Evidence。

## Typed IPC 与 Test Model 边界

`plan_mobile_goal` 输入为 `{ input: { goalId, observationId? } }`，返回 decision、引用、waitingExecutor 和可选 Proposal DTO。TypeScript 只表达 DTO，不新增 Executor endpoint、按钮、Enter 自动规划、自主循环或模型驱动 UI 流程。Home 仍只创建 pending Goal，不产生模型费用。

测试使用隔离 SQLite、仅 cfg(test) 的确定性输出，以及驱动真实 Gateway/生产 Service 的本地 HTTP server。这些证明架构、format 选择、本地拒绝、timeout、大小、取消、事务、历史/revision 与无执行，不代表真实付费 Provider 验收。不需要或使用 AVD。

最新计数及构建/Gate 证据见[当天 DevLog](../devlog/2026-09-27.zh-CN.md)。Live Provider Gate 为 **NOT_RUN_MODEL_NOT_CONFIGURED**；完整 M3 在 [Issue #12](https://github.com/Btkkgo/OrdinConn/issues/12) 保持 OPEN / NEEDS VALIDATION，#11 保持 OPEN。Commit **NONE** / Push **NONE**。Phase 3 后停止；Phase 4 Executor 与 Phase 5 Stop/Approval 集成尚未实现。
