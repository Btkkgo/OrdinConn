# 公开架构

[English](OVERVIEW.md) | [简体中文](OVERVIEW.zh-CN.md)

## 产品边界

OrdinConn 是一个开源、模型无关的 Agent Runtime 方向。V0.1 将该 Runtime 用于 AI Financial Intelligence and Execution Agent；当前产品尚不是通用电脑控制平台。

已验证的金融流程为：

`Data -> Evidence -> Agent Analysis -> Signal -> Report -> User Approval -> Action -> Result -> Memory`

真实资金执行、Wallet Signing、Transfer、Private-key Access 和自动 Mobile Control 均不属于 V0.1。

## Runtime 分层

`Core crates -> Application services -> Tauri adapter -> Typed IPC -> React UI`

- Core crates 负责领域规则，不依赖 Tauri 或 React。
- `ordinconn-app` 负责 SQLite、Migration、Application Service、Recovery 和内存 Event Bus。
- Tauri 负责 Host Integration 和稳定的 Command/Event DTO。
- React 使用 Typed IPC，不会得到原始 Database、Provider Secret 或 Approval Capability。

## 可替换的 Model 与 Source

Model Gateway 在 Agent Runtime 使用 Provider 前统一其 Capability、Request、Streaming 和 Tool-call 行为。当前仓库实现 OpenAI-compatible Adapter 与 Mock Provider；支持其他具体 Model Family 是方向，不是已实现清单。

Connector Registry 是唯一 Source 入口。Public Collector 规范化 Raw Record、跟踪 Health 和 Schema Drift、去重 Observation，并向确定性 Strategy 提供输入。Model Inference 不能冒充 Source Evidence。

## Evidence 与 Execution

`SignalCandidate` 只有在验证通过并至少关联一个非推断 Evidence 后才能成为 Published Signal。冲突 Evidence 会继续保留。Signal Quality 由 Signal Engine 计算。

Signal 不等于 Trade。带版本的 Proposal 必须创建 Approval Request；Paper Execution Adapter 执行前，必须原子消费绑定到精确 Proposal 的一次性 Capability。

## Mobile Intelligence

Mobile 路径按阶段推进：

`Device state -> Observe -> Structured UI state -> Policy -> MobileObservation`

未来阶段可能增加：

`Agent decision -> Action -> Post-action observation -> Verification -> Evidence -> Strategy -> SignalCandidate`

当前 M1 代码仅观察。Android-specific Process Execution 保持在 Tauri Adapter；平台无关的 Mobile Contract 保持在 `mobile-runtime`；Persistence 与 Workspace Projection 保持在 `ordinconn-app`。

## Perception 方向

预期的本地 Perception 顺序为：

`System events -> Accessibility/UI tree -> Bounded capture -> Change detection -> OCR/Vision`

这是设计方向，不表示所有 Desktop Layer 已经实现。Continuous Recording 不是默认设计。Structured State 成本更低、更易审计，也更容易受 Privacy Policy 约束，因此优先使用。

## 隐私模式

计划中的 Perception Mode 为 Manual Capture、Application Allowlist 和显式 Work Session。Password Field、Banking、Payment、Wallet、Recovery Phrase、Verification Code、Password Manager、Private Browsing 和 Camera Video 默认禁止。

Mobile M1 已强制 Application Allowlist 与 Sensitive-node Redaction。更广泛的 Desktop Perception Mode 仍属于设计工作。
