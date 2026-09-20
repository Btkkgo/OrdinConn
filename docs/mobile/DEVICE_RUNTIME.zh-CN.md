# Device Runtime

[English](DEVICE_RUNTIME.md) | [简体中文](DEVICE_RUNTIME.zh-CN.md)

## M1 Device Contract

`MobileDeviceSession` 记录 Session/Device Identity、Platform、Device Type、OS Version、Screen Size、Connection Time、Current Application/Activity、Status 与 Latest Observation Time。

第一版 Adapter 面向 macOS Apple Silicon 上的 Android Emulator，使用 Android Studio AVD、ADB 与 UIAutomator。上层 Protocol 保持平台无关，便于以后增加 Physical Android 与 iOS Adapter。

## Observation Sequence

1. 按 Saved Setting、`ANDROID_SDK_ROOT`、`ANDROID_HOME`、`~/Library/Android/sdk`、Process Path 的顺序寻找现有 Android SDK。
2. 解析 `adb`、`emulator`、`sdkmanager` 与 `avdmanager`，不要求 Shell PATH 已配置。
3. 列出现有 AVD 与 Online Emulator Device。
4. 读取 Package/Activity 与 Screen Size。
5. 强制 Application Allowlist。
6. Capture UIAutomator Hierarchy。
7. 对 Sensitive Node 脱敏并分配 Snapshot-scoped Element Ref。
8. Capture 最新 PNG Frame。
9. 从已脱敏 Semantic Snapshot 创建 `MobileObservation`。
10. 持久化 Session、Snapshot、Observation 并追加 Audit Event。

Frame 只保留在有界 In-memory Ring Buffer 中。M1 不持续录制 Video。

## 真实状态

Runtime 暴露 `unavailable`、`disconnected`、`observing`、`paused` 与 `error` 状态。Command Exit Code 本身不能证明 Observation 成功；必需的 Package/Activity、UI Tree 与 Frame Output 都必须通过校验。

## M1.5 环境诊断

Settings 显示 SDK、ADB、Emulator、AVD 与 Online-device Readiness 及解析后的 Path。Detection 不安装 Android Studio、SDK Package、System Image、Application 或 AVD。

`list_avds` 通过 `availableAvds` Diagnostic Collection 表示。每个 Entry 保留 AVD Name，并在现有 `config.ini` 提供信息时报告 Running State、Device Profile 与 Architecture。

启动现有 AVD 时使用发现的 Emulator Executable，然后同时等待 Online `emulator-*` ADB Device 与 `sys.boot_completed=1`。Production Timeout 为 120 秒。Timeout 不会让 OrdinConn 删除、Reset 或终止 AVD。OrdinConn Shutdown 只结束其逻辑 Mobile Session。

Environment Probe 的 Total Deadline 为 5 秒，单个 Observation Command 为 10 秒。Tauri 在 Blocking Worker Pool 中执行这些 Blocking Host Operation，避免挂起 Android Tool 无限占用 Async Command Executor。120 秒 AVD Deadline 包含 Discovery 与 Boot Polling。

Child Process 运行时持续读取 ADB stdout/stderr。真实 PNG Capture 需要这一点，因为若先等待 Process Exit 再读 Pipe，Frame 超过操作系统 Pipe Buffer 后会 Deadlock。Foreground Detection 使用完整 `dumpsys window`，因为 Android 16 的窄版 `dumpsys window windows` 不再包含 `mCurrentFocus`。

UIAutomator Node 的 Rectangle 非法或反转时，只丢弃该 Node，保留其余有效 Tree。Platform 生成的 Off-screen Node 不应破坏可用的真实 Snapshot，任何非法 Rectangle 都不会成为 Element Ref。

Logical Shutdown 同样持久化：Session Row 变为 `ended`，并追加已脱敏 `mobile.session_ended` Event。OrdinConn 仍不会 Kill Emulator Process。

## 已验证环境

M1.5 已在 macOS arm64、OpenJDK 21、Android API 36 Google APIs ARM64、Emulator 37.1.11、ADB 37.0.1 与专用 `OrdinConn_M1_5` Pixel 8 AVD 上通过。Packaged Desktop Path 也验证了 Status、Controlled Allowlist Error、Observation 与 Logical Stop，且没有终止 AVD。该证据只覆盖 Observation，不授权 M2 Action。
