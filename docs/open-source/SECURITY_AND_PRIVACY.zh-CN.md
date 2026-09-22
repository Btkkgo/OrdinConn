# 安全与隐私

[English](SECURITY_AND_PRIVACY.md) | [简体中文](SECURITY_AND_PRIVACY.zh-CN.md)

## 公共仓库规则

仓库及其开发日志均按公开内容处理。某个值存在于本机，不代表允许公开它。

## 禁止材料

- Provider、GitHub、X 或 Service Credential
- Password、Cookie、Session、Authorization Header、Private Key 或 Recovery Phrase
- 包含 Secret 的 Environment File
- 私人账户、客户、联系方式或地址数据
- 原始机器用户名或 Home-directory Path
- 原始私密对话
- 受限第三方 Asset 或 Dataset

## Fail-closed 发布

Push 或定时同步前，Automation 会扫描 Working Tree、可达 Git History、Tracked Path 与 Commit-email Metadata；验证 Diff；检查 Official Remote；并确保 Scheduled Commit 只包含 Allowlist 中的 Public-record Path。任何疑似 Credential-shaped Value 都会停止操作。

Scanner 只报告 File 与 Detector Category，不输出疑似值。macOS Home Path 脱敏为 `~/...`。

## 产品安全边界

OrdinConn 不采集 Private Key、Recovery Phrase 或 Password-field Content。真实资金 Action 需要显式批准，且 V0.1 未实现真实资金执行。Mobile Collection 受 Application Allowlist 约束，Sensitive UI Node 会被脱敏，`MobileObservation` 不能直接发布 Signal。

## 事件响应

如果怀疑 Git History 中存在 Secret，立即停止 Sync 和 Publication。先轮换或撤销 Credential，再按 Hosting Provider 的 History-remediation 流程处理。不能依靠之后的删除 Commit 把已暴露 Secret 变安全。首次 Push 前必须从 Public Branch 中移除私人 Commit Email 与原始机器 Home Prefix。

## 隐私修复记录 — 2026-09-20

一次经过明确授权的一次性 Privacy History Repair，只把 M1.5 Merge Commit 的 Author/Committer Identity Metadata 替换为当前 GitHub No-reply Identity。Tree、两个 Parent、Timestamp、Commit Message、File、Code、Test、Documentation 与 Issue State 均未变化。只有在 Remote 仍与预期旧 SHA 完全一致后，才使用精确 `--force-with-lease` 更新 `main`；普通 Force-push 继续禁止。

Branch 更新后，修复前 SHA 仍可通过 GitHub 直接访问，PR #5 也仍引用该 SHA。没有继续执行更多历史重写。该状态记录为 `GITHUB_CACHED_COMMIT_REMAINS`；是否需要 GitHub Support Cleanup 应另行评估。

## M2 隐私修复 — 2026-09-22

M2 Merge Commit `af843783` 的 Author 与 Committer 元数据不符合 noreply。一次明确授权的 `--force-with-lease` 将其替换为 `5a19c25`。Tree、按顺序排列的 Parents、Message 与时间戳完全相同；`git diff` 为空。由于本机未配置现成签名环境，替代提交为未签名，没有复制 GitHub 原签名而造成无效签名。公开可达的 `main` 身份预检现已符合 noreply。旧提交仍可从 GitHub 访问，PR #8 也仍引用它（`GITHUB_CACHED_COMMIT_REMAINS`）；托管平台缓存清理属于后续独立任务。本轮没有再次 Force Push 的授权。

仓库本地身份门、公开历史身份门与版本化 pre-push Hook 负责防止再次发生。在网页合并身份行为完成独立验证前，暂停 GitHub 自动生成 Merge Commit 的路径。维护者也应开启 **Keep my email addresses private** 和 **Block command line pushes that expose my email**，但账号设置不能代替仓库检查。
