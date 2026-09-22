# Codex 现场笔记

[English](CODEX_FIELD_NOTES.md) | [简体中文](CODEX_FIELD_NOTES.zh-CN.md)

这些笔记描述实际观察到的工程协作过程，不构成产品背书。

## 当前阶段 — Mobile Intelligence M0 到 M1.5

### 做得好的地方

- Repository-first 检查让实现遵循当前产品基线，而不是旧概念。
- 明确的 Phase Gate 在真实 Android 环境缺失时阻止了 Mobile Navigation 工作。
- 小型 Contract Test 让 Privacy Redaction、Observation Lineage、IPC Projection 和 Shutdown Behavior 变得具体。
- 新一轮 Review 发现了仅靠绿色 Unit Suite 未暴露的问题：Subprocess Bound、Environment Consistency 与 Production-path Persistence。

### 容易跑偏的地方

- 第一次 M1.5 通过对部分 Production Acceptance Behavior 的验证过于间接。
- Test 可以证明受控 ADB Parsing，却仍可能缺少真实 Emulator Path。
- 大型 Prompt 会混合 Product Direction、Architecture、UI、Security 和 Acceptance Criteria；没有明确 Phase Boundary 时，容易过早实现后续阶段。

### 更好的 Prompt 模式

使用：`Read -> State the phase boundary -> Name observable acceptance checks -> Implement one boundary -> Run the exact checks -> Review -> Stop at the gate`。

提前要求证据格式。“展示十项命名 Gate Result”比“让 Android 工作”更有用。

### Token / Context 经验

把 Plan 和 Decision 固化到仓库文档中。重新读取一个聚焦的 Task Brief，比在每个 Prompt 中携带完整产品历史更省 Token，也更可靠。

### 验证技巧

完成声明的强度取决于支撑它的 Command。Fixture Test、成功 Build 和 Real-device Smoke 回答的是不同问题，必须分开报告。

### 当前对 Codex 的工程感受

Codex 最适合作为边界明确的工程协作者：它可以检查大型 Codebase、保留明确约束、编写 Test 并跨层迭代。当成功依赖不可用 Hardware、含糊的 External State，或一个 Prompt 同时要求多个未来阶段完成时，它更容易失准。Independent Review 与 Real-environment Evidence 仍然不可替代。

## Issue #4 — 绿色重跑不等于确定性测试

### Codex 起初可能作出的假设

串行通过、后续并行重跑变绿后，很容易把进程夹具问题当成机器偶发噪声。但这些都不是修复证据。

### 真实重复测试显示了什么

新鲜默认并行 Desktop 首轮再次以 24/28 失败于同样四个 Process/AVD 用例，随后九次热态重跑通过。四个用例各自单独运行 20 次也通过。受控的四夹具冷启动测试在原一秒成功预算下失败，改为有界测试专用预算后通过。关键区别是测试时序与生产 Deadline 行为，不只是串行和并行之别。

### 证据如何改变做法

Codex 保留短时限失败路径断言和全部生产 Command Deadline，增加独立并发夹具覆盖，并要求重复 Desktop/Workspace 测试及真实 Android Smoke。Smoke 首次正确拒绝白名单外的 Launcher；只有随后 Settings 前台运行才通过。两次结果都应记录。

## Issue #7 — 真实导航改变了判断

假 ADB 测试证明了命令形式，但专用 Android 16 AVD 暴露出两道不同边界：Settings Search 属于独立系统包；Android Task 复用也使“任意画面变化”不足以验证 OpenApp。最初真实尝试如实记为失败；加入目标包验证和固定 clear-top 启动后，完整门控流程才通过。单独的打包 App UI 检查先捕获到一次预期的过期快照拒绝；刷新后选择 Inspector 元素，才得到 `executed · VERIFIED`。

这轮有效的 Codex 做法是保留每次失败的确切证据，添加小型失败回归，再用真实设备复测。容易跑偏之处是把夹具通过、Bundle 成功或任意屏幕变化等同于 M2 验收。另一个小型隐私测试在公开文档定稿前发现被拒绝请求的标识可能进入 Receipt。用双语计划、聚焦测试命令和简短进度账本管理上下文，避免反复加载完整任务说明。M3 与 X 始终不在授权范围。
