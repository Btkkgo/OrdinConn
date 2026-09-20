# English

## Goal

Establish GitHub as OrdinConn's public engineering source of truth with issue-first work, sanitized DevLogs, architecture decisions, a fail-closed security gate, and documentation-only scheduled synchronization.

## Current State

Closed as verified. The official public repository, documentation-only sync, security gates, and independent two-hour macOS scheduler are operational. X publication remains manual.

## Scope

- Public English and Chinese project entry documentation.
- Current status with Implemented, Verified, Partial, Blocked, Designed, Planned, and Not Started states.
- Issue templates, labels, DevLog, ADRs, security policy, contributing guide, and changelog.
- Working-tree and Git-history secret/path checks.
- Two-hour synchronization limited to public documentation, GitHub metadata, and manual X drafts.
- Manual X draft generation only; no X authentication or publication automation.

## Out of Scope

- Product runtime refactoring.
- Automatic X publishing.
- Uploading private prompts, conversations, credentials, local runtime data, build artifacts, Android SDKs, or emulator images.

## Acceptance Criteria

- Public repository exists and Issues are enabled.
- Initial public branch is `main` and contains the current publishable project.
- Security gate passes for the working tree, Git history, tracked paths, and commit metadata.
- Public documents and templates validate.
- Scheduled sync cannot stage product code.
- The primary two-hour scheduler runs through macOS `launchd` without Codex or ChatGPT and passes a real change/push plus no-change acceptance.
- First manual X draft links to verified GitHub records.
- The Issue receives a final comment with commands, validation, results, and remaining risks.

## Result

The official repository is public on `main`; the public history and working tree passed the security gates; the scheduler completed real change/push and no-change acceptance; and no product code or X publication entered the scheduled path.

## Validation

Validation covered sanitizer and history-gate tests, allowlisted sync integration, document/link checks, plist lint, `launchctl` operation, a real background push, a second no-change run, and public remote verification.

# 中文

## 目标

将 GitHub 建立为 OrdinConn 的公开工程事实来源，采用 Issue-first 工作、脱敏 DevLog、Architecture Decision、Fail-closed Security Gate 与仅文档的 Scheduled Sync。

## 当前状态

已作为 Verified 关闭。正式公开仓库、仅文档 Sync、Security Gate 与独立的两小时 macOS Scheduler 已运行。X 继续由用户人工发布。

## 范围

- 公开的英文与中文项目入口文档。
- Current Status 区分 Implemented、Verified、Partial、Blocked、Designed、Planned 与 Not Started。
- Issue Template、Label、DevLog、ADR、Security Policy、Contributing Guide 与 Changelog。
- Working-tree 与 Git-history Secret/Path Check。
- 两小时一次的 Sync，只允许 Public Documentation、GitHub Metadata 与 Manual X Draft。
- 只生成 Manual X Draft；不进行 X Authentication 或 Automatic Publication。

## 范围外

- Product Runtime Refactor。
- 自动发布 X。
- 上传 Private Prompt、Conversation、Credential、Local Runtime Data、Build Artifact、Android SDK 或 Emulator Image。

## 验收标准

- Public Repository 存在并启用 Issue。
- 初始 Public Branch 为 `main`，包含当前可公开项目。
- Security Gate 对 Working Tree、Git History、Tracked Path 与 Commit Metadata 通过。
- Public Document 与 Template 校验通过。
- Scheduled Sync 不能 Stage Product Code。
- 主要两小时 Scheduler 通过 macOS `launchd` 运行，不依赖 Codex/ChatGPT，并通过真实 Change/Push 与 No-change Acceptance。
- 第一份 Manual X Draft 链接已验证 GitHub Record。
- Issue 收到包含 Command、Validation、Result 与 Remaining Risk 的 Final Comment。

## 结果

正式仓库已在 `main` 上公开；Public History 与 Working Tree 通过 Security Gate；Scheduler 通过真实 Change/Push 与 No-change Acceptance；Scheduled Path 未包含 Product Code 或 X Publication。

## 验证

验证覆盖 Sanitizer/History-gate Test、Allowlisted Sync Integration、Document/Link Check、plist Lint、`launchctl` 运行、一次真实 Background Push、第二次 No-change Run 与 Public Remote Verification。
