# OrdinConn

[English](README.md) | [简体中文](README.zh-CN.md)

**开源、模型无关的 AI Agent Runtime。**

OrdinConn 的目标，是为 AI 模型提供一层能够感知、理解并操作电脑与移动设备环境的运行时。当前 V0.1 应用首先把这套运行时用于传统金融与加密市场的可追溯情报处理。

OrdinConn 不是另一个聊天机器人。它关注的是模型推理与真实工具之间那层可审计、可授权、可验证的基础设施。

```text
AI Model
   ↓
Reasoning
   ↓
OrdinConn Runtime
   ↓
Perception
   ↓
Tools / Applications
   ↓
Action
   ↓
Verification
```

## 为什么开发 OrdinConn

模型可以替换，但工具调用、权限、证据来源与结果验证需要稳定协议。OrdinConn 把这些能力从具体模型中分离出来，让 Agent 能读取真实状态、只在授权范围内行动、验证结果，并留下完整审计记录。

## 架构

Rust 核心保持与传输层无关；React 通过类型化 IPC 与 Tauri Host 通信。所有模型统一经过 Model Gateway，所有数据源统一经过 Connector Registry。

当前金融智能闭环为：

```text
Data → Evidence → Signal → Agent Report → Trade Proposal → Approval → Paper Execution
```

参见[架构总览](docs/architecture/OVERVIEW.md)和[产品基线](docs/PRODUCT_BASELINE.md)。

## 当前状态

- 版本：`0.1.0`
- Mobile 阶段：M1/M1.5 环境验证
- M1.5 Gate：**未通过**
- M2：**尚未开始**
- 真实资金执行：未实现，V0.1 仅支持 Paper Execution

最近一次真实 Android Gate 没有发现可用的 Android SDK、ADB、Emulator、AVD 或在线模拟器：3 项前置检查失败，7 项依赖检查被阻断。详细边界见[当前状态](docs/CURRENT_STATUS.zh-CN.md)。

## Mobile Intelligence

M0/M1 已实现设备会话契约、受限 Screen Frame、语义 UI Snapshot、作用域化 Element Ref、敏感节点脱敏策略、类型化 IPC 投影、持久化和 `MobileObservation`。

真实 Frame Capture、UI Tree、敏感节点脱敏验证、Observation 生成和关闭链路仍被 Android 运行环境阻断。Fixture 测试不能替代真实设备证据。

## Computer Runtime

OrdinConn 定义了受限的电脑感知与操作接口、权限门、审计事件和操作后验证。设计目标不是持续录屏或无限制自动化。当前 Computer Runtime 仍属部分实现，详见[路线图](docs/roadmap/README.md)。

## 模型无关

所有模型适配器都经过 Model Gateway。当前仓库包含 OpenAI-compatible Adapter 和可重复的 Mock Adapter。模型输出始终属于推断，不会仅因模型生成就自动成为 Evidence。

## 安全与权限

- 不采集私钥、助记词或密码输入框内容。
- 用户资金操作必须显式批准；V0.1 不执行真实资金操作。
- Trade Proposal 在进入适配器前必须获得精确绑定且有时限的 Approval Capability。
- Mobile 访问受 Application Allowlist 约束，敏感 UI 节点必须脱敏。
- 公开同步发现疑似 Secret、本机路径或禁止文件时一律 Fail Closed。

参见[安全与批准](docs/SAFETY_AND_APPROVAL.md)、[Mobile Security](docs/mobile/MOBILE_SECURITY.md)和[安全报告规则](SECURITY.md)。

## 开发与验证

```bash
npm install
npm test
npm run typecheck
npm run build
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
npm run desktop:build
```

依赖真实外部环境的验收必须与 Fixture 覆盖分开记录。

## Build in Public

GitHub 是 OrdinConn 唯一的公开工程事实来源。Issue 定义正式工作，DevLog 记录决策、失败、验证和剩余风险。X 只保留人工审核的草稿，Codex 不会自动发布。

- [开发日志](docs/devlog/)
- [Problems and Solutions](docs/PROBLEMS_AND_SOLUTIONS.md)
- [Codex Field Notes](docs/codex/CODEX_FIELD_NOTES.md)
- [公开开发规则](docs/open-source/BUILD_IN_PUBLIC.md)

## 文档

- [当前状态](docs/CURRENT_STATUS.zh-CN.md) · [English](docs/CURRENT_STATUS.md)
- [架构](docs/architecture/OVERVIEW.md)
- [Mobile Intelligence](docs/mobile/MOBILE_INTELLIGENCE.md)
- [Model Gateway](docs/MODEL_GATEWAY.md)
- [Connector 与 Source Registry](docs/SOURCE_REGISTRY.md)
- [架构决策](docs/decisions/)
- [安全与隐私](docs/open-source/SECURITY_AND_PRIVACY.md)

## 路线图

所有阶段必须通过真实 Gate 后才能前进：先在真实 Emulator 上完成 M1.5，再进入 M2 的验证式导航、M3 的生产级 App Skills、M4 的 Evidence 晋升，以及 M5 的物理 Android 设备。详见[公开路线图](docs/roadmap/README.md)。

## 贡献

项目采用 Issue-first 开发。重要工作开始前应创建或关联范围明确的 Issue，并保留 Evidence 与模型推断的边界，提交相关验证结果。参见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 已知限制

- 真实 Android M1.5 Gate 因缺少运行环境而被阻断。
- Mobile Navigation Actions 尚未实现。
- Computer Runtime 仍是受权限约束的部分实现。
- 当前金融执行适配器仅支持 Paper Execution。
- 默认并行 Rust 测试负载下，两项 1 秒 AVD 生命周期测试可能超时；同一 Desktop Library Suite 串行运行时通过。

## License

OrdinConn 使用 [MIT License](LICENSE)。
