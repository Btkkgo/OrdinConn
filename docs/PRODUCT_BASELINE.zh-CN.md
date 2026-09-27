# OrdinConn 产品基线 — V0.1 Foundation

[English](PRODUCT_BASELINE.md) | [简体中文](PRODUCT_BASELINE.zh-CN.md)

> 之前的 OrdinConn 产品基线均已废止。本文件是 OrdinConn V0.1 的唯一产品基线。

## 产品

OrdinConn 是 AI 金融情报与执行 Agent：主动采集市场信息、形成证据、识别可解释信号、生成报告，并仅在用户明确批准后执行允许的动作。

两个市场为 Traditional Finance 与 Crypto，共享流程：

`Data -> Evidence -> Agent Analysis -> Signal -> Report -> User Approval -> Action -> Result -> Memory`

## V0.1 范围

包含桌面 Shell、Agent Runtime、Model Gateway、Tool Runtime、Approval Engine、Evidence 与 Signal 领域、传统金融与加密体验、上下文 Agent Dock、SQLite 持久化、Mock Connector、公开数据 Collector Runtime、Source Registry、确定性 Strategy Engine、至少 18 个演示信号、Agent Report、Trade Proposal、Approval Capability 和 Paper Execution。

Mobile Intelligence 增加 Android Emulator 观察、语义 UI Snapshot、脱敏 Mobile Observation，以及 Home/Warehouse/Settings Shell。已验证 M2 增加六种人工触发且绑定快照的动作（Tap、Swipe、安全 Type、Back、Home、OpenApp），执行默认拒绝与白名单、敏感/金融目标拒绝、输入前审计、动作后观察和回执。M2 不增加自主手机执行。手机数据仍通过 Source Registry、Evidence、Strategy 与 Signal 准入。

不包含真实经纪商或交易所执行、实盘交易、充值、提现、转账、钱包签名、私钥或助记词访问、订阅、云平台、高频后台自动化、高级计算机视觉，以及自主登录、发布、消息发送、下单或超出 M3 有界 Settings 目标范围的操作。

## 市场与 ABC

传统金融涵盖股票、ETF、期货、石油、黄金、白银、外汇、债券与商品：A — Trading Signal；B — Event Signal；C — Demand Signal。

Crypto 涵盖主要资产、稳定币、NFT/Ordinals、Meme、现货、永续与期权：A — On-chain Signal；B — Event Signal；C — Exchange Signal。

信号必须说明方向、置信度、紧迫性、时间范围、证据、催化剂、风险、失效条件与观察。仅买入/卖出标签不是有效信号。

## Evidence 策略

Connector 与 Agent 生成 Signal Candidate。只有通过验证且至少关联一条非 `MODEL_INFERENCE` Evidence 后，Candidate 才能成为 Published Signal。推断可以解释、综合、形成传导路径或假设，不能冒充来源事实。

Evidence 关联分为 `primary`、`supporting`、`contradicting`、`context`。冲突保持可见。最终质量由 Signal Engine 计算，不由模型决定。不充分 Candidate 不能生成正式报告、提案、审批或执行。

## Agent 与模型策略

Rust Agent Runtime 使用 Thread、Turn、Item、Tool、Approval、Event 协议，支持流式输出、标准化工具调用、上下文注入、中断、取消、重试边界、类型化错误与已完成 Item 持久化。

所有模型通过 Model Gateway。V0.1 实现可配置 OpenAI-compatible `/v1/chat/completions`，支持非流式、流式与明确声明的工具调用。Agent Runtime 不接触厂商响应格式；Provider 能力明确声明。Mock Model 在无凭据时维持演示可用。

默认语言为 English，完整支持 English 与简体中文 `zh-CN`，可在设置中即时切换。正式文案使用 Locale Key，无效或缺失偏好回退 English。领域、API、文件、代码、Agent 协议、Signal、Evidence 与 Report 字段采用 English。

## Runtime 与持久化

采用 Tauri 单进程内嵌 Rust Runtime。React 只通过类型化命令和事件通信，不直接访问 SQLite、Provider、文件或核心状态。核心 Crate 与传输无关。

SQLite 关系表存当前状态；`runtime_events` 存仅追加审计；实时增量使用内存 Event Bus。启动时将不安全未完成工作标为 Interrupted，不自动重放有副作用工具。

公开数据遵循 `Continuous Collection -> Rolling History -> Baseline -> Ready Strategy -> SignalCandidate -> Evidence Gate -> Published Signal -> Agent Analysis`。Runtime 管理按来源调度、有界历史、重启恢复聚合桶、长连接 WebSocket 重连/重订阅。Binance Spot 与 USDⓈ-M 使用独立标准化 Instrument。REST、WebSocket、RSS/Atom、HTML 与白名单仅观察手机 Collector 明确公开数据策略、健康、Schema Drift、速率预算、保留与确定性来源可靠性。禁止登录、付费墙、CAPTCHA、Cookie Gate、私人访问、私人消息与敏感字段采集。

Strategy Readiness 与 Evidence 验证分离。`WARMING_UP`、`MISSING_INPUT`、`STALE_INPUT`、`SCHEMA_ERROR`、`INSUFFICIENT_HISTORY` 只生成可审计 Strategy Run，不生成普通 Candidate。真实 Published Signal 只能包含真实 Evidence；无就绪阈值触发时零真实 Signal 是有效结果。

## Approval 与执行

Signal 不等于 Trade。Published Signal 可创建版本化 Trade Proposal，独立 Approval Request 绑定精确 Canonical Proposal Hash。

Approval Capability 为单次、对象绑定、版本绑定、限时、不可转移。React 不能签发或保存；SQLite 只保存摘要。校验、消耗和起始 Execution Record 创建具有原子性。模糊或无效条件全部拒绝。

Paper Execution 是唯一 V0.1 执行适配器，必须通过真实 Approval；无有效 Approval 就无 Execution。

## UI

主导航为 Home、Warehouse、Settings。Home 集成情报、手机画面、关联 Signal 和上下文数据讨论。市场、Signal、Agent、Automation、Model、Source 与 Approval 能力通过已有整合工作区保留，不增加主导航入口。

主页以用户提供的实时工作台 Reference A（视觉）和 Reference B（结构）为批准方向，替代之前的主页视觉基线。采用克制黑灰、细圆角边框、主/次级文字、蓝色选中态和绿色设备连接点。三栏为实时数据、手机操作与 Agent 指令、Agent 计划。不新增分类图标、彩色 KPI 海洋、渐变、Logo 或装饰图表。仓库、设置和既有导航保留；正式文案使用双语 Locale Key，其他工作区保留原品牌体系。明确选择的开发视觉 Fixture 不得填充生产状态。

## 验收

应用启动后显示批准的深色主页工作台并保留仓库/设置。需实际 Tauri 截图对照、真实设备诊断/Frame、证据关联且去重的已加载对象统计、类型化人工命令与保留 Approval/Safety。Pending 研究目标不等于自主计划或执行；未支持的自主能力必须明确呈现。

用户可切换双语、访问两市场与六 ABC Lane、打开 Signal、检查 Evidence、使用上下文 Agent、生成 Report、创建 Paper Trade Proposal、请求与批准后取得 Paper Execution Record。Model Settings 可配置 OpenAI-compatible Provider。本地最终 Commit 前测试与构建必须通过。

## M3 有界目标扩展

所有者授权的 M3 Phase 5 将真实 Model Gateway Planner 接入既有真实单步 Executor，仅用于 Android Settings 及其系统预装搜索。Typed Next Action 保留默认拒绝策略、最新 Observation 绑定、显式 Approval、统一 Step/Model/Action/Time Budget、精确验证、Stop 及重启失败即拒绝。一般目标完成必须具备所有者显式提供且不可变的 Activity/Text 完成条件；模型自行宣称完成无效。现有 Home 提交接入 Rust Runner，无 UI 重设计。生产 Provider 未配置时返回 MODEL_NOT_CONFIGURED，不调用模型或设备动作。Live 验收为 BLOCKED_MODEL_NOT_CONFIGURED；Full Autonomous M3 仍为 NOT_COMPLETE。M4、账户、外部 App、真钱与商业授权不在范围内。
