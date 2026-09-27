# M3 Phase 2 — 手机执行契约与持久化

[English](M3_CONTRACTS.md) | [简体中文](M3_CONTRACTS.zh-CN.md)

> Phase 2 历史范围。有界单步执行现已在 [Phase 4](M3_EXECUTOR.zh-CN.md) 实现，完整自主 M3 仍为 NOT_COMPLETE。

## 范围与架构

Phase 1 已审计可复用的 M2 观察、Typed Actions、Safety、Receipt、Stop、事件、Model Gateway 和 Approval Engine。Phase 2 仅实现执行契约与持久化。自主执行和 M3 Planner 仍为 **NOT_IMPLEMENTED**。本阶段不调用模型、不配置 Provider、不执行设备动作、不运行真实 AVD Gate。

`MobileGoal -> MobilePlan revision -> MobilePlanStep -> Observation / Action Receipt -> Step Result / Evidence references`

Canonical Rust 契约位于 `crates/mobile-runtime/src/execution.rs`，`MobileGoalRepository` 位于 `crates/ordinconn-app/src/mobile_goals.rs`。沿用现有 SQLite、sqlx 迁移器、UUID v7、RFC 3339 时间格式、`runtime_events` 和提交后事件总线。React 通过 typed IPC 获取 camelCase DTO，不裁决执行状态、不直接查询 SQLite。

## Goal、预算与错误

Goal 保存强类型稳定 ID、原始 objective、可选 normalized objective、生命周期时间、active plan 引用、预算、deadline、计数器及 typed error code/message。原始目标按输入原样保存；纯空白或超过 1000 字符的目标会被拒绝。创建 Goal 不创建 Plan、不调用模型、不观察设备。

默认值由 Rust 管理：`max_steps=8`、`max_runtime_ms=120000`、`max_consecutive_failures=2`、`max_identical_observations=3`。验证范围分别是步骤 1–100、时间 1–3,600,000 毫秒、连续失败 1–max_steps、相同观察 1–100。字段会持久化，但不启动计时器或自主预算执行循环。首次转入 RUNNING 时建立 deadline，从审批状态返回时保留原 deadline。

| 当前状态 | 允许的下一状态 |
| --- | --- |
| PENDING | PLANNING、FAILED、STOPPED |
| PLANNING | RUNNING、FAILED、STOPPED |
| RUNNING | WAITING_APPROVAL、COMPLETED、FAILED、STOPPED |
| WAITING_APPROVAL | RUNNING、FAILED、STOPPED |
| COMPLETED / FAILED / STOPPED | 无 |

RUNNING 必须有 active plan。完成 Goal 要求 active plan 的所有步骤均为 VERIFIED 或 SKIPPED，并在同一事务完成 Plan。失败或停止会结束未完成步骤并使未完成 Plan 失效。终态 Goal 不得恢复运行；未来 Retry 必须建立新的明确 execution attempt，不能复活终态对象。

错误代码和固定可读消息分开保存。代码覆盖设备、模型、观察、动作、验证、审批、策略、预算、STALLED、USER_STOPPED、INTERRUPTED_BY_RESTART、INVALID_PLAN、INVALID_STEP、INVALID_STATE_TRANSITION。原始 Provider 响应、UI 文本和 objective 不复制到错误或生命周期审计 payload。

## Plan revision 与 Step

同一 Goal 的 revision 单调增加，以 `UNIQUE(goal_id,revision)` 保证唯一。Plan 状态为 DRAFT、ACTIVE、COMPLETED、SUPERSEDED、FAILED。激活在同一事务中 supersede 旧 active revision、激活新 revision、更新 Goal 指针并追加事件。不能回退到旧 revision，也不能在旧 Step 执行或等待审批时替换 Plan。旧 Plan 保留。

Step 保存强类型 ID、所属 Plan、唯一 sequence、semantic type、status、reason、持久化 risk、target/input、typed expected result、时间、追踪引用和错误。类型为 OBSERVE、SCROLL_DOWN、SCROLL_UP、TAP_ELEMENT、INPUT_TEXT、BACK、WAIT、EXTRACT、COMPLETE、STOP。这些是语义契约，绝不是 shell/ADB command payload；Phase 2 不把它们映射成运行时动作。

| 当前 Step 状态 | 允许的下一状态 |
| --- | --- |
| PENDING | EXECUTING、WAITING_APPROVAL、SKIPPED、STOPPED |
| WAITING_APPROVAL | EXECUTING、FAILED、STOPPED |
| EXECUTING | VERIFIED、FAILED、STOPPED |
| VERIFIED / FAILED / SKIPPED / STOPPED | 无 |

只有 active plan 能推进步骤，前序步骤必须先 VERIFIED/SKIPPED；同一 Plan 最多一个步骤执行或等待审批。VERIFIED 必须通过原子结果持久化获得，不能单独更新状态。关联的 pending action intent 不能冒充已验证执行。Result 引用必须与 Step 一致，Evidence 必须已关联到该 Step。

Risk 为 READ_ONLY、REVERSIBLE、APPROVAL_REQUIRED、FORBIDDEN。FORBIDDEN 不得进入 EXECUTING。持久化 risk 和状态不代表执行授权：后续 Rust Safety 仍必须独立分类，Approval 仍必须校验真实 capability。本阶段没有执行入口。

ExpectedResult 是有限 tagged enum：UI_CHANGED、ELEMENT_VISIBLE、TEXT_EQUALS、ACTIVITY_CHANGED、ACTIVITY_EQUALS、NEW_DATA_OBJECT、NO_CHANGE_EXPECTED。持久化 input 和文本相等值复用既有安全输入验证。密码字段、私钥、助记词和私密数据访问仍被禁止。未来验证仍须使用可信 Runtime 和隐私策略，本契约不实现语义验证。

## 持久化与追踪

迁移 `0008_mobile_goal_contracts.sql` 新增 `mobile_goals`、`mobile_plans`、`mobile_plan_steps`、`mobile_step_results` 以及小型关系表 `mobile_step_evidence`。Canonical domain JSON 存在 SQLite 中，辅以索引关系字段和一致性检查；没有独立 JSON 存储，也没有第二套数据库。

外键分别关联 Plan→Goal、Step→Plan、现有 mobile observation、现有 action receipt、现有 Evidence。active plan 的复合外键保证 Plan 属于同一 Goal。同一 Receipt 只能关联一个 Step。Evidence 关系保留来源 Observation ID。Result 只引用既有对象，不向 Goal/Result 复制 frame 或 UI tree。Repository 提供关联查询，包括 `get_step_evidence`、`get_step_result`。

创建、激活、状态变更、关联和结果写入均使用 `BEGIN IMMEDIATE`，与审计事件同事务提交；提交后才发布事件。故障注入验证审计插入失败时全部回滚。外键使用 RESTRICT，执行记录有禁止删除触发器，Result/Evidence 关系不可修改。本阶段不提供 retention/deletion API。

生命周期事件沿用命名：`mobile.goal_created/planning/started/waiting_approval/completed/failed/stopped`、`mobile.plan_created/activated/superseded/completed/failed`、`mobile.step_created/started/waiting_approval/verified/failed/stopped/skipped`。复用原 `runtime_events` aggregate/entity 引用；typed payload 携带可选 goalId、planId、stepId、observationId、actionId、evidenceId，不建立另一张手机事件表。

## 启动恢复

初始化将 PLANNING、RUNNING、WAITING_APPROVAL Goal 转为 FAILED，错误为 INTERRUPTED_BY_RESTART。正在执行或等待审批的 Step 失败，其余未完成步骤停止，未完成 Plan 失败；同一事务记录事件。恢复幂等，保留 pending/terminal Goal，不重放动作、不调用模型。

## Approval subject 与模型边界

`MobileApprovalSubject` 包含 Goal ID、Plan ID、Step ID、语义 action type、可选 target hash 和 Observation ID。确定性 SHA-256 hash 包含版本化域分隔符 `ordinconn.mobile-approval-subject.v1`，每个绑定字段都会影响 hash。不可变 Plan ID 标识对应 revision，新 revision 获得新 ID。

Subject 不等于 capability。Trade Approval 继续使用既有 TradeProposal、hash、expiry、nonce、issuer 和单次消费路径；没有第二个引擎。后续必须谨慎扩展现有引擎的 subject 抽象，保持 Trade Approval 语义，并在 Observation/Plan/Target 改变后使旧手机绑定失效。

既有聊天路径 `ordinconn-app/services.rs::finish_agent_turn` 仍调用 MockModelAdapter，属于本阶段范围之外。**M3 Planner MUST NOT use MockModelAdapter**；Phase 3 必须通过 Model Gateway 使用真实模型。观察到的生产 Provider 数为 0；预留 MODEL_NOT_CONFIGURED，但 Phase 2 没有 Planner 或 execute command。

## Desktop 与 Research Task 兼容

IPC 只提供 create_mobile_goal、get_mobile_goal、list_mobile_goals、get_mobile_goal_plan。Home Enter 与按钮使用相同 form 提交路径，保留原始 objective，持久化 canonical PENDING Goal。保留现有卡片 markup/classes、三列布局、样式、导航及手动控件。无 Plan 的 Goal 显示 locale 文案“等待规划”，它是 Goal 卡片，不是伪造的 Plan。

Goal 从 Rust 持久化重新加载。契约元数据事件刷新 Goal，不冒充设备正在观察。原 ResearchTask 持久化及详情页研究命令保持可用。历史 research task 不偷偷升级为 Plan、不重复创建 Goal，原记录仍可读取；只有新的 Home 手机目标改用 MobileGoal。

## 验证与下一边界

最新测试和构建结果见[每日 DevLog](../devlog/2026-09-27.zh-CN.md)。测试使用隔离 SQLite 和合成观察/Receipt，不使用真实设备。迁移测试先运行旧迁移至版本 7、写入旧数据，再运行当前迁移器，检查事件、观察、收藏/保存的手机 feed、研究任务、设置和外键完整性。

既有并发 AVD fixture 时间问题保留为 KNOWN_TEST_INFRA_LIMITATION。本阶段未修改其预算或工作区并发设置。真实 AVD/模型/自主验收仍为 NOT RUN / NOT_IMPLEMENTED。生产迁移将在以后启动应用时发生，Phase 2 验证不修改生产数据库。

Phase 2 完成后停止，等待 owner 验收。Phase 3 为 Real Model Planner + Strict Typed Next Action；Executor、Stop/Approval 集成及自主实机 Gate 属于后续工作。Commit NONE / Push NONE。
