# X Draft 0002 — Mobile Intelligence M1.5 Real Validation

- Stage: Mobile Intelligence M1.5 real-environment acceptance
- GitHub Issue: https://github.com/Btkkgo/OrdinConn/issues/1
- Follow-up Issue: https://github.com/Btkkgo/OrdinConn/issues/4
- GitHub Commit: https://github.com/Btkkgo/OrdinConn/commit/f8705c5
- Status: DRAFT — manual owner review required; M1.5 PASS; M2 NOT STARTED
- Repository Link: https://github.com/Btkkgo/OrdinConn

## English — Publication Version

### Suggested Post

OrdinConn Mobile Intelligence M1.5 now passes against a real Android 16 ARM64 Emulator. The packaged desktop also proved status, allowlist error, observation, and stop through the real Rust → Tauri → React path. Sensitive redaction passed; M2 remains intentionally not started.

### Suggested Thread

### 1/5

OrdinConn Mobile Intelligence was blocked by a missing Android runtime. The rule was simple: fixtures could prove parser behavior, but they could not substitute for a real Emulator, frame, UI tree, observation, IPC path, or shutdown.

### 2/5

The new environment is deliberately narrow: OpenJDK 21, official Android command-line tools, Android 36 Google APIs ARM64, and one dedicated Pixel 8 AVD named `OrdinConn_M1_5`. No Android Studio install and no existing AVD was modified.

### 3/5

The real device exposed three fixture-blind problems: PNG output filled a child-process pipe, Android 16 changed the useful window-dump surface, and UIAutomator emitted platform nodes with reversed bounds. Each fix received a failing regression before passing.

### 4/5

Final result: 188,909-byte frame, 70 sanitized UI elements/refs, a real MobileObservation, persisted/audited workspace projection, and clean shutdown. Packaged Tauri GUI IPC passed separately. A real password node was redacted without serialized plaintext.

### 5/5

Technical lesson: real integration evidence is also schema discovery. Codex lesson: a truthful blocked gate creates the exact checklist needed for the eventual pass. M2 remains NOT STARTED pending owner review. https://github.com/Btkkgo/OrdinConn/issues/1

### Suggested Screenshots

1. The visible `OrdinConn_M1_5` Android Emulator showing the public-safe Android Settings home page with no account, notification, or personal data.
2. A sanitized terminal crop showing `adb devices -l` plus the 15-item M1.5 PASS matrix; remove the local username path and unnecessary host metadata.
3. The existing OrdinConn Mobile diagnostic/observation view if it can be shown without private content. Do not add product UI solely for this draft.

### Technical Lesson

Draining subprocess output is part of timeout correctness. A producer blocked on a full pipe cannot exit, so reading only after exit turns a healthy large capture into a false timeout. Real operating-system diagnostic formats must also be treated as versioned external schemas.

### Codex Lesson

The strongest contribution was preserving the first blocked result, then using the real dependency to drive three evidence-backed fixes instead of weakening the gate. A later green rerun does not erase an intermittent concurrency failure, so that test-harness risk remains public in Issue #4.

## 中文 — 参考版本

### 1/5

OrdinConn Mobile Intelligence 最初因缺少 Android Runtime 而受阻。规则很简单：Fixture 可以证明 Parser 与 Policy，但不能代替真实 Emulator、Frame、UI Tree、Observation、IPC Path 或 Shutdown。

### 2/5

新环境刻意保持最小化：OpenJDK 21、官方 Android Command-line Tools、Android 36 Google APIs ARM64，以及一个名为 `OrdinConn_M1_5` 的专用 Pixel 8 AVD。没有安装 Android Studio，也没有修改已有 AVD。

### 3/5

真实设备暴露了三个 Fixture 未覆盖的问题：PNG Output 填满 Child-process Pipe、Android 16 改变有效 Window-dump Surface，以及 UIAutomator 输出 Bounds 反转的 Platform Node。每项修复都先看到 Regression Test 失败，再验证通过。

### 4/5

最终结果：15/15 强制项通过。真实 Capture 得到 188,909-byte Frame、70 个已脱敏 UI Element/Ref、`MobileObservation`、持久化与审计的 Workspace Projection，并完成 Clean Shutdown。Packaged Tauri GUI IPC 与真实 Password-node Redaction 另行通过。

### 5/5

技术经验：真实 Integration Evidence 同时也是 Schema Discovery。Codex 经验：如实保留受阻 Gate，才能得到最终通过所需的准确清单。M2 仍为 NOT STARTED；Issue #4 继续跟踪并行测试 Flakiness。https://github.com/Btkkgo/OrdinConn/issues/1

### 人工发布提醒

发布由用户控制。请根据 Issue #1 与 Commit `f8705c5` 检查每项声明，只选择可安全公开的 Screenshot，并人工发布。Repository Automation 不得登录 X、读取 Cookie/Session、保存 X Token、调用 X API 或发布本 Draft。
