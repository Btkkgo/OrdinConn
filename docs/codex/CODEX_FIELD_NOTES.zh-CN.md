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
