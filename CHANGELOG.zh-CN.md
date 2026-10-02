# 更新日志

[English](CHANGELOG.md) | [简体中文](CHANGELOG.zh-CN.md)

从开源初始化开始，OrdinConn 的所有重要公开变更都会记录在这里。更早的开发历史仍保留在 Git 中，但本项目不声称重新构建了未曾记录的发布历史。

## 尚未发布

### 新增

- M3 有界 Planner/Executor 集成：持久化预算、Stop 优先级、绑定 Approval 和 Owner 条件验证。真实模型验收仍 BLOCKED_MODEL_NOT_CONFIGURED，Full Autonomous M3 为 NOT_COMPLETE。见 [Phase 5](docs/mobile/M3_PHASE5.zh-CN.md)。

- Issue-first 的公开开发流程和 GitHub 模板。
- 英文与中文项目入口文档。
- 公开状态、DevLog、ADR、安全规则和 Codex Field Notes 结构。
- Fail-closed 的工作区与 Git 历史安全门。
- 仅限文档的定时 GitHub 同步。
- 禁止自动发布的人工 X 草稿流程。
- 双语 Build in Public 语言规范、成对维护的核心文档和双语 Issue 记录。

## Mobile Interaction 与数据采集 — 2026-10-02

[Issue #14](https://github.com/Btkkgo/OrdinConn/issues/14) 在既有 Android Runtime 和 SQLite 上增加人工控制、本地保存的 `Observe → Interact → Observe → Diff → Extract → Data Object → Data Stream → Provenance` 闭环。数据链为 `Source → Observation → Extraction → Data Object → Insight → Plan → Action → Result`；本阶段实现到 **Data Object**，Insight/Plan 保留接口。Home 提供实际采集对象、元素检查与人工操作、确定性 Observation Context。敏感值在持久化前脱敏，确定性去重保留重复观察及原始证据。自动 fixture 验证与 **PENDING HUMAN ACCEPTANCE** 分开；生产动作沿用既有模拟器/Settings 安全范围。

M3 状态与历史、Provider、retry/backoff/deadline、Keychain 和保留的 App 均不变。自动真实 Android 动作 **0**；Codex 人工 Android 动作 **0**；真实 Provider 请求 **0**；签名验证 **0**；X Draft **NONE**。允许 Rust/frontend 编译；支持的包装器会执行 `codesign`，因此 macOS 验收打包 **NOT RUN / BLOCKED BY THE NO-SIGNING BOUNDARY**。参见[实现与人工验收](docs/mobile/MOBILE_DATA_ACQUISITION.zh-CN.md)。
