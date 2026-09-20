# Mobile App Skills

[English](APP_SKILLS.md) | [简体中文](APP_SKILLS.zh-CN.md)

`MobileAppSkill` 是带版本的声明式 Contract，包含 Package Matcher、Known Screen、Recognizer、Navigation Rule、Extractor、Allowed/Blocked Action、Sensitive State、Verification Rule 与 Stop Condition。

M1 只提供 `GenericAndroidSkill`，用于识别已脱敏 UI Snapshot 并提取可见事实，不执行导航。后续阶段按以下顺序一次增加一个 Application：

1. Runtime 稳定后增加一个 Public-information Skill。
2. `TelegramPublicSkill`，只用于明确允许的 Public Channel。
3. `XPublicSkill`，只用于 Public Post 与 Search。
4. `BinancePublicSkill`，只用于 Public Market 与 Announcement Surface。

Skill 不能绕过 Application Allowlist、Source Registry、Evidence Gate、Action Policy 或 Research Budget。
