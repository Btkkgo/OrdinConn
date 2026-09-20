# 公开交互日志规则

[English](INTERACTION_LOG_POLICY.md) | [简体中文](INTERACTION_LOG_POLICY.zh-CN.md)

## 目的

有意义的工程工作应留下简明、可审计的公开摘要。摘要来源是已完成工作与验证证据，而不是原始私密对话。

## 必需字段

每条 `Public Interaction Summary` 包含：

- Timestamp（时间）
- Stage（阶段）
- GitHub Issue
- User Goal（用户目标）
- What Codex inspected（Codex 检查内容）
- What Codex changed（Codex 变更内容）
- Files changed（变更文件）
- Problems discovered（发现的问题）
- Solution（解决方案）
- Commands and tests executed（执行的命令和测试）
- Result（结果）
- Remaining risks（剩余风险）
- Next recommended step（建议下一步）
- Codex engineering note（Codex 工程笔记）

## 转换过程

`Private interaction -> Technical extraction -> Redaction -> Fact check -> Public summary -> DevLog`

有工程意义的任务完成后，将英文摘要追加到 `docs/devlog/YYYY-MM-DD.md`，同步中文摘要追加到 `docs/devlog/YYYY-MM-DD.zh-CN.md`。拼写检查、状态询问或其他无变更交互不需要记录，除非发现可复用问题。

## 禁止记录

- 完整 Prompt 或对话 Transcript
- Credential、Authorization Material、Cookie 或 Session
- 私人账户或客户信息
- 私人联系方式或精确个人地址
- 机器用户名或原始 Home-directory Path
- 未公开业务材料或受限第三方 Asset
- 未通过声明中 Acceptance Check 的结论

## 状态纪律

按[公开开发索引](README.zh-CN.md)中的定义使用 `Implemented`、`Designed`、`Verified` 与 `Blocked`。当环境或 Command 会实质影响验证结论时，必须记录它们。
