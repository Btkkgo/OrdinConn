# 真实移动数据采集验收

[English](REAL_MOBILE_ACCEPTANCE.md) | [简体中文](REAL_MOBILE_ACCEPTANCE.zh-CN.md)

日期：2026-10-02（Asia/Shanghai）。Issue：[#14](https://github.com/Btkkgo/OrdinConn/issues/14)。开发基线 `5dae0cf86d023a64ddd318b448e63cc713afda57`，分支 `codex/realtime-workbench`，加下述人工目标匹配最小修复。本次有界实测 **REAL MOBILE DATA ACQUISITION ACCEPTANCE = PASS**。不是全人工验收，也不代表已证明零失败可靠性。

## 操作者与范围

H1–H2：**Human-operated**，由用户明确确认。H3–H7：**Codex-operated under explicit user authorization**。设备 `OrdinConn_M1_5`、Pixel 8 模拟器、Android 16、`emulator-5554`；只使用 Settings 和已解析的系统启动器。现有 UI 命令 Audit 中 `triggeredBy=owner` 表示授权主体，不区分实际操作者；本报告补充操作者归属，不改写历史 Audit。

未构建、签名或启动安装版 bundle，使用当前真实 Tauri 开发进程及 SQLite。Desktop 通过原生 Accessibility 操作和核验，不是 fixture 或替代浏览器。工具无屏幕录制权限，未采集 Desktop 截图。未打开账号、凭据、私人聊天或私人应用内容。

## Gate 证据

| Gate | 结果 | 证据 |
| --- | --- | --- |
| H1 | PASS — HUMAN | UTC 13:35:15 Observe completed；Settings 首页、70 元素；`mobile_observation_01a0fcd3-8437-75ab-af4e-0c5a542442cf`。 |
| H2 | PASS — HUMAN | 用户确认真实元素 ID/Text/Role/Class/Bounds/State 可检查。Codex 后续检查可点击父容器 `@e23`，LinearLayout，bounds `0,632 1080×206`，enabled/clickable，无敏感标记。 |
| H3 | PASS — CODEX | `manual_action_01a0fced-d268-7124-a6ef-967aa94c4149` completed。Before `mobile_observation_01a0fced-dfc5-770a-99ff-661015fa814f`，After `mobile_observation_01a0fcee-049c-7260-9d8d-c930b397c09e`；首页进入 SubSettings；Diff +62/-67；具有 running/receipt/action_diff/completed Audit。 |
| H4 | PASS — CODEX | Settings 应用列表 `spa.SpaActivity`：上滑 `manual_action_01a0fd08-6547-746b-bed5-55b9a2ddc4bb`，Diff +38/-33；下滑 `manual_action_01a0fd09-0417-7187-9d14-0b01a196c2b9`，Diff +33/-38，恢复原始 UI Tree hash。均 completed，前后 ID 与 Audit 完整。UI 方向表示手指方向。 |
| H5 | PASS — CODEX | 真实来源 `mobile_observation_01a0fcf0-c6e8-7424-bbf7-05751f1bec0f`，Settings 首页，UTC 14:07:13.118341 采集。本地 rule/UI-tree Extractor 生成 40 条提取记录、19 对象；提取执行于 UTC 14:20:20.564519。无模型请求。 |
| H6 | PASS — CODEX | 既有 SQLite 保存 Source → Observation → Element → Extraction → Data Object。UTC 14:21:30.768745 重复提取，新增 canonical 对象 0；保留 19 canonical 对象、40 条去重后的提取证据/sightings、两条 extraction-run/Audit。无孤立关联或重复 canonical key。 |
| H7 | PASS — CODEX | 原生 Desktop 展示真实 19 个列表对象；列表过滤恢复卡片，指标过滤显示真实 Empty State。Storage 来源详情显示设备/package/activity/time/Observation/Element/method/去重标识。展开原始证据可见 text/resource/bounds，UI Tree 置信度 100%、列表规则 65%。 |

示例对象 Storage（`data_01a0fcfc-cac3-77b7-b31f-4b5a75c17908`），注册来源 `android-manual-com.android.settings`，元素为 `el_872623eaf9a1387c680329647a9b2e5ca6f2e0640eea2e6196b1a5a272a3bdbe_1`，Resource `android:id/title`，bounds `210,954 182×71`。采集时间为原 Observation 时间，提取执行时间存于 `mobile_extraction_runs`。同一 Observation 重复 Extract 保留两条执行记录，不伪造第二次独立观察。数据 **LOCAL ONLY**。

## 失败证据与最小修复

第一次 Tap 在发送 input 前被 `TARGET_CHANGED` 拒绝。Settings 父容器没有 Resource ID/Content Description，复用的 M3 语义匹配器因此返回空，即使整棵 UI Tree 完全相同。假 ADB 回归在修复前复现该失败。

产品代码只修改 `apps/desktop/src-tauri/src/mobile_collection.rs`：人工路径保留原语义匹配，再仅对匿名元素、完整元素树/session/package/activity/尺寸/脱敏状态相同的情况，允许按精确 ref 绑定。树、文本、bounds、Activity 或脱敏状态变化时拒绝。M3 executor 匹配器、安全策略、Provider 和架构不变。新增两项回归覆盖匿名容器闭环及变化上下文拒绝。

随后一次 Tap 已发送，但自动后置采集失败（`ACTION_FAILED`，receipt `INTERRUPTED`，未伪造 after-state）；单独 Observe 确认确实到达目标页面。旧 host 丢弃了底层错误，精确原因仍为 **UNKNOWN**，不声称已修复。之后正常 Back 再完整 Tap 通过 H3。所有失败保留，后续应补后置采集诊断与可靠性验证。

顶部第一次下滑真实返回 `NO_CHANGE`，不计 H4 通过。首页上滑露出敏感设置标题后，值被脱敏、画面隐藏、后续导航被阻止；保护保持原样。改在安全应用列表完成 H4，页面准备使用一次明确授权的 ADB Settings Intent，单独计数。

## 动作计数与安全

Codex 本轮从 UTC 13:40 之后计数，不含此前人工 Observe。

| 动作 | 产品请求 | 实际改变状态的命令 |
| --- | --- | --- |
| Observe | 5 completed | 只观察 |
| Tap | 3：1 completed、2 failed | 实发 2；第一次拒绝未发送 |
| 上滑 | 2 completed | 2 |
| 下滑 | 2 completed | 2，其中一次无变化 |
| Back | 2 completed | 2 |
| Home | 1 completed | 1 |
| Open App | 1 completed | 1 |
| 安全 Settings 应用列表准备 | 1 次外部 ADB Settings Intent | 1 |
| Extract | 2 次本地执行 | 不发送设备 input |

共新增 34 次持久化 Observation，包括动作内部 before/after/稳定采样。16 条产品动作请求包含两条失败 Tap。被审批拒绝或审批超时的工具尝试未发送设备命令，不计动作。自动 unit/integration test 未控制真实模拟器；上述真实动作均为用户明确授权后逐项选择执行。

仅为 Home/Open Settings，通过既有 Desktop 手机设置临时加入实际解析出的启动器 `com.google.android.apps.nexuslauncher`。Home 和返回 Settings 均完成；**已恢复原 allowlist [com.android.settings]，并从 SQLite 读回确认**。最终 Observe 成功显示 Settings、70 元素。数据与原有八项未跟踪内容保留。

敏感保护：合成 fixture 回归及现场 Settings 标题脱敏/阻止通过。未提供或读取真实密码/PIN/OTP/秘密；Input 拒绝及 Repository 明文拒绝仅通过 fixture 验证，不冒充真实账号测试。Provider Requests **0**，Signing Verification **0**，Cloud 上传 **0**，X Draft **NONE**。

NOT RUN：Input（未进入输入场景）、Left/Right（无适合横向页面）、Stop（未选择持续操作）、七项 opt-in 真实设备/final 脚本、真实 Provider/Planner/Gemini 检查、签名/公证/打包/发布/部署、Desktop 截图。不把这些项目判 PASS。

## 回归

Rust workspace **341 PASS**，明确排除七项真实设备/final Gate；Desktop **77 PASS**，Contracts **9 PASS**；typecheck、前端/native Rust build、格式和 diff 检查 PASS。验收操作未改代码，复用相同代码的成功检查。公开前 public-log/secret/history/identity scan PASS。唯一建议下一阶段：动作后置采集的有界诊断与稳定性。
