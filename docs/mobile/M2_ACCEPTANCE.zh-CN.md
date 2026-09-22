# Mobile Intelligence M2 — Verified Navigation 验收

[English](M2_ACCEPTANCE.md) | [简体中文](M2_ACCEPTANCE.zh-CN.md)

日期：2026-09-22。Issue：[ #7 ](https://github.com/Btkkgo/OrdinConn/issues/7)。范围：专用 `OrdinConn_M1_5` Android 16 模拟器、Android Settings，以及一次性密码输入测试 App。M3、物理设备、自主导航、账号登录、资金操作与 X 发布均不属于本 Gate。

## 契约与策略

公开 Action 只有 `tap(elementRef)`、`swipe(direction)`、`type(elementRef, text)`、`back`、`home` 和 `open_app(packageName)`；没有原始坐标、Shell、ADB 参数或自由形式 Intent 字段。Rust 策略要求活跃 Emulator Session、用户控制的 Package Allowlist、预期与实时前台 Package/Activity 一致、快照不超过十秒、目标 Ref 属于该快照，以及每会话最多 20 次已派发 Action 尝试。M2 生产 Host 进一步将非逃离导航限制在 Android Settings 与 Settings Intelligence；用户自行添加包名不能放行任意 App。动态解析的 Launcher 只用于 Home 验证及返回 Settings。敏感或金融屏幕只允许 `back`/`home` 安全退出。Tap 坐标由已验证 Element Bounds 计算；Swipe 坐标由合理屏幕尺寸约束。Type 要求已聚焦、启用的 EditText，目前只接受 1–256 个 ASCII 字母、数字或空格；其他文本直接拒绝。

Host 串行化 observe/action/stop；非逃离动作在固定 ADB 命令前重查设备、前台并重新读取实时 UI Tree。同一 Activity 内元素或 Bounds 改变时阻断旧坐标。动作后重新走生产 Observation；Tap、Swipe、Type、Back 要求前台或 UI Tree 变化，不能凭偶发画面像素变化判 Verified；Home 必须到动态解析的 Launcher 包，OpenApp 必须到指定包。OpenApp 使用固定 clear-top 标志，避免旧 Settings Search Task 冒充目标。设备输入前原子地持久写入脱敏 Pending Receipt 与 Audit Intent，写入失败即不输入。结果持久化会原位完成该 Receipt 并追加 Outcome Event；若输入后写入失败，Pending Receipt 仍在、逻辑会话停止，设备结果未知且须人工调查。Type 明文不进入 Request 序列化、Debug、Receipt 或 Event，动作后语义 Capture 脱敏。Frame 仅在内存中；实时画面的截图像素仍可能可视地显示非敏感输入文本，不能拿字符串扫描证明像素已脱敏。

## 真实环境证据

显式门控的 `ORDINCONN_MOBILE_M2_SMOKE=1` 测试在唯一在线的 `OrdinConn_M1_5` AVD 上执行：用真实 Settings Snapshot 的 Element Ref 进入 Network 子页；旧 Ref 在 ADB 输入前被拒绝；真实 Settings 页面 Swipe 后发生变化；Back 返回此前 Settings Activity；进入 Settings Search，输入无个人信息的 `wifi`，验证搜索状态变化及语义脱敏；Home 到动态解析的真实 Launcher；再打开 `com.android.settings`。错误预期 Package 与未授权 OpenApp 均在输入前被阻断。测试持久化每个 Receipt，并核对输入前 Intent 以及 requested/blocked/executed/verified 结果事件。

单独门控的真实密码字段负例在同一模拟器临时安装本地 APK。对其 `password=true` EditText 尝试 Type，返回 `SensitiveScreen`/`Blocked`，没有发送 ADB 输入。随机、非个人测试字符串未出现在序列化 Capture、Receipt、Workspace 或 SQLite 文件中；只保存长度与 Hash 元数据。测试后卸载 APK，本地构建目录移入废纸篓。金融动作词使用合成 UI Fixture 验证，没有连接真实金融 App。

最终打包版 macOS Tauri App 可见地完成 React Inspector → Typed IPC → Rust Policy/ADB → 持久化 Receipt → React Projection：选择可点击的 Settings 元素并手动 Tap，UI Tree 从 70 项变为 65 项，显示 `executed · VERIFIED` 及不同的动作前/后 Snapshot ID。Stop Session 后 UI 回到 Disconnected，`emulator-5554` 仍在线。此前的过期快照手动尝试可见地显示 `blocked · STALE_SNAPSHOT`，均不算成功导航。

M2 改动后重跑 M1.5 真实 Smoke，环境、Capture、Projection、Shutdown 共十项全部通过，包括真实 70 元素脱敏 Settings Tree。默认并行 Desktop Suite 44/44 通过；完整 Rust Workspace、Rust Formatting、禁止 Warning 的 Clippy、Rust Build、30 个 Desktop 与 5 个 Contract TypeScript 测试、Typecheck、Vite Build 及 macOS Tauri Bundle 均通过。

## 强制验收矩阵

证据键：**U**＝单元/夹具自动测试，**R**＝真实 AVD Gate，**G**＝打包 GUI，**S**＝仓库安全门。夹具证据不冒充真实设备证据。

| # | 项目 | 证据 | 结果 |
|---:|---|---|---|
| 1 | Action 契约 | U：六种类型化目标及严格 JSON 拒绝 | PASS |
| 2 | 仅 Emulator 策略 | U；R：专用 AVD 且仅一台在线模拟器 | PASS |
| 3 | Session 前提 | U：非活跃/不匹配 Session 拒绝 | PASS |
| 4 | Allowlist 策略 | U；R：Settings/测试包；生产 Host 拒绝用户添加的任意 App | PASS |
| 5 | Expected Package 前提 | U；R：错误 Package 拒绝 | PASS |
| 6 | Snapshot 新鲜度 | U；R/G：过期 Ref 阻断 | PASS |
| 7 | Element Ref 目标校验 | U；R：绑定快照的 Network Ref | PASS |
| 8 | 敏感目标阻断 | U；R：密码 EditText 拒绝 | PASS |
| 9 | 金融目标阻断 | U：中英文支付、转账、购买、签名夹具 | PASS |
| 10 | 有界 TypeText | U：长度、字符、焦点、EditText | PASS |
| 11 | Type 明文不落盘 | U；R：序列化与 SQLite 扫描 | PASS |
| 12 | Action Budget | U：20 次边界在 ADB 输入前阻断 | PASS |
| 13 | 真实 Tap | R；G：Settings 子页 | PASS |
| 14 | 真实 Swipe | R：Settings UI/Frame 变化 | PASS |
| 15 | 真实 Type | R：Settings Search 变化 | PASS |
| 16 | 真实 Back | R：从 Settings 子页返回 | PASS |
| 17 | 真实 Home | R：Launcher 成为前台 | PASS |
| 18 | 真实 OpenApp | R：Settings 目标包成为前台 | PASS |
| 19 | 过期 Ref 真实阻断 | R：旧 Snapshot Ref，未发送指令 | PASS |
| 20 | 错误 Package 阻断 | R：未发送指令 | PASS |
| 21 | 未授权 App 阻断 | R：未发送指令 | PASS |
| 22 | Action Receipt 持久化 | U：Pending 经重启保留且原位完成；R：最新 Receipt 投影 | PASS |
| 23 | Audit Event | U；R：输入前持久 Intent 与 requested/blocked/executed/verified 结果 | PASS |
| 24 | 动作后 Observation | U；R：新 Frame/UI Capture | PASS |
| 25 | Verification | U；R/G：状态变化及目标包一致 | PASS |
| 26 | Tauri IPC | U：严格输入；G：手动命令响应 | PASS |
| 27 | React 手动控件 | U；G：Inspector 选择与手动 Tap | PASS |
| 28 | Session Shutdown 回归 | U；R/G：逻辑停止，AVD 在线 | PASS |
| 29 | M1.5 回归 | R：十项真实 Smoke；默认并行 Desktop | PASS |
| 30 | 安全门 | S：公开文档、脱敏器、仓库扫描、Diff 检查 | PASS |

**最终 Gate：30 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN。**

## 命令与已知边界

真实 Gate 必须显式设置环境变量；默认 `cargo test --workspace` 绝不会自动操作模拟器：

```bash
ORDINCONN_MOBILE_M2_SMOKE=1 cargo test -p ordinconn-desktop real_m2_navigation_smoke_is_explicitly_gated -- --nocapture
ORDINCONN_MOBILE_M2_SENSITIVE_SMOKE=1 cargo test -p ordinconn-desktop real_m2_sensitive_type_block_is_explicitly_gated -- --nocapture
ORDINCONN_MOBILE_SMOKE=1 ORDINCONN_MOBILE_ALLOWED_APPS=com.android.settings cargo test -p ordinconn-desktop real_emulator_smoke_is_explicitly_gated -- --nocapture
```

Launcher 包经动态检测，与 Settings Intelligence 一样仅加入真实 Smoke 的测试白名单，生产默认值未扩张。验收中曾出现瞬时动作后采集失败、Task 复用导致 OpenApp 目标不匹配，以及 GUI 过期快照尝试；历史失败与最终通过分开记录。动作后采集失败为 `Failed`/`Interrupted`，不自动重试，也不声称 Verified。输入前 SQLite 不可用则不发送输入；输入后存储故障会留下持久 Pending Receipt，设备结果未知，此时停止会话并需人工调查。保守的语义拒绝词表无法证明理解所有语言或纯视觉付款控件；M2 固定导航面避免用户添加金融 App 后只依赖该不完美分类器。M2 Gate 不授权 M3 或任何 X 发布。
