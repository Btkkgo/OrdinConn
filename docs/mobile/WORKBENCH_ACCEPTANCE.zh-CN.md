# 实时工作台验收 — Issue #11

[English](WORKBENCH_ACCEPTANCE.md) | [简体中文](WORKBENCH_ACCEPTANCE.zh-CN.md)

> 当前 Phase 5：有界 Planner/Executor 集成已实现；真实模型验收 **BLOCKED_MODEL_NOT_CONFIGURED**（Production Provider Count **0**），Full Autonomous M3 **NOT_COMPLETE**。下文较早的 Phase 2–4 验证记录保留为历史证据；最新验证见 [Phase 5 收口记录](M3_PHASE5.zh-CN.md)。


> 本文为较早的工作台验收记录。[M3 Phase 4](M3_EXECUTOR.zh-CN.md) 现已单独验证显式单步设备执行，完整自主 M3 仍为 NOT_COMPLETE；操作体验验收仍待所有者完成。

## 历史范围与结果

主页以所有者提供的 Reference A 为视觉目标、Reference B 为结构基线。仓库、设置、数据详情与讨论、M2 人工 Inspector 保留。本地修改基于 `cc1d0c9`，未提交或推送。

**整体：完整自定义目标执行为 BLOCKED；已实现的界面与人工指令集成可独立审查。** 现有研究任务保存 pending 目标，不是可自主执行的计划。工作台明确显示手机自主规划执行 `NOT_IMPLEMENTED`。M3 保持 NOT STARTED；Issue #10 所有者操作体验仍为 PENDING USER ACCEPTANCE。

## 真实数据与 Runtime

- 设备连接与名称来自新鲜 ADB 诊断，历史会话不能证明已连接。Observe 通过类型化 IPC 更新真实 Frame、语义 Snapshot、结构化观察、持久化和 Audit。
- 统计按本地日历的今日/昨日聚合已加载对象，并去重；窗口最多 200 条观察和 200 条 Evidence，不代表全库总量。分类依显式数据类型；手机观察与未指定类别的 Evidence 属于“其他”。缺少对比基线时不编造增长率。
- 提取复用已有脱敏观察解析和持久化；继续采集先 Observe，再调用现有 Snapshot-bound 向上手指 Swipe，要求动作后的 Receipt 为 executed/VERIFIED。敏感页面拒绝操作，不制造采集成功。
- Stop 中断后续 Runner 步骤，仅关闭逻辑会话，不杀 AVD 或移除观察。数据库失败明确显示失败。回归测试复现写锁竞争，修复为读取前取得 `BEGIN IMMEDIATE`，在现有 Busy Timeout 内等待写锁，保留会话与 Audit 原子持久化。
- 研究目标持久化，任务创建事件仅含脱敏元数据。只投影手机研究任务，排除迁移默认字段为空的普通任务。七种计划状态按真实状态显示或隐藏。“忽略”是本地视图偏好，不是持久化取消。
- 未增加前端 ADB 桥、新模型适配、自主循环，未重写 Approval Token 或扩大生产动作策略。敏感与金融目标仍拒绝。消息发送、发布、删除、账号操作和真实资金不在人工动作面内。
- Fixture 同时要求 development 与显式 `workbenchFixture=1`，显示明确标记，指令控件不能写入 Fixture 观察或任务。生产 JS 排除 Fixture 入口与演示数据。

## 视觉审计

修正循环覆盖布局、卡片密度、蓝色选中态、字体层级、手机尺寸、指令行、输入框、右栏留白和页脚。完整原生 Tauri Fixture 截图与真实设备截图分别留存；Stage Manager 缩略图不作为验收证据。

| 检查项 | 审查 |
| --- | --- |
| 标题/副标题层级、右上 Android 状态 | 已检查 |
| 三栏比例、顶部/底部对齐、中栏核心 | 已检查 |
| Panel/Card 边框、圆角、卡片间距 | 已检查 |
| 数值靠右、蓝色选中边框、当前分类文案 | 已检查 |
| 手机居中、真实画面保持比例、合理留白 | 已检查 |
| 右上 Agent 状态、手机下方 2×2 纯文字按钮 | 已检查 |
| 输入框 + 执行、右栏留白 | 已检查 |
| Observe / Capture / Plan / Approval 顺序 | 已检查 |
| 无新增图标、彩色分类、图表、Logo、旧 Dashboard | 已检查 |

保留的差异：窄版主页/仓库/设置导航和 macOS 原生标题栏；真实 Pixel 8 内容在参考比例外框内保持更窄的真实宽高比；设备工具折叠入口保留人工 Inspector；真实统计、手机内容、pending 目标和错误不同于演示数据。明确披露这些差异，不用 Fixture 填充生产界面。

## 验证与保留的失败

实际桌面包检查完成后，在下方记录最终命令结果与截图尺寸。测试不等于所有者 UX 验收。

早期尝试保留：软件 Emulator 崩溃/EMULATOR_READY 失败、打包期间一次真实 M2 动作被中断、后续 M2 搜索页面转换未取得动作后 Frame。真实动作 Gate 有单独的成功重试。一次新鲜默认并行 Rust 运行也在 Release 编译期间复现已有 AVD Lifecycle 并发夹具超时。未放宽生产时限或无关夹具。

## 交付边界

Commit NONE；Push NONE；未生成或发布 X 内容。本地 Issue #11 修改保留审查，不声明自主引擎完整验收或所有者 UX 通过。

## 最终验证 — 2026-09-27

| Gate | 结果 |
| --- | --- |
| TypeScript 测试 | PASS：55 Desktop + 5 Contracts |
| TypeScript Typecheck | PASS |
| Rust Workspace 串行 | PASS：165；显式 Gate 真实测试另列 |
| Rust Workspace 默认并行 | 早期 PASS；之后两次在已有 AVD 并发夹具 FAIL，未隐去 |
| Stop 写锁竞争回归 | 修复前 FAIL、修复后 PASS；持久化关闭幂等性也 PASS |
| 显式真实 Observe Gate | 最终 Rust 代码 PASS |
| 显式真实 M2 导航 Gate | 公开 Settings 重置后，最终 Rust 代码 PASS；保留早期失败 |
| 打包 Tauri 真实 Observe | PASS：Frame + 结构化观察 + Audit |
| 打包 Tauri 真实 Collect | PASS：真实 Swipe、VERIFIED Receipt、已完成结果关联一个对象 |
| 打包 Tauri 真实 Stop | PASS：会话 ended、关闭事件恰好一条、AVD 仍已启动 |
| Enter 自定义目标 | pending 持久化/事件 PASS；自主执行 NOT_IMPLEMENTED |
| 生产排除 Fixture | PASS |
| 公共安全/文档/Diff Gate | 最终文档收尾前 PASS；最后检查记录于 DevLog |

后续仅改前端手机 Surface/CSS，Rust 逻辑和依赖相同，复用最终 Rust 验证。不声明默认并行完全确定。人工采集虽真实可用，完整自定义目标自主循环仍为 BLOCKED。

## 原生截图收尾

Reference A 人工视觉审查 PASS；Reference B 结构审查 PASS。完成六轮全尺寸原生截图审查/修正，排除被拒绝的缩略图。这是人工参考图比对，不是自动 Pixel Diff 分数。最终生产截图为 `artifacts/ordinconn-workbench-live-final.jpg`，1448×1086。默认桌面窗口为 1448×1086；短窗口保留页脚、允许 Panel 内滚动。真实画面在参考比例手机外框内保持比例并留黑边，Inspector 绑定同一拟合画面 Surface。手机尺寸仍比 Reference A 小约 9%。原生标题栏、保留的窄导航、设备工具折叠入口、真实内容/状态差异已披露。

Frontend/Rust/Tauri Desktop Build PASS；最终原生包启动/Observe PASS。生产排除 Fixture、公共安全门、公共文档验证、Rustfmt、Diff Check PASS。公共安全首轮因私有诊断日志含本机路径而 BLOCKED；日志和临时预览包移至公共仓库外保留，未修改扫描规则。

### 保留图像来源

以下 JPEG 是保留的本地既有资料，不包含在 Phase 5 提交中；本轮私有真实设备截图不发布。

- `artifacts/ordinconn-workbench-fixture-02.jpg`: 1448×1086; SHA-256 `1d6c15572889b5effaf73f0455931dafcbe9e00835078a52b8c49ba42fa8a497`.
- `artifacts/ordinconn-workbench-fixture-final.jpg`: 1444×1085; SHA-256 `924ad0a7de79267da0891737f8e05f6976a0686b5a038ccf84d5358e473358de`.
- `artifacts/ordinconn-workbench-live-final.jpg`: 1448×1086; SHA-256 `a2967e891821c60edf386c57147b6074c7429a21ed1db92a09300762111846af`.
- `artifacts/ordinconn-workbench-live-iteration-04.jpg`: 1440×920; SHA-256 `ac010b38ed80e5aefc4f335368d3b1b271218a19344e7567ec6e08f47ba5d316`.
