# Desktop 启动诊断 — 2026-10-02

[English](DESKTOP_STARTUP_DIAGNOSIS.md) | [简体中文](DESKTOP_STARTUP_DIAGNOSIS.zh-CN.md)

[Issue #15](https://github.com/Btkkgo/OrdinConn/issues/15)，关联 [Issue #14](https://github.com/Btkkgo/OrdinConn/issues/14)。源码基线：`codex/realtime-workbench` / `0eff77e6e08b1170589d397dc52645b4fc0aa132`。

## 已证明的根因

保留的 `target/release/bundle/macos/OrdinConn.app` 是从 `d3e6c1488135169dcfc5939d82d772606206b923` 构建的冻结二进制，不是当前 Mobile Data Acquisition 可执行文件。其 SHA256 与冻结记录相符，Mach-O UUID 与崩溃报告相符：`85E502DA-0D8D-3888-B872-33E8162DFB05`。

冻结源码只内嵌 SQLite 迁移 1–11。当前开发版本启动时，共用应用数据库已在 10 月 2 日 05:28:41 UTC 应用迁移 12。只读对比确认迁移 1–11 的校验和全部一致，迁移 12 是冻结源码唯一缺失的已应用版本。SQLx 0.8.6 的 `validate_applied_migrations` 对此返回 `MigrateError::VersionMissing(12)`。临时数据库测试复现了该精确错误，并确认当前 runtime 可以重新打开同一数据库且保留哨兵数据。

这是**过时 packaged executable 与较新共用 schema 不兼容**，不是当前迁移、MobileHost、ADB、Extractor、UI 或新增嵌套 async runtime 导致的故障。

## Crash 与启动调用链

用户提供的 10 月 2 日 20:06:05 +0800 报告及 19:59 报告均显示 Thread 0 / `com.apple.main-thread`：`__pthread_kill → pthread_kill → abort → ordinconn-desktop`，偏移 `0xd49c84`、`0xd49a0c`，随后为应用启动回调。Release 可执行文件已移除符号；没有将未知偏移伪称为已解析源码符号。

终端复现提供了上游源码证据：

```text
tauri-2.11.5/src/app.rs:1425:11
Failed to setup app: error encountered during setup hook: database migration error
panic in a function that cannot unwind
thread caused non-unwinding panic. aborting.
```

源码路径：`apps/desktop/src-tauri/src/lib.rs::run` 的 setup closure → `AppRuntime::initialize` → `crates/ordinconn-app/src/db.rs::open_database` 第 19 行（`sqlx::migrate!().run`）→ `AppError::Migration` → Tauri `make_run_event_loop_callback` / `RuntimeRunEvent::Ready` 第 1425 行。Tauri 对返回的 setup 错误触发 panic，随后在 native callback 边界 abort。失败发生在 `MobileHost::discover`、`app.manage` 和采集 command 运行之前。Setup 完成前分配 WebView 不代表应用启动成功。

20:20:45 +0800 的终端直接复现生成了新的 SIGABRT / signal 6 报告。终端工具返回 exit status 1；OS Crash Report 单独证明 signal 6。stdout 为空，stderr 保留了上述 panic。原始日志和报告只留在本地，不提交。

部分终端启动停在 macOS 的窗口恢复提示。一秒主线程采样显示 `NSPersistentUIRestorer::promptToIgnorePersistentStateWithCrashHistory → NSAlert runModal`。选择“不恢复窗口”后才继续 setup 并复现迁移 abort。停留在提示框不算启动成功；终端启动也没有消除底层不兼容。

## 恢复方式与边界

使用现有正式入口 `npm run desktop:dev`，设置 `ORDINCONN_INTERNAL_M3_ACCEPTANCE=0` 并关闭真实验收 opt-in，以当前 debug 可执行文件打开现有数据库。开发入口恢复不需要修改生产源码、回滚数据库、跳过迁移、覆盖 schema、替换 bundle 或签名。冻结 bundle 仍与较新数据库不兼容，不是 Issue #14 的有效启动入口。

当前开发启动打开了现有 SQLite/WAL，创建 WebKit，完成主 frame 加载，并在回归期间持续运行。仅编辑新增 fixture 测试文件时由开发 watcher 自动重启。Native UI 自动化工具无法绑定未打包的 debug 进程，独立视觉确认仍由用户提供。WebKit 加载、持续桌面轮询和进程存活与 H1 分开记录。

已启动既有 `OrdinConn_M1_5` emulator，只读 ADB inventory 确认 `emulator-5554` 已连接。未执行 Observe 或 Android Action；H1–H7 仍未验证。Android Actions = 0；Provider requests = 0；Signing verification = 0；X Draft = NONE。

## 此前自动测试为何未捕获

此前测试和编译使用当前源码、当前迁移及 fixture 数据库，没有用刻意未替换的旧 bundle 打开已升级至迁移 12 的用户数据库。开发进程存在也不能证明 Finder/LaunchServices 会启动哪个 App。本次补充旧 migrator / 新 schema 的兼容性 fixture，并明确二进制身份属于独立启动前提。

## 验证

- Rust：338 PASS；七项真实 Android/final gate 明确排除。首轮沙箱不允许绑定本地 HTTP/WebSocket fixture 端口，在解除该限制后同一套回归通过。
- Desktop：73 PASS；Contracts：9 PASS。typecheck、frontend build、native Rust build、Rust 格式检查：PASS。
- 新诊断 fixture 初次遗漏必填时间戳，已修正后通过完整回归；没有为测试修改生产代码。
- 旧 packaged app 仍复现 SIGABRT。开发启动恢复不代表 packaged app 恢复，也不代表人工手机验收通过。
- 保留原有八项无关未跟踪内容、M3 状态/历史、Provider 配置、Keychain 与冻结可执行文件。
