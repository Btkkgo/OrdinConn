# 架构

[English](ARCHITECTURE.md) | [简体中文](ARCHITECTURE.zh-CN.md)

OrdinConn V0.1 使用单进程嵌入式 Tauri Runtime，并保持严格分层：

`Core crates -> Application services -> Tauri adapter -> Typed IPC contracts -> React UI`

## 决策

- Core crates 负责领域规则，不依赖 Transport 或 UI Framework。
- `ordinconn-app` 负责 SQLite、Migration、Recovery、Service 和内存 Event Bus。
- Tauri 是 Host 与 Transport Adapter，对外提供稳定 DTO，而不是原始领域结构。
- 启动长任务的 Command 立即返回标识符，进度由 Event 持续传递。
- 一个公共 Event Envelope 承载 Agent、Model、Signal、Approval、Execution 和 System Event。
- 关系表回答当前状态；只追加的 `runtime_events` 解释状态如何变化。
- OpenAI-compatible Chat Completions 被隔离在 Provider Adapter 中。
- Approval 是绑定到一个规范化 Proposal Version 的一次性 Capability。
- V0.1 不包含 Localhost Server、Daemon、Sidecar、Distributed Bus、CQRS 或完整 Event Sourcing。

未来的 `StandaloneRuntimeHost`、`DaemonRuntimeHost` 和 `RemoteRuntimeHost` 必须复用相同的 Domain、Model、Tool、Approval 与 Signal Protocol。

## Crate 与传输边界

- `market-core`、`evidence-core`、`signal-core`、`traditional-finance` 和 `crypto-core` 负责金融领域定义。
- `model-gateway`、`agent-runtime`、`tool-runtime`、`approval-engine`、`execution-core` 和 `computer-use` 负责可替换的 Runtime Capability。
- `connector-runtime` 是 Connector Output 进入 Evidence 和 Signal Candidate 的唯一入口。Mock Connector 与未来真实 Connector 使用相同 Evidence Gate。
- `collector-runtime` 负责真实公开数据源定义、REST/WebSocket/Feed/HTML Adapter、Raw Record、Normalization、Deduplication、Health、Schema Drift 与 Retention Policy。
- `strategy-core` 负责带版本的确定性 Strategy、Rolling-window Contract、规范化 Instrument/Entity Alias、Parameter Snapshot、Reason Code 与 Rejected-run 语义。
- `ordinconn-app` 协调 Repository、Transaction、Recovery 和公共 Application Service。
- `apps/desktop/src-tauri` 将稳定 Command/Event 转换为 Tauri IPC。React 永远不会得到 Database Handle、Provider Secret 或 Approval Token。

## Runtime 生命周期与恢复

启动顺序为 Database Initialization、Migration、Runtime Initialization、Connector Initialization、Model Gateway Initialization、Event Bus Ready，最后 UI Ready。关闭时先停止新的 Agent Turn，取消活跃的内存任务，将活跃 Turn 以 Interrupted 状态和 Audit Event 持久化，最后关闭 SQLite。启动时也会对异常退出遗留的不安全 `running` 或 `waiting_tool` 状态执行 Fail-closed Recovery。

## Evidence 质量

Signal Engine 根据 Evidence Freshness、Source Reliability、Factual Level 和 Evidence Confidence 计算质量。Model Inference 无法单独通过 Publication Gate。冲突 Evidence 会保留关联、降低质量与置信度，并在适当情况下把 Signal 移入 Watch。V0.1 的公式有意保持简单且可替换。

Evidence Cluster 持久化 Original、Syndicated、Independent 和 Contradicting Member。只有不同的 Original/Independent Source 增加确认权重。应用将每个 Collection Boundary 写入关系表；已完成的 Signal 保留 Data Origin、Strategy ID/Version/Parameters、Reason Code、Observation/Source/Evidence ID、Baseline Window、Trigger Metric、Input Snapshot 和 Publication Time。

## Core Intelligence 生命周期

应用启动时，Catalog Initialization 注册已验证 Source 与不可变 Strategy Version。Event Forwarding 就绪后，Tauri Host 启动由 Application 管理的 `CollectorScheduler` 和 Binance Futures WebSocket。每个 Source Task 均为 Single-flight、全局有界、可暂停，并在 Shutdown 时等待完成；失败节奏采用有上限的指数退避和确定性 Jitter。Scheduler State 与 Aggregate History Bucket 在重启后恢复。每次 Collector Run 都记录成功/失败和 Source Health。Schema Drift 或 Rolling History 缺失会产生 Not-ready Strategy Run，而不会产生 Candidate；真实路径不得回退到 Mock。

## UI 与本地化

React 的所有正式 UI Label 都使用 Locale Key。`en` 与 `zh-CN` Dictionary 的 Key Coverage 完全一致；英文为默认语言，非法的已存偏好回退到英文，Settings 可立即切换语言，无需改变 Component 或 Runtime。视觉系统从用户提供的 OrdinConn 标识中提取紫色背景、黑色结构和黄色 Action Accent。原始图片保存在 `apps/desktop/src/assets/ordinconn-logo-source.jpg`，桌面 Icon 是同一素材转换得到的 PNG。
