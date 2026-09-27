# M3 Phase 5 — Planner 与 Executor 集成

[English](M3_PHASE5.md) | [简体中文](M3_PHASE5.zh-CN.md)

日期：2026-09-27。[Issue #12](https://github.com/Btkkgo/OrdinConn/issues/12) 保持 **OPEN / status:needs-validation**。Phase 5 集成已实现；**Live Planner + Executor Gate: BLOCKED_MODEL_NOT_CONFIGURED**。**Full Autonomous M3: NOT_COMPLETE**。

## 配置与生产路径

桌面应用使用自身应用数据目录中的 `ordinconn-v0-1.sqlite3`。本轮只读打开生产数据库，确认 **Production Provider Count = 0**。审计未执行生产迁移、添加 Provider、读取凭据或请求真实模型。

复用 **Settings → Models**，配置一个启用的 OpenAI-compatible chat Provider：endpoint、默认 model identifier，以及服务要求时的 API key。现有保存 Provider IPC 持久化配置和凭据引用，现有系统凭据库将密钥存入 OS Keychain。密钥不得进入 Goal 文本、SQLite 业务记录、日志、截图或公开记录。模型选择确定且唯一；缺失、不兼容或有歧义的配置在 Observe 或模型派发之前失败。

生产路径为 **Workbench Goal → typed Tauri IPC → Rust 有界 runner → fresh Observe → Model Gateway / 配置的 Provider → 严格 Planner DTO → canonical 持久化 Step → 已验收 Executor → Receipt / Observe After / Verify → 下一次 Planner 决策或独立完成验证**。React 只提交身份和 Owner 意图，不能向 Executor 提交 ADB、坐标、任意动作对象或凭据。测试适配器仅用于测试；localhost 协议夹具通过实际 Gateway adapter 证明集成，但属于 **TEST_ONLY**，不能作为真实模型验收。

## 边界与修改

- 保留现有动作类型。模型输出仍为严格 typed JSON，复用有界格式错误重试，不支持任意工具或 shell 执行。
- 每次外发模型请求，包括失败和格式错误响应，均预留持久化 Goal 模型预算。Step 数、执行次数、重试、重复观察、连续失败及一个统一运行期限都使用 canonical Goal 状态；等待 Approval 不重置期限。
- Planner 使用最新 Observation 提交；指定历史 Observation 会被拒绝。未 claim 的过期 Pending Step 可跳过并在预算内重新观察和规划；已 claim 或已发出的动作不重放。
- Native UI 读取每次使用独立临时 XML 路径，检查该路径的实际 dump 成功标记，只清理自己生成的文件。0 退出但失败的 dump 不能作为新 Observation，也不能读取共享或旧 XML；派发前检查使用同一保护。
- Stop 立即取消等待中的模型请求，禁止后续决策与副作用，等待已发出动作的 Receipt 和验证收敛，再停止未结束 Goal 和设备会话。批准动作收敛后也优先保存 STOPPED，不允许完成或继续。错误和 panic 路径释放设备 lease。
- Approval 复用现有 Approval Engine 的会话绑定、过期、单次 capability，绑定 Goal、Plan revision、Step、Observation 和 canonical action intent。持久化 `APPROVAL_REQUIRED` 风险保持不变；授权与执行 claim 原子消费。模型不能批准自身。
- Completion 必须有 Owner 明确提供的不可变条件、归属正确的 Verified Step、动作 Receipt、Result 和最新 post-observation。Owner target 支持精确 Settings activity 或指定 Android resource 的安全输入值。模型完成提议本身不能完成 Goal。人工批准的最后一步使用同一完成验证，不额外请求模型。缺少绑定完成条件的自然语言 Goal 在完成边界 fail closed。
- 静态 Android component/resource 标识符与用户文本分开验证，password、wallet、private-key 和 messaging 标识符仍被拒绝。输入保留既有安全 ASCII 策略，Owner 输入完成 target 在持久化或进入模型 context 前拒绝 Secret-like token 和 seed-like 词组；未放宽通用 Secret 检查或外部 App 白名单。
- 持久化 Action Intent 后重启，仍中断未确认执行，不自动重放设备副作用。

唯一产品基线仅扩展 Android Settings 及其预装系统搜索组件上的有界 M3 执行。未增加 M4、新动作类别、复杂 UI、商业账号、支付、外部 App 或通用自动化。现有 Workbench 和 Phase 2–4 修改已保留。

## 验证

| Gate | 当前结果 |
| --- | --- |
| 默认并行 Rust workspace | PASS：287；最终测试准备修改后 Desktop 54/54 PASS |
| Desktop TypeScript / contracts | PASS：64 / 9 |
| Typecheck / frontend build | PASS |
| Rust build / macOS app packaging | PASS：工作区 build 和最新 release app bundle |
| 隔离并发 AVD 夹具 | PASS：最终重复调用 20/20 |
| 相关 AVD lifecycle 测试 | PASS：3/3 |
| 真实 BACK / SCROLL_DOWN / EXTRACT | PASS：Before/Action/After/Verify、Completion、重复执行拒绝、Evidence/Projection；Planner TEST_ONLY |
| 真实安全 INPUT_TEXT 表面 | PASS：修复后连续 3/3；最终测试文本调整后 2/2；精确值和键盘设置恢复均验证 |
| 生产 Provider 发现 | PASS：数量 0，只读 `MODEL_NOT_CONFIGURED`，无凭据读取 |
| 真实模型 + Planner + Executor，至少两次决策 | **BLOCKED_MODEL_NOT_CONFIGURED** |
| Full Autonomous M3 | **NOT_COMPLETE** |
| 安全 sanitizer / 公开文档链接 / diff | PASS |

回归覆盖 Gateway 的两次不同决策、精确完成真值、最新 Observation 绑定、未 claim 过期步骤重规划、模型/动作/时间预算、零 Provider 提前退出、等待模型及动作已发出时的 Stop、Approval 过期/重启/绑定/单次消费、重复执行与 Intent 持久化后重启。除明确标注真实 AVD Gate 的项目外，这些均属于夹具回归。

原来失败的并发 AVD 测试现在分别拥有独立临时 SDK、AVD home、host、serial 命名空间、延迟标记及命令 trace。合成冷启动延迟每个夹具只发生一次，测试专用期限由 3 秒改为 5 秒。生产超时与 freshness 规则保持不变，未增加全局测试串行化。

历史失败保留：初始默认套件在旧生命周期延迟夹具上失败；M3 之前的数据库夹具错误加载缺少前置依赖的后续迁移；精确输入准备暴露静态 metadata 被拒绝及下划线测试值超出既有输入策略。这些分别在夹具或校验路径解决。一次真实输入尝试在并发 release 编译负载下因 `StaleSnapshot` 被拒绝，输入命令未发送，未计为 PASS 或重放。后续真实结果仅取代该次运行的最终状态，不删除其历史安全证据。

后续输入尝试暴露点击后的 Observe 中断，以及预装键盘首次使用的 stylus 教程截获输入。尽管教程收到无敏感测试值，Settings 字段仍为空，精确验证正确失败。准备测试表面时对已观察到的教程执行一次 BACK，未重放失败 Goal 或修改 App 白名单。一次 workspace doc-test 因另一 Cargo 调用替换依赖产物而失败；最终 workspace 验证顺序运行，并保留默认测试并行度。这些失败尝试不计为成功。

审计还发现 [Android native dump 命令](https://android.googlesource.com/platform/frameworks/testing/+/refs/heads/main/uiautomator/cmds/uiautomator/src/com/android/commands/uiautomator/DumpCommand.java) 可在 idle/root 失败且未生成文件时正常退出。固定文件路径可能复用旧 dump 或其他 Host 的结果。独立文件归属和成功标记检查现已 fail closed；回归模拟这种 0 退出失败，证明不会读取旧 XML。未增加命令 timeout 或放宽 freshness。

批准最后一步执行期间的 Stop 回归先以 Completed 而非 Stopped 复现失败（RED），修复后通过（GREEN），并进入最终 287 项默认测试。已完成动作的 Receipt 保留，但 Goal 必须 STOPPED。

最后真实 Executor 回归在动作发出前因测试准备使用 Android 16 不支持的 `--activity-new-task` 而失败。移除该测试专用参数后沿用已验证的 clear-top 启动，修正后真实 Gate 1/1 PASS（78.04 秒），BACK/SCROLL_DOWN/EXTRACT 及重复执行拒绝均通过。此 Gate 的通用页面无输入表面；独立 Search Gate 的 3/3 精确输入结果单独记录。保留失败记录；生产执行代码、超时和打包产物不受影响。

首次正常推送因当前分支缺少已安装 pre-push hook 引用的身份脚本而失败。五个现有 identity/history/hook/installer/test 文件从同一仓库 `5056389` 安全提交逐字节恢复。已安装 hook 未改，且与 canonical 版本一致。身份夹具测试、本地 noreply 身份和 HEAD 可达历史均 PASS。未绕过安全门、改写历史或强推。

## 真实输入表面与剩余验收

专用 Android emulator 的 Settings 提供搜索入口。预装系统搜索 activity 为 `com.google.android.settings.intelligence.modules.search.SearchActivity`，已聚焦非密码输入 resource 为 `com.google.android.settings.intelligence:id/open_search_view_edit_text`。显式启用的 Gate 通过系统 intent 准备已观察到的预装搜索 activity，Observe 并解析输入字段，再通过真实 typed Executor 输入无敏感随机值 `M3 <nonce>`，检查精确 Receipt attestation，并独立 Observe 实际字段值，随后返回。测试使用空格以保留已验收输入策略，不安装辅助 App，不操作个人或账号字段。

显式 Gate 在准备阶段临时关闭专用 AVD 的 `stylus_handwriting_enabled`，保留原值（包括未设置默认值），在 PASS 前恢复并检查。[Android 定义了此系统开关](https://android.googlesource.com/platform/frameworks/base/+/refs/tags/android-16.0.0_r2/core/java/android/provider/Settings.java)。这仅影响测试准备。Gate 仅对独立 Observe 失败重试最多三次，不重发动作、不重试值不匹配。键盘 overlay 背后的 EditText 显示 focused，不能证明输入会到达目标字段；精确 post-value 比较仍为必需。

Owner 配置现有模型设置后，基于 emulator 当前真实 UI 和明确 Owner completion target，执行至少包含 **两次有意义 Planner 决策** 的安全真实 Goal。记录 Provider type 和 model identifier，排除凭据，证明真实 Observe → 真实模型规划 → typed action → 真实 Executor → post-observation → 独立验证 → 继续/完成。在此之前不能声称真实模型成功或 M3 完成。即使 Gate PASS，Issue #12 也等待 Owner Acceptance 后才关闭。本轮未生成 M3 milestone X draft。

修改前创建了私有起始快照、binary diff 和文件归档。所有初始 dirty 路径仍存在，初始五个 JPEG 哈希均未变化。私有 trace 与 capture 不公开提交。
