# Mobile M2 可验证导航实施计划

[English](2026-09-22-mobile-m2-verified-navigation.md) | [简体中文](2026-09-22-mobile-m2-verified-navigation.zh-CN.md)

本计划与英文版使用同一 Issue #7、五项任务及同一验收边界。英文版是精确命令与接口的主版；此处保留完整中文工程语义，出现状态冲突时以代码和真实测试为准。

## 目标与架构

仅实现六种受限、人工触发的 Android 模拟器导航动作：当前快照元素 Ref 的 Tap、定向有界 Swipe、非敏感安全 Type、Back、Home、允许列表内 Open App。`mobile-runtime` 管平台无关契约与策略；现有 Tauri `MobileHost` 管 ADB 和会话状态；`ordinconn-app` 管 SQLite 回执与审计；React 只能走类型化 IPC。

## 全局约束

- 只支持模拟器，每会话最多 20 次实际动作；不得提供任意坐标、ADB/Shell/Intent、真机、凭据/OTP、金融操作、Agent 自动动作循环或 M3。
- `GenericAndroidSkill::v1()` 保持仅观察。能力声明不等于自动执行授权。
- Type 最多 256 个 Unicode 字符，不允许控制字符、换行或 Shell 不安全字符；明文不得进入持久化、日志和回执。回执仅保留长度和 SHA-256。
- 所有尝试均产生脱敏回执；实际执行后必须重新走生产观察路径。`NO_CHANGE` 不等于 `VERIFIED`。
- 默认测试绝不操作模拟器；真实测试要求 `ORDINCONN_MOBILE_M2_SMOKE=1` 和专用 `OrdinConn_M1_5` AVD。
- 公共记录英文主版与中文副版同步。本轮不生成或发布 X；M3 保持未开始。

## 任务 1：领域契约与默认拒绝策略

文件：`crates/mobile-runtime/src/action.rs`、`lib.rs`。先写失败测试，覆盖活动会话、模拟器、预算、包名、前台、快照新鲜度、敏感屏幕、目标能力及六种动作参数；再实现 `MobileActionRequest`、目标、策略、决定、回执和验证。运行 `cargo test -p mobile-runtime action::tests -- --nocapture`、格式与 Clippy，通过后提交 `feat(mobile): add bounded M2 action policy Refs #7`。

## 任务 2：Android 执行与动作后观察

文件：`apps/desktop/src-tauri/src/mobile.rs`。先用假 ADB 写失败测试，确保过期 Ref、前台变化不会触发输入，执行失败或动作后采集失败不得标记已验证。再在单一会话互斥状态内实现预检、固定安全 ADB 动作和全新观察；真实 smoke 单独由环境变量门控。通过桌面专项测试与 Clippy 后提交 `feat(mobile): execute guarded emulator navigation Refs #7`。

## 任务 3：回执持久化与审计

文件：`crates/ordinconn-app/src/mobile_intelligence.rs`、现有迁移和测试。先测试执行与阻止两种回执、四类事件 `mobile.action_requested/blocked/executed/verified`，并扫描数据库/事件/工作区序列化无输入明文；再实现原子写入及最新回执投影。通过专项测试和 Clippy 后提交 `feat(mobile): persist sanitized action receipts Refs #7`。

## 任务 4：类型化 IPC 和人工控件

文件：Tauri commands/lib、`packages/contracts`、React client/Home/DeviceView、英中文案及测试。先测试类型契约、安全禁用状态和提交后清空 Type 输入；再加 `execute_mobile_action` 与最小动作控件。Rust 必须二次验证，React 不得运行 ADB。运行前端测试、typecheck、build 后提交 `feat(mobile): add manual verified-navigation controls Refs #7`。

## 任务 5：真实验收、回归与双语收尾

文件：新增双语 `docs/mobile/M2_ACCEPTANCE`，同步 Current Status、Roadmap、Mobile Intelligence、DevLog、相关问题/Field Notes 及 Issue #7。真实 AVD Settings 流程和负向门禁逐项记录，不得把 fixture 当作真机证据；重跑 M1.5 与 Issue #4 并行回归。执行 Rust fmt/Clippy/workspace 与 desktop 测试、TS 测试/typecheck/build、Tauri bundle、公开文档校验、安全扫描及 `git diff --check`。只有全部强制项通过才创建双语 PR、合并 main、关闭 Issue；否则保留分支和 OPEN Issue，准确记录阻塞。本轮不做 X 内容。
