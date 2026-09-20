# Mobile Intelligence M1.5 验收

[English](M1_5_ACCEPTANCE.md) | [简体中文](M1_5_ACCEPTANCE.zh-CN.md)

## 目的

M1.5 证明 Production OrdinConn Host 能通过 Typed Tauri Boundary 使用真实、现有 Android Emulator。它是 Observe-only M1 与 Verified Navigation M2 之间的强制 Gate。

## 环境发现

Detector 按以下顺序搜索：

1. OrdinConn Settings 保存的 Android SDK Path。
2. `ANDROID_SDK_ROOT`。
3. `ANDROID_HOME`。
4. macOS 上的 `~/Library/Android/sdk`。
5. Process `PATH` 中的单个 Tool。

它解析并报告 `adb`、`emulator`、`sdkmanager`、`avdmanager` 的真实路径、`adb version` 第一行、现有 AVD Metadata 与 Online Emulator Device。它不会下载或安装缺失 Component。

## 自动化行为覆盖

`cargo test -p ordinconn-desktop` 覆盖 SDK-root/Direct-ADB Normalization、Tool Discovery、AVD Metadata Parsing、Online/Offline/Unauthorized/Physical Device Filtering、Unknown AVD Rejection、Bounded Boot Timeout、`sys.boot_completed` Readiness、UIAutomator Parsing、Redaction、Allowlisting 与 Idempotent Logical Session Shutdown。

这些测试使用受控 Process Fixture，不能算作真实 Android Evidence。

## 真实 Gate

在现有 Emulator 中打开安全且明确加入 Allowlist 的 Package 后运行：

```bash
ORDINCONN_MOBILE_SMOKE=1 \
ORDINCONN_MOBILE_ALLOWED_APPS=com.android.settings \
cargo test -p ordinconn-desktop real_emulator_smoke -- --nocapture
```

没有 Device Online 时，Harness 可以启动一个现有 AVD。它最多等待 120 秒让 ADB 与 Android Boot 完成，然后使用 Production Observation Path 输出以下十项 JSON Check：

1. `ADB_READY`
2. `EMULATOR_READY`
3. `DEVICE_ONLINE`
4. `FRAME_CAPTURE`
5. `UI_TREE_CAPTURE`
6. `SNAPSHOT_PARSE`
7. `ELEMENT_REFS`
8. `MOBILE_OBSERVATION`
9. `WORKSPACE_PROJECTION`
10. `SESSION_SHUTDOWN`

每项都必须为 `PASS`。缺失前置条件会让依赖项变为 `BLOCKED`；Skipped 或 Unknown Result 不能转换为 `PASS`。

`WORKSPACE_PROJECTION` 只有在真实 Capture 经过 `observe_mobile_device` 使用的 Persistence/Projection Helper 后才通过：SQLite Capture Persistence、三个允许的 Mobile Audit Event、包含 Observation/Feed Item 的 `MobileWorkspaceData` Projection 与 JSON Serialization 必须全部成功。它不被表述为 Frontend 调用 Tauri Command 的证据。`SESSION_SHUTDOWN` 还要求 Logical Host Shutdown 与持久化 `ended` Session Transition 同时完成。

Typed Tauri IPC 是独立的 Real-desktop Acceptance Item。Packaged Application 必须可见地证明 Rust → Tauri Command/Event → React Path，覆盖 Initial Status、Controlled Error、Session Start、Observation 与 Session Stop。同一 Session 的 Persisted Event Sequence 必须包含 `mobile.session_started`、`mobile.snapshot`、`mobile.observation` 与 `mobile.session_ended`。

Sensitive-node Acceptance 使用含一个 Password Input 和临时 Test Value 的本地临时 Application。该 App 只在 Test 期间加入 Allowlist。运行独立 Gate 时不能把测试值写入 Source Control：

```bash
ORDINCONN_MOBILE_SENSITIVE_SMOKE=1 \
ORDINCONN_MOBILE_TEST_SECRET=<local-test-value> \
cargo test -p ordinconn-desktop real_sensitive_redaction_smoke -- --nocapture
```

检查要求至少一次 Recorded Redaction、`SensitiveFieldBlocked`、一个 `[REDACTED]` Element，且 Serialized Capture Data 中不得出现测试值。

## 已观察本地结果 — 2026-09-20

- Host：macOS arm64 / Apple Silicon。
- Java：OpenJDK 21.0.12.1。
- SDK Root：`~/Library/Android/sdk`。
- ADB：37.0.1。
- Emulator：37.1.11，使用 Hypervisor.Framework Acceleration。
- System Image：`system-images;android-36;google_apis;arm64-v8a` Revision 7。
- AVD：`OrdinConn_M1_5`，Pixel 8 Profile。
- Online Device：`emulator-5554`，Android 16 / API 36，1080×2400，420 dpi。
- 最终 Automated Rerun Frame：188,909-byte PNG，SHA-256 非空。
- UI Tree：70 个有效已脱敏 Element；忽略两个 Bounds 反转的 Android-generated Node。
- Snapshot/Ref/Observation/Workspace Projection/Shutdown：通过 Production Persistence、Audit、Projection、Serialization 与 Logical-session Path，结果为 `PASS`。
- Tauri IPC：在 Packaged Desktop Application 中为 `PASS`。Empty Allowlist 产生预期 Frontend Error；允许 `com.android.settings` 后，UI 显示 `Observing`、`emulator-5554`、`com.android.settings`、`VERIFIED`、真实 Frame 与 70 个 UI Element。Stop 让 UI 返回 `Disconnected`，持久化 `mobile.session_ended`，并让 Emulator 保持 Online。
- Sensitive-node Check：一个真实 Password Node、一次 OrdinConn Redaction，Serialized Capture Data 不含测试值。

## 最终强制验收矩阵

1. Android SDK Readiness：`PASS`
2. ADB Readiness：`PASS`
3. Emulator Readiness：`PASS`
4. Dedicated AVD Readiness：`PASS`
5. Online Emulator Device：`PASS`
6. Real Frame Capture：`PASS`
7. Real UI-tree Capture：`PASS`
8. Sensitive-node Redaction：`PASS`
9. Snapshot Parse：`PASS`
10. Snapshot-scoped Element Reference：`PASS`
11. Real `MobileObservation`：`PASS`
12. Persisted/Audited Workspace Projection：`PASS`
13. Packaged Tauri IPC Path：`PASS`
14. Controlled Allowlist Error：`PASS`
15. Persisted Logical Session Shutdown：`PASS`

最终计数：**15 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN**。

结果：**M1.5 PASS — M2 仍为 NOT STARTED。** 未实现或启用任何 Mobile Navigation Action。

第一次 Corrective Default-parallel Workspace Run 中，两个现有 AVD Lifecycle Process Fixture 失败，另有一个 Gate-label Assertion 随后被修正。修正后 Default-parallel Workspace 重跑 125 个 Rust Test 全部通过，28 个 Desktop Test 串行全部通过。AVD Fixture Nondeterminism 继续记录在 Issue #4，不能被后续绿色重跑隐藏。
