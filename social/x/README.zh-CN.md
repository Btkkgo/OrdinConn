# OrdinConn 的 X 内容

[English](README.md) | [简体中文](README.zh-CN.md)

精选里程碑 Build in Public 材料分为两种状态：

- `drafts/`：等待用户人工审核与发布的脱敏内容。
- `published/`：不可变内容，以及 Publication Time、X URL、Source Commit 与 SHA-256 Content Hash。

Template 位于 `templates/`。普通 Code Change 应记录在 Daily DevLog，而不是 X Post。

只在出现用户可真实体验的重大能力或里程碑、完整的跨阶段故事、具有显著公共技术价值的重大突破，或用户明确要求时新增 Draft。普通 Bug Fix、Test Infrastructure、Refactor、Internal Architecture 与 Small Stage 保留在 GitHub 工程记录中。现有 X 历史保持不变。

## 发布规则

- 只陈述 Repository-backed 或刚刚验证的事实。
- 严格区分 `Implemented`、`Designed`、`Verified` 与 `Blocked`。
- 发布前运行 Public-log Sanitizer。
- Stage Thread 必须包含已验证 GitHub Project Reference。
- 不读取 Browser Cookie/Session，不索取 Password，不在 Git 中保存 Authentication。
- 发布由项目所有者人工完成。Repository Automation 不登录 X、不读取 Browser State、也不调用 X API。
- 每份 Draft 使用 `## English — Publication Version` 放置正式英文 Thread，使用 `## 中文 — 参考版本` 放置完整中文参考 Thread。两部分应基于同一脱敏事实自然表达，不能机械翻译私密对话。

## 人工流程

1. 检查并脱敏 `drafts/` 中的文件。
2. 确认已验证 GitHub Reference，并选择真实、可公开的 Screenshot。
3. 人工发布 Thread。
4. 增加 Publication Time、X URL、Source Commit 与 Content Hash。
5. 将完成文件移动到 `published/`。

本仓库不得包含任何 X Credential。
