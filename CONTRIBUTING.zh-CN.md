# 为 OrdinConn 贡献

[English](CONTRIBUTING.md) | [简体中文](CONTRIBUTING.zh-CN.md)

## Issue-first 开发

在开展有意义的工程工作前，先创建或加入一个 GitHub Issue。Issue 定义目标、范围、风险和验收标准。小型拼写修正可以直接进行，但仍需有清晰的 Commit 和验证记录。

## Bug

使用 Bug 模板。提供可复现行为、环境详情、预期行为和最小化的安全诊断输出。不得包含凭据、私有应用内容、客户数据或完整本机路径。

## Feature 与研究

使用 Feature 或 Research 模板。严格区分当前实现与设计意图。Runtime 工作必须说明权限边界和所需的真实环境证据。

## Pull Request

1. 关联相关 Issue。
2. 变更保持在 Issue 范围内。
3. 添加或更新测试。
4. 对有工程意义的变更，同步更新公开文档和当天 DevLog。
5. 运行相关 Rust、TypeScript、Typecheck、Build 和 Packaging Gate。
6. 运行 `scripts/security/check-public-repo.sh` 和 `git diff --check`。
7. 如实报告失败和受阻检查。

工作尚未完整完成时使用 `Refs #N`；只有全部验收标准满足后才使用 `Fixes #N`。

## 安全合并 PR

维护者合并前必须使用仓库本地 GitHub noreply 身份，并运行 `scripts/security/check-git-identity.sh`。在 GitHub 网页合并的身份行为完成独立复核前，暂停使用 GitHub 自动生成 Merge Commit 的路径。CI、Review 和必要测试通过后，获取最新 `main`，执行受控本地合并，对拟合并提交运行 `scripts/security/check-public-history-identity.sh`，然后正常 Push `main`。发布后立即对公开 `main` 重跑历史身份扫描与 `scripts/security/check-public-repo.sh`；失败会阻断阶段收尾。架构兼容时用 `scripts/git/install-hooks.sh` 安装版本化 pre-push Hook。未配置现成且获 GitHub 认可的签名环境时，允许未签名的本地合并。

维护者应在 GitHub 账号设置中开启 **Keep my email addresses private** 和 **Block command line pushes that expose my email**。无论账号设置如何，仓库 Gate 都必须独立执行。不得打印检测到的私人邮箱。

## 安全

不要在公开 Issue 中提交包含敏感细节的漏洞报告。遵循 [SECURITY.zh-CN.md](SECURITY.zh-CN.md)。不得提交 Secret、Token、Cookie、Session、私钥、恢复短语、个人数据或受限第三方材料。

## 语言

代码和 Commit Message 使用英文。公开文档、Issue Body、重要 Issue Comment 和 Pull Request Body 使用英文在前、简体中文在后，并在同一次变更中同步两个语言版本。
