# H1 MOBILE_BUSY 诊断

[English](H1_MOBILE_BUSY_DIAGNOSIS.md) | [简体中文](H1_MOBILE_BUSY_DIAGNOSIS.zh-CN.md)

Issue：[#14](https://github.com/Btkkgo/OrdinConn/issues/14)。基线：`ff33c7d5c4789d4e44ebed5917938905b989cabd`，`codex/realtime-workbench`。用户人工测试 H1 失败，没有进入 H2。本报告只记录脱敏元数据，不包含采集到的 UI 文本。

## 证据与根因

只读检查现有 SQLite Repository 和追加式 Audit，确认真实 Settings Observation 有效：设备 `emulator-5554`，Package `com.android.settings`，Activity `com.android.settings.homepage.SettingsHomepageActivity`，70 个元素，采集时间 **2026-10-02T12:44:14.705852Z**，来源 `android_ui_tree`，注册来源 `android-manual-com.android.settings`，具有非空 UI Tree hash 及前次 Observation 关系。尚未执行提取。对未变化页面重复观察，产生无变化 Diff 是正常结果。

| UTC 时间 | 已持久化证据 |
| --- | --- |
| 12:44:11.343986 | 请求 A 开始；11.344265 Audit 记录 running。 |
| 12:44:13.163000 | 另一请求 B 在 A 运行时开始。B 返回 `MOBILE_BUSY`，13.163002 完成，13.163874 写入 Audit；无 before/after Observation。 |
| 12:44:14.705852 | A 获取 70 元素 Observation；14.725903 写入创建 Audit。 |
| 12:44:14.731320 | Diff Audit 记录 `NO_MEANINGFUL_CHANGE`，0/0/0。 |
| 12:44:14.732313 | A 成功完成，before/after ID 均指向该 Observation，错误码和错误信息均为 null；14.732546 写入完成 Audit。 |

这是**两个不同的 ActionResult**，成功结果没有被覆盖。`MobileHomePage` 人工观察按钮和 `App.observe` 缺少执行中保护，允许请求重叠。Native `interact_mobile_device` 在工作前设置 `manual_mobile`，正确拒绝另一条重叠请求。Repository 按 `captured_at = started_at` 倒序排列动作，`MobileObservationContext` 显示 `actions[0]`；因此即使 A 后完成，开始更晚的 B 仍是最近尝试。失败记录真实存在，继续保留显示。

调用链：按钮 → `App.observe` → typed `runtimeClient.interactMobileDevice` → Tauri `interact_mobile_device` → `DesktopManualDriver` → 既有 `MobileHost`/ADB → 规范化 Observation → Repository/Audit → 最终 ActionResult → 释放执行锁 → workspace 响应。Observe 在 `run_manual_interaction` 内只采集一次。前端五秒刷新只调用 workspace 读取命令；采集分支没有 effect 或表单提交触发 Observe。Audit 证明请求重叠，**不能证明一次点击产生两次 invoke**；没有逐点击遥测可区分重复点击或另一调用方。没有证据支持锁永久卡住或成功动作被错误标记。

## 最小修复

`App` 中人工观察和导航共用 `ManualMobileFlight`：在第一次 await 前同步关闭入口，在响应应用完成后的 `finally` 释放。执行中禁用观察/检查设备及冲突导航按钮；Stop 保持可用。真实后端拒绝结果原样传递。后端并发保护、结果排序、Audit 和历史数据不变。Native command 仅为 fixture 测试委托到同一内部实现，未增加第二套 Runtime 或数据库。

## 验证与边界

修复前 UI 回归因 Observe 按钮仍可用而失败。可控 Promise 测试覆盖重复调用、完成、IPC 错误恢复及真实 `MOBILE_BUSY` 保留。Native 测试使用假 ADB、有界 fixture 屏障、内存凭据和临时 SQLite：首条 Observe 生成证据并完成；重叠 Observe 失败但不释放他人的锁、不伪造 after-state；Runtime 恢复 idle 后下一条 Observe 成功。fixture 不发出 input 命令。

人工结果仍为 **H1 FAIL / BLOCKED，等待用户复测**；修复后开发版只有在本地检查通过后才标记 **READY TO RETEST**。**OBSERVATION DATA = VALID**；原成功 ActionResult 有效，缺陷在前端动作入口并发控制。H2–H7 未运行。本轮 Codex 真实 Android 动作、真实 Provider 请求、签名验证：**0 / 0 / 0**。无 codesign、打包、发布或 X draft。保留原有八项未跟踪内容。参见[当日验证记录](../devlog/2026-10-02.zh-CN.md)。
