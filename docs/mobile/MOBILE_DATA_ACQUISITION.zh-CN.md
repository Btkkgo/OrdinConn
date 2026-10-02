# Mobile Interaction 与数据采集

[English](MOBILE_DATA_ACQUISITION.md) | [简体中文](MOBILE_DATA_ACQUISITION.zh-CN.md)

Issue：[ #14](https://github.com/Btkkgo/OrdinConn/issues/14)。阶段：**REAL MOBILE DATA ACQUISITION ACCEPTANCE = PASS（H1–H2 人工；H3–H7 明确授权 Codex）**。M3 状态、Provider 配置、历史结果、retry/backoff/deadline 实现、Keychain 与冻结 App 均保持不变。

最新证据：[真实验收报告](REAL_MOBILE_ACCEPTANCE.zh-CN.md)。匿名人工目标仅在完整结构化树及上下文一致时精确绑定，M3 匹配不变。下文初始禁止 Codex 动作的交接边界，已由用户对 H3–H7 的明确授权更新。

## 数据链

`Source → Observation → Extraction → Data Object → Insight → Plan → Action → Result`

本阶段实现到 **Data Object**。Insight / Plan 保留为后续接口边界。人工交互另行记录 `Observe Before → Action → Observe After → 确定性 Diff`，不请求模型、不生成自主计划。

## 复用 Runtime

现有 Android 环境检测、ADB 子进程桥、UI XML parser、`MobileHost`、Snapshot 绑定导航、设备租约、动作回执策略与追加式 Audit 继续作为权威实现。没有新增第二套 Android Runtime 或数据库。采集投影包装既有 capture，不改变历史 M3 `MobileObservation` 合约，也不把历史 M3 记录自动迁移成采集业务对象。

生产动作仍限定在既有 Android Emulator / Settings / 预装 Settings 搜索 / 允许的 Home 导航范围。任意 App 与物理设备动作仍由现有 Host 阻断；观察也必须符合用户配置的允许列表。人工适配器在执行前重新观察，并用唯一语义身份绑定选中目标；不会自动重放动作。

## 模型与提取

- `mobile-runtime::collection::MobileObservation` 绑定设备、Package/Activity、采集时间、屏幕尺寸、规范化元素、UI Tree Hash、前序 Observation 与脱敏记录；ID 沿用底层 capture Observation ID。旧 M3 Observation 模型保持可用且不变。
- 稳定 `el_…` ID 来自 Resource/Class/Bounds 与重复出现序号；Snapshot 局部的 `@e…` 动作引用通过既有语义 resolver 重新绑定，不作为持久业务数据身份。
- 本地 Diff 识别 App/Activity、新增/移除、文本、选中、层级/状态及滚动内容变化；无有效变化有明确状态。动作最终 Diff 比较执行前和最终稳定 Observation，避免稳定性轮询遮盖动作造成的变化。
- Extractor 只读取结构化 Observation，输出可见文本、标题、列表项、按钮、链接类元素、数值、时间类文本、状态与选中项。标题/列表/时间规则判断记录较低置信度；没有使用 OCR/Vision，也不伪装成 UI Tree。
- 每个贡献元素形成通用 Data Object，保存规范内容、Values、来源、原始 Observation、Extraction ID 与 Element ID。分类为 text/list/metric/status/content/unknown，不虚构业务 Schema 或模型总结。

## 动作与稳定性边界

统一类型：`observe`、`tap`、`scroll_up`、`scroll_down`、`scroll_left`、`scroll_right`、`back`、`home`、`input_text`、`open_app`、`stop`。

每个人工动作保存 running/终态 ActionResult，包括时间、设备、owner 触发者、实际执行前后 Observation ID 和固定错误码；执行输入前，running 记录已持久化。继续复用固定 ADB 命令、Host 安全策略和动作回执；React 不提供原始坐标或 Shell 命令。Back/Home 保留逃离用途。输入沿用既有有界 ASCII 安全范围，并补充敏感目标、验证码/私钥形态值、破坏性/账号/发布目标及歧义选择阻断。

状态改变后，连续两次 Tree Hash、Package/Activity、元素数量一致才判定稳定。轮询间隔 120ms、循环预算 5 秒；底层 ADB 沿用既有有界子进程 timeout。已经开始的一次 capture 可能在循环预算后才返回，因此不是端到端严格 5 秒截止。Stop 在输入前与每次观察后检查，包含最终稳定观察；中断保留实际证据，不伪造 after-state。重启将未完成的人工动作标记失败，不重放。

## 本地 Repository 与来源

Migration `0012_mobile_data_acquisition.sql` 扩展现有应用 SQLite，增加 Observation、Action、Diff、Extraction、Object、Sightings 和 Extraction Run 表。Repository 提供 ID/Recent/Device/Package/Observation 查询；插入经过应用服务校验。外键及 immediate transaction 保持 Source/Observation/Extraction 关系。事件进入现有追加式 Audit，提交后才广播。

来源经既有 Source Registry 的人工本地策略注册：精确匹配用户允许列表、Android endpoint、不认证、禁用调度。采集数据不上传；本阶段不把对象晋升成已验证 Evidence、Insight、Signal 或 Trade。

去重组合设备、Package/Activity、规范内容、Resource/Class 和对象分类，排除短暂 Observation ID 与移动 Bounds。重复 Observation/Extraction 可保留，相同对象只进入一次数据流；Sightings 将重复采集关联到实际 Extraction。原对象来源与首次采集时间保持不变，详情显示观察次数。页面窗口为最近 100 条 Observation/Diff/Action 和 200 个对象；精确来源查询仍可读取窗口外的原始 Observation。

来源详情回答：数据是什么、来源设备与 App/Package/Activity、采集时间及方式、Observation/Element ID、原始 UI 文本/Resource/Bounds、提取置信度、去重标识与观察次数。结构化对象与本地 Observation/Diff 构成后续 Agent 上下文边界。

## 敏感保护

Password/PIN/OTP/验证码/私钥/助记词/支付凭据标签及既有敏感节点标记，会在采集持久化前移除文本、描述与 Resource 元数据；仅记录无明文的 `REDACTED_SENSITIVE_ELEMENT`。可编辑字段值不进入业务提取。Repository 拒绝未脱敏的敏感 Observation。敏感桌面 capture 隐藏截图；其余截图只驻留内存，也不作为提取输入。标签判断偏保守，不能普遍识别完全无标签的秘密；人工验收必须使用公开、非账号页面。

## 初始人工验收流程（历史）

代码与自动 fixture 完成后停在真实 Android 操作之前。用户可先确保内部 M3 验收 opt-in 关闭，再通过 `npm run desktop:dev` 启动开发应用。本轮未启动应用、初始化生产数据库、配置 Provider 或改变现有冻结 Bundle。

1. 连接既有专用模拟器，必要时配置现有 Settings 允许列表。
2. Observe 并查看 UI Tree，选择元素，检查 Text/Role/Bounds/Clickable/Scrollable/Editable/Resource。
3. 人工 Tap，查看执行前后结果与 Diff，再人工 Scroll。
4. 提取已观察页面，确认真实对象进入 Data Stream 并可分类过滤。
5. 打开对象，核对 Device/Package/Activity/Time/Method/Observation/Element 和原始 UI 证据。
6. 重复 Observe/Extract，确认观察次数增加但新增数据流对象不重复；Stop 并核对终态。

不得使用真实账号、密码/验证码、发送/发布、购买/支付、破坏性设置或 APK 安装；Codex 不执行真实 Android 验收动作。

## 验证边界

Fixture/unit/Repository/UI 测试和本地 Rust/frontend 构建，与人工 Android/Tauri 验收分开记录。支持的 `npm run desktop:build` 包装器会执行本轮禁止的 `codesign`，且现有 M3 Bundle 已冻结，因此 **NOT RUN / BLOCKED BY THIS TASK'S NO-SIGNING BOUNDARY**。没有绕过包装器直接做 Tauri 验收构建。签名、公证、真实 Provider/Planner 和 Gemini 检查均 **NOT RUN**。X Draft = **NONE**。
