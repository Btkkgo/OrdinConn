# Mobile Intelligence Runtime

[English](MOBILE_INTELLIGENCE.md) | [简体中文](MOBILE_INTELLIGENCE.zh-CN.md)

## 产品角色

Mobile Intelligence Runtime 是 OrdinConn 的一等 Collection Surface。它观察用户授权的 Android Application，生成 Structured Observation，并进入与 API、WebSocket、RSS、HTML、Browser 与 Desktop Source 相同的 Evidence/Strategy Path。

它不是 Simulator Manager、通用 Phone Assistant 或 Automation Demo。

规范流程：

`Device state -> Observe -> Structured UI state -> Policy check -> Agent decision -> Auditable action -> Post-action observation -> Verification -> MobileObservation -> Evidence -> Strategy -> SignalCandidate -> Published Signal`

M1 截止到 Observation，不执行 Tap、Swipe、Text Input、Application Launch 或 Navigation Action。

## Phase Scope

- M0：Contract、Product Policy、Security、Audit Event 与 Documentation。
- M1：Android Emulator Detection、Observe-only Session、Frame、Semantic UI Snapshot、Element Reference、Observation、Intelligence Feed、Warehouse 与 Three-column UI。
- M2：Verified Navigation。本阶段明确不包含。
- M3：一个 Production App Skill。
- M4：Observation-to-evidence Promotion 与 Mobile-backed Strategy Input。
- M5：Physical Android Device。

## 产品界面

主导航包含 Home、Warehouse 与 Settings。现有 Financial、Signal、Agent、Approval、Connector 与 Model Capability 保留在 Codebase 中，并集成到这三个 Surface，而不是删除。

Home 回答三个问题：

- 左：系统观察到了什么？
- 中：Mobile Runtime 当前在哪里观察？
- 右：Combined Evidence 可能意味着什么？

UI 不显示 OrdinConn Product Name、Version Copy 或 Logo Wordmark，只在 Navigation Rail 顶部保留 Application Icon。

## 架构边界

- `mobile-runtime` 负责 Platform-independent Domain Contract 与 Semantic Snapshot Construction。
- Tauri Adapter 负责 ADB Process Execution 与 Android-specific Parsing。
- `ordinconn-app` 负责 Persistence、Audit Event、Intelligence-feed Projection、Warehouse State、Research Task 与 Strategy Setting。
- React 使用 Typed IPC Contract，不运行 ADB。
- Mobile Observation 不能直接创建 Published Signal。未来 Evidence-promotion Phase 中，它必须通过 Connector Registry、Evidence Validation、Strategy 与 SignalCandidate。

## 验收边界

只有在可用 Android Emulator 上运行 `ORDINCONN_MOBILE_SMOKE=1`，才能验证 Real Emulator Support。Android SDK、ADB、Online Emulator 或 Allowed Application 缺失时会如实报告 Unavailable，绝不以伪造成功状态替代。
