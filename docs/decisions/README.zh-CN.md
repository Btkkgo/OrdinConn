# 工程决策

[English](README.md) | [简体中文](README.zh-CN.md)

已接受的 Architecture Decision Record：

- [ADR-0001：模型无关 Runtime](ADR-0001-model-agnostic-runtime.md)
- [ADR-0002：Local-first Perception](ADR-0002-local-first-perception.md)
- [ADR-0003：默认不持续录屏](ADR-0003-no-continuous-screen-recording.md)
- [ADR-0004：以 GitHub Issue 作为开发台账](ADR-0004-github-issues-as-development-ledger.md)

每份短 ADR 在同一文件中使用 `English` 与 `中文` 两个章节。

## 当前决策

### Evidence 不等于 Inference

Model Output 可以解释或综合信息，但不能满足事实 Evidence Gate。Published Signal 保留 Evidence Link 与 Contradiction。

### Signal 不等于 Trade

Trade Proposal 必须经过显式 Approval。V0.1 只提供 Paper Execution；Approval Capability 精确绑定 Object 与 Version，有时限且只能使用一次。

### Model 与 Source 使用 Gateway

所有 Provider 通过 Model Gateway。所有 Collection Source 通过 Connector Registry。Domain Runtime 不直接依赖 Vendor Response Shape 或临时 Source Implementation。

### Local-first 单进程 Runtime

V0.1 使用 Tauri 单进程嵌入式 Rust Runtime、SQLite Current State、Append-only Audit Event 与 Typed IPC。Daemon、Distributed Bus、完整 Event Sourcing 和 Remote Host 不是当前架构。

### Readiness 与 Evidence Validation 分离

Strategy 可能因为 Input 缺失、过期、非法或历史不足而不能运行。该状态会被记录，但不会伪造 Candidate。

### Mobile Observation 是独立边界

`MobileObservation` 既不是 Evidence，也不是 Signal。最早要到 M4 才计划增加显式 Observation-to-Evidence Promotion Path。

### 真实 Integration Gate 不能由 Fixture 代替

Fixture 验证 Parsing 与 Policy。Real-environment Claim 必须使用真实环境与 Production Path。

### Structured Perception 优先于 Vision

长期 Perception Design 优先使用 Event 与 Accessibility/UI Tree，再使用 Bounded Image Capture 和 OCR/Vision。这能降低成本并提供更清晰的 Privacy Boundary。

### 公开摘要，而不是原始对话

公开开发记录包含 Technical Goal、Inspected Area、Change、Command、Result、Risk 与 Lesson，不包含 Raw Prompt、Private Chat、Credential 或 Machine Identity。
