# Build in Public

[English](BUILD_IN_PUBLIC.md) | [简体中文](BUILD_IN_PUBLIC.zh-CN.md)

OrdinConn 公开工程进展，让架构、失败模式与 AI 辅助开发实践可检查。

## 公开内容

- 当前实现状态与 Phase Boundary
- Architecture 与 Security Decision
- 问题、根因和已验证方案
- 带环境限制的 Test 与 Build 结果
- 已脱敏的每日工程摘要
- 精选里程碑 X Thread 与可复用工程经验

## 不公开内容

- 原始对话或 Prompt
- 用户、客户或私人账户数据
- Credential、Token、Cookie、Session 或私钥
- 受限第三方资源
- 未验证声明、虚构 Benchmark 或被包装成真实结果的 Simulation

## 节奏

有意义的工程工作遵循 Issue → 开发 → 测试 → 双语 DevLog → Issue 更新 → 安全/Diff 门禁 → Commit → Push。主要本地文档 Scheduler 是 macOS LaunchAgent `com.ordinconn.github-sync`；它在登录时及每 7,200 秒运行，不要求 Codex 或 ChatGPT 保持打开。Codex Heartbeat 可暂停保留为恢复后备，但不是主要 Scheduler。Stage Close 会更新 Current Status 和相关公开工程记录，但不会自动生成 X Draft。

LaunchAgent 从 `~/Library/Application Support/OrdinConn/automation/` 启动一个专用、Ad-hoc Signed 的 `OrdinConn GitHub Sync` Launcher。该 Launcher 是读取 `Documents` 下仓库的窄范围 macOS Privacy Identity，不会向通用 Shell 授予 Files and Folders 或 Full Disk Access。Launcher 调用 `github-sync-runner.sh`，设置明确 Tool Path，执行仓库内 Fail-closed Sync Script，并将结构化记录写入 `~/Library/Logs/OrdinConn/github-sync.log`。日志到 5 MiB 时轮换，且不记录 Credential。

第一次安装时，macOS 可能询问是否允许 `OrdinConn GitHub Sync` 访问 Documents Folder。只允许该特定应用。之后若要撤销权限，在 **System Settings → Privacy & Security → Files & Folders** 中找到 **OrdinConn GitHub Sync** 并关闭 Documents Folder；若系统将其显示在 **Full Disk Access**，也只移除或关闭这个同名应用。随后运行 `scripts/github/uninstall-sync-launchagent.sh` 卸载任务并移除本地 Runner 与 Launcher。

Product-code Delivery 与文档同步分开。它必须通过相关 Test/Build Gate，并使用 Product Commit，不能借用两小时文档同步 Commit。

有意义的工作采用 Issue-first。Issue 定义范围和验收、接收执行结果，并在验证受阻时保持 Open。完整 Acceptance Criteria 满足前，Commit 使用 `Refs #N`；完全满足后才可使用 `Fixes #N`。

## 语言规范

GitHub 公开记录以英文为主要/默认版本，简体中文作为同步维护的辅助版本。核心文档使用成对的 `.md` 与 `.zh-CN.md` 文件，并提供双向语言链接。Issue Title 与 Commit Message 使用英文；Issue Body、重要 Issue Comment 与 Pull Request Body 使用英文在前、中文在后。Codex 与用户默认使用中文沟通。

DevLog 使用 `YYYY-MM-DD.md` 与 `YYYY-MM-DD.zh-CN.md`。X Draft 包含 `## English — Publication Version` 与 `## 中文 — 参考版本`，两种语言都要提供完整 Thread，由用户人工审核，且绝不自动发布。两个语言版本必须报告相同 Stage、Test、Result、Risk 与 Next Step。

仅在出现用户可真实体验的重大能力或产品里程碑、多个工程阶段形成完整故事、具有显著公共技术价值的重大突破，或用户明确要求时，才新增 X Draft。普通 Bug Fix、Test Infrastructure、Refactor、Internal Architecture 与 Small Stage 只记录在 GitHub。保留既有 X 历史。GitHub 是完整工程历史；X 是精选里程碑渠道。

## 发布状态

- `draft`：已检查、等待用户人工发布的公开材料。
- `published`：带有 Timestamp、不可变 Content Hash、Source Commit 与 URL 的材料。
- `blocked`：未通过 Sanitization、Verification、Authentication 或 Remote Check 的材料。
