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
