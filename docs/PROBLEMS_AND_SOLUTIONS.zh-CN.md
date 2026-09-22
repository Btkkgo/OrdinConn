# 问题与解决方案

[English](PROBLEMS_AND_SOLUTIONS.md) | [简体中文](PROBLEMS_AND_SOLUTIONS.zh-CN.md)

## 问题 001 — 推断可能看起来像来源证据

### 背景

Agent 可以在没有事实来源的情况下生成很有说服力的解释。

### 症状

生成的解读可能错误地通过本应只接收外部事实依据的信息发布路径。

### 根因

Inference 与 Evidence 不能安全地作为可互换输入。

### 失败尝试

仓库中没有记录被放弃的实现；该风险从一开始就作为基础领域约束处理。

### 最终方案

Evidence Record 携带 Factual Level 与 Source Lineage。Signal Publication 至少需要关联一个不是 `MODEL_INFERENCE` 的 Evidence Item。

### 验证

Evidence 与 Signal Test Suite 会拒绝仅含推断的 Candidate，并保留冲突 Evidence。

### 可复用经验

生成式置信度不等于 Provenance。应通过 Type 与 Gate 编码这种差异，而不是依赖 Prompt 措辞。

## 问题 002 — Source Schema 会漂移

### 背景

公开 Feed 和交易所 Payload 可能在没有预告时改变结构。

### 症状

过于宽松的 Parser 可能接受不完整或被误解的数据，并将其送入 Strategy。

### 根因

网络请求成功不代表语义兼容。

### 失败尝试

历史中有专门的后续修复 `c71ce1a`，说明首次 Collection Pipeline 之后仍需更严格的 Schema Drift 边界。

### 最终方案

Collector 对 Shape 建立 Fingerprint，校验必需字段与 Timestamp，更新 Source Health，并在真实路径不使用 Mock Fallback 的前提下 Fail Closed。

### 验证

Collector Test 覆盖非法或缺失 Timestamp、Schema Drift 与 Normalization Error。

### 可复用经验

即使 Provider 没有发布版本，也要把 External Schema 当成带版本的 Contract。

## 问题 003 — Baseline 未就绪时 Strategy 可能运行

### 背景

Rolling Metric 需要足够的新鲜历史，而 Live Data 可能迟到或乱序。

### 症状

用默认值代替缺失历史，可能触发错误的高置信 Signal。

### 根因

Computation Availability 与 Decision Readiness 是不同状态。

### 失败尝试

没有记录被放弃的 Algorithm；Phase 2 Design 在扩大 Live Collection 前先显式定义了 Readiness。

### 最终方案

`WARMING_UP`、`MISSING_INPUT`、`STALE_INPUT`、`SCHEMA_ERROR` 与 `INSUFFICIENT_HISTORY` 都是可审计结果，不会产生普通 Candidate。

### 验证

Rolling-history 与 Derivatives Test 覆盖 Capacity、Ordering Tolerance、Stale Data、Missing Metric、Restored Bucket 与 Readiness Failure。

### 可复用经验

让“尚未就绪”成为一等结果，而不是 Exception 或填零 Input。

## 问题 004 — 本地无法证明 Android Integration

### 背景

M1.5 需要真实 Android SDK、ADB、Emulator、AVD 和 Online Device。

### 症状

显式 Smoke 首次报告 3 个前置条件失败和 7 个下游检查受阻。

### 根因

机器缺少所需 Android 环境。

### 失败尝试

第一次 Acceptance Run 有意 Fail Closed，没有伪造 Device Evidence。之后在授权下只安装必要的官方 Command-line Component、稳定 ARM64 Image 与一个专用 AVD。

### 最终方案

安装 OpenJDK 21 和官方 Android Command-line Tools，再以 Android 36 Google APIs ARM64 Image 创建 `OrdinConn_M1_5`。保留有序 Environment Discovery、有界 Boot Readiness，以及区分 `FAIL` 与 `BLOCKED` 的 Capture Report；真实 GUI IPC 另行验证。

### 验证

AVD 冷启动后在 `adb` 中在线，且 `sys.boot_completed=1`。Production Capture Gate 通过真实 Frame/UI Capture、Snapshot Parse、Ref、Observation、Persistence、Audit Event、Workspace Projection 与 Session Shutdown。打包 Desktop 另行通过真实 Typed IPC 与 Frontend Acceptance。独立的真实 Password-node Test 也通过。

### 可复用经验

保留第一次受阻结果，只能通过明确命名的真实环境重跑替代它。安装 Dependency 不等于验收；完整 Production Path 仍必须通过。

## 问题 005 — 外部 Mobile Command 可能挂起或发生分歧

### 背景

ADB 与 Emulator 是从多个可能 SDK Location 中选择的 External Process。

### 症状

无响应工具可能阻塞 Desktop Host；Diagnostics 与 Observation 可能选中不同 ADB；大 PNG 也可能在 Child Process 退出前填满 Pipe。

### 根因

初始 Process Execution 没有 Hard Deadline，Environment Selection 被重复实现。第一版有界实现也在读取 stdout 前等待 Child Exit，真实 Screencap 超过 Pipe Buffer 后形成 Deadlock。

### 失败尝试

首次 M1.5 Implementation 的 Independent Review 在发布前识别了这些缺口。

### 最终方案

Diagnostics 与 Observation 使用同一 Ordered Resolver；Process Work 放到 Blocking Pool；执行 Total Deadline；通过 Nonblocking Pipe 持续读取 stdout/stderr；每个受控 Command 放入独立 Process Group。Timeout 时只终止该 Group；Direct Child 退出后，不等待无关 Descendant 继承的 Pipe。

### 验证

Regression Test 覆盖独立 ADB Path、Configured-SDK Consistency、刻意挂起的 Fixture、成功或 Timeout 后仍持有继承 Output Pipe 的 Descendant，以及 256 KiB stdout Payload。之后约 190 KiB 的真实 PNG Frame 也通过相同 Helper。

### 可复用经验

Subprocess Timeout 必须约束 Process 本身，而不只是等待结果的 Caller。Producer 被阻塞前必须持续排空 Piped Output。

## 问题 006 — Android 16 改变真实 Window 与 UI-tree 行为

### 背景

Production Capture Path 最初使用受控 ADB Fixture 验证，当时真实 Android 16 Emulator 尚不可用。

### 症状

真实 Observation 无法从 `dumpsys window windows` 识别前台 Application，随后因两个 Platform Node 的下方 y 坐标小于上方坐标而让整个 UI Tree 失败。

### 根因

Android 16 在完整 `dumpsys window` 输出中提供 `mCurrentFocus`，但在更窄的 `windows` Section 中没有。UIAutomator 也可能输出 Lower Y 小于 Upper Y 的离屏 Platform Node。

### 失败尝试

保持 Fixture 兼容的原 Command 通过 Unit Test，却在真实 API 36 Image 上失败。重复同样 Capture 不会改变这两种 Output Shape。

### 最终方案

读取完整 Window Dump 以检测 Focus。丢弃非法的非敏感 Platform Node，同时保留 Snapshot 其余部分；若非法 Node 属于 Sensitive 或 Financial，则让整个 Parse Fail Closed，避免屏幕被误判为普通内容。

### 验证

Regression Test 在修复前确实失败，修复后通过，覆盖非法 Password Node 与 Financial-action Classifier。真实 Settings Snapshot 生成 70 个有效脱敏 Element，并排除 2 个 Bounds 反转的非敏感 Platform Node；完整 Gate 通过。

### 可复用经验

把 Operating-system Diagnostic 当成带版本的 External Schema。Identity 缺失时 Fail Closed；对于非敏感的可选非法 Node，只隔离该 Node，而不是丢弃其他有效 Observation。

## 问题 007 — Workspace Serialization 不能证明 Tauri IPC

### 背景

M1.5 要求真实 Rust Mobile Runtime → Tauri Command/Event → React Frontend 链路，包括 Status、Error、Start、Observation 与 Stop。

### 症状

第一版 Real-smoke Harness 直接调用 Persistence/Projection Helper，却把结果标记为 `TAURI_IPC`，没有真实 Frontend Invocation，也没有执行 Stop Command。

### 根因

一个有价值的 Lower-level Integration Test 被赋予了超出真实边界的 Acceptance Label，而且产品当时有 Observation，却没有显式 Frontend Stop Control。

### 最终方案

把 Automated Gate 重命名为 `WORKSPACE_PROJECTION`；新增并注册 `stop_mobile_session` Tauri Command 和本地化 React Control；由 Host 投影 Active/Disconnected Status；Real-desktop Check 继续作为独立 GUI Acceptance。

### 验证

打包应用首先显示预期的 Empty-allowlist Error。允许 `com.android.settings` 后，它显示真实 Emulator、Package、已验证 Observation、Frame 与 70 个 UI Element。Stop 让 UI 回到 `Disconnected`；SQLite 为同一 Session 记录 `mobile.session_started`、`mobile.snapshot`、`mobile.observation` 与 `mobile.session_ended`；ADB 仍报告 Emulator Online。

### 可复用经验

Command Helper 边界上的 Serialization 不能证明 UI 调用了 Tauri Command。准确命名 Lower-level Gate，并在真实 Desktop Application 中验证 GUI-only Acceptance。

## 问题 008 — 冷态并行启动耗尽成功路径测试预算

### 问题与观察结果

Issue #4 记录了四个 AVD/Subprocess 测试的间歇性失败。历史上出现过 Desktop 21/24 与 24/28，随后串行或 Workspace 重跑变绿。2026-09-22 的十次新鲜默认并行 Desktop 运行中，第一次再次由相同四个用例形成 24/28，随后九次通过。第一次冷态 Test Binary 内耗时 1.86 秒，热态约 0.5 秒。

### 为什么串行通过、并行失败

成功路径夹具会依次启动多个短生命周期 Shell Command，却把一秒时限当成进程启动性能契约。串行或热态通常能在预算内完成；冷态并行启动的调度开销使无关的成功路径检查触及时限。测试原本已使用独立的 `TempDir` SDK/AVD Root、脚本路径与 Process Group；审计未发现固定端口、全局环境变量修改或共享夹具路径。真正的竞争是测试专用墙钟预算与外部进程启动，而非共享 AVD 状态文件。

### 根因证据

新增回归测试用 Barrier 同时启动四个名称独立的 AVD 夹具，每个都有自己的 SDK、ADB Script、Emulator Script 与 AVD Home。Fake Tool 中受控的 200ms 延迟让原一秒成功预算稳定产生 `AvdBootTimeout`；仅把测试专用预算改为三秒后通过。这个 RED→GREEN 复现了时序机制，没有修改生产命令时限。

### 失败方法

此前绿色重跑和串行执行只是观察，不是修复。本轮没有采用全局串行、重试直到通过、忽略测试、增加生产 Timeout 或额外生产 Sleep。

### 正确修复

对成功的 Large-output 与 Inherited-pipe 夹具使用有界两秒测试预算，对成功的 AVD Lifecycle 夹具使用有界三秒测试预算。刻意验证超时的 30ms Command Test 与 50ms No-boot Test 保持不变。新并发回归覆盖独立 Root 与幂等逻辑关闭；生产代码和 Process-group Cleanup 未改。

### 回归覆盖

改动前四个既有用例各单独运行 20 次全部通过，相关测试在八线程和 32 线程热态下也各运行 20 次通过；这说明问题偏向冷启动，而不是证明问题不存在。改动后默认并行 Desktop 20/20 次通过（每次 29/29），完整 Workspace 10/10 次通过，八线程 Desktop 29/29 通过。真实 Android Smoke 首次因冷启动前台 Launcher 不在 Settings 白名单内而按设计拒绝；在专用 AVD 上打开 Settings 后，原样 Smoke 十项全部通过，包括 Frame、UI Tree、`MobileObservation` 与逻辑关闭。

### 可复用经验

成功路径测试应验证行为，而不是无意间测量冷态进程启动性能。短时限断言应留在专门的失败路径测试中；重复运行和受控延迟才能区分确定性覆盖与碰巧变绿。

## 问题 009 — Android 既有 Task 可遮蔽 OpenApp 的真实结果

### 背景与症状

M2 真实 Settings Search 流程将前台切到独立的系统 Settings Intelligence 包。Home 后对 Settings Launcher Component 执行 `am start`，可能恢复已有 Search Task。画面确有变化，通用变化检测最初把它判为 Verified，但目标包并未成为前台。

### 根因与失败尝试

动作后验证只检查 Frame/UI 是否变化，没有检查 OpenApp 的目标 Package。最初的固定启动命令也未清理既有 Task Stack。真实 OpenApp 的第一次断言失败，尽管夹具测试通过。

### 最终方案与验证

OpenApp 现在使用固定的 `--activity-clear-top` 标志，并要求 Post Observation 的前台包等于请求的允许包。专项单元测试拒绝“画面变了但包不对”；最终真实 M2 流程回到 `com.android.settings` 并通过。Settings Intelligence 只加入真实 Search 测试的白名单，没有成为生产默认值。

### 可复用经验

Verification 必须证明动作特定的目标状态，而不只是“有变化”。Android Task 复用只有真实环境才能暴露。

## 问题 010 — 被拒绝请求的标识可能泄入动作回执

调用方可以在过期 Session ID、Snapshot ID、Element Ref 或未授权 Package 字符串里放入任意文本。最初的 Receipt 构造函数即使拒绝动作也复制这些字符串。RED→GREEN 测试暴露了该持久化入口。现在 Receipt 的 Session/Snapshot ID 来自可信 Capture；未知 Ref 与未批准 Package Target 被脱敏，只有策略允许后才恢复已批准目标。回归测试确认调用方提供的哨兵字符串不出现在序列化 Receipt 中。

## 问题 011 — GitHub 自动合并身份未通过公开历史门

M2 PR 的 29 项技术与行为检查已通过，但 GitHub 自动生成的 Merge Commit 使用了不符合 noreply 的 Author 与 Committer 元数据。原提交带有 GitHub 已验证签名，因此修改身份会使原签名失效。一次授权的替代提交使用仓库本地 noreply 身份；因没有现成签名配置，移除了旧签名。Tree、按顺序排列的 Parents、Message 与时间戳完全一致，`git diff` 为空。新的公开 `main` 通过可达历史身份预检。旧对象仍可从 GitHub 访问，PR #8 仍引用它；没有再次改写。

长期修复是在合并前检查仓库本地身份、在 Push `main` 前后检查公开可达历史，并安装版本化 pre-push Hook。GitHub 自动生成的网页 Merge Commit 在身份行为完成独立验证前暂停使用。另一个独立 Worktree 中，原脱敏器误把 `.git` 指针文件当作公开内容扫描；排除该 Git 元数据文件并继续扫描普通文件后，完整仓库 Gate 恢复通过。这是修正扫描边界，不是绕过安全检查。
