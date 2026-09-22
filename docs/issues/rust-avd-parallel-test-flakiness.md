# English

## Goal

Make the AVD lifecycle and subprocess fixtures deterministic under repeated default-parallel Rust test execution without weakening production deadlines or forcing the whole suite to run serially.

## Summary

Rust AVD lifecycle and subprocess fixture tests fail intermittently under default parallel execution.

## Current State

Issue #4 reached **VERIFIED** on the reliability branch after repeated default-parallel, high-concurrency, and real Android validation. The historical failure remains documented. It never invalidated the completed M1.5 real-environment acceptance.

## Observed Behavior

- Initial default-parallel desktop run: 21/24 passed; three process-fixture tests timed out.
- Serial desktop run at that checkpoint: 24/24 passed.
- First corrective default-parallel workspace run: two AVD lifecycle fixture failures were reproduced; one unrelated gate-label assertion was also corrected.
- Final default-parallel workspace rerun: 125/125 passed.
- Final serial desktop run: 28/28 passed.
- Fresh stage-close default-parallel workspace run: 24/28 desktop tests passed and four timing-sensitive fixtures failed (`adb_command_drains_large_stdout_before_process_exit`, both AVD lifecycle cases, and `command_returns_after_success_when_descendant_keeps_stdout_open`).
- Immediate stage-close serial desktop run: 28/28 passed.
- Immediate stage-close default-parallel workspace rerun: 125/125 passed.

The later green rerun does not prove that the intermittent concurrency sensitivity has been eliminated.

On 2026-09-22, the first of ten pre-change default-parallel desktop runs again failed the same four tests at 24/28; the next nine warm runs passed. Each named case passed 20 isolated runs, and related warm concurrent tests passed 20 runs each at eight and 32 threads.

## Impact

The real M1.5 gate, sensitive redaction, packaged Tauri IPC, and logical shutdown passed independently. Issue #4 therefore does not invalidate M1.5, but it must not be hidden or closed on the strength of one green rerun.

## Validation Needed

- Reproduce or disprove the timing sensitivity across repeated default-parallel runs.
- Isolate lifecycle fixtures and shared process resources.
- Keep production command deadlines bounded.
- Make the default-parallel suite deterministic without globally forcing serial execution.

## Acceptance Criteria

- Repeated default-parallel workspace and desktop runs remain green.
- Fixture processes and shared resources are isolated.
- Production deadlines remain bounded and unchanged unless new evidence justifies a reviewed change.
- No global serial-test workaround hides the concurrency defect.

## Result

The root cause was a one-second success-path test budget that also measured cold, parallel external-process startup. Distinct temporary SDK/AVD roots, scripts, and process groups were already isolated; no shared path, fixed port, or global environment mutation was found. A new four-fixture concurrent cold-start regression failed under that original budget and passed with a bounded three-second test-only budget. Successful large-output and inherited-pipe fixtures now use a bounded two-second test budget. Deliberate 30 ms command-timeout and 50 ms no-boot tests, production deadlines, and production runtime code are unchanged. No global serial or retry workaround was used.

## Validation

Historical evidence remains the initial 21/24 desktop result, 24/24 serial result, two reproduced AVD lifecycle failures, a later 125/125 workspace rerun, and the stage-close 24/28 desktop → 28/28 serial → 125/125 workspace sequence. New verification: default-parallel desktop 20/20 runs at 29/29 each; full workspace 10/10 runs at 136 tests each; eight-thread desktop 29/29; Rust formatting, Clippy with warnings denied, 31 TypeScript tests, typecheck, Rust/Vite builds, and Tauri bundle PASS. The first real Android smoke failed closed on an unallowlisted cold-boot Launcher; after selecting Settings on the dedicated AVD, the unchanged smoke passed all ten checks, including frame, UI tree, `MobileObservation`, and logical shutdown.

## Boundary

This Issue does not authorize M2 or any weakening of production timeouts.

# 中文

## 目标

让 AVD Lifecycle 与 Subprocess Fixture 在重复的 Default-parallel Rust Test 中保持确定，同时不削弱 Production Deadline，也不强制整个 Suite 串行运行。

## 摘要

Rust AVD Lifecycle 与 Subprocess Fixture Test 在默认并行执行下存在间歇性失败。

## 当前状态

Issue #4 在 Reliability 分支经过重复默认并行、高并发与真实 Android 验证后达到 **VERIFIED**。历史失败继续保留记录。它从未推翻已经完成的 M1.5 真实环境验收。

## 已观察现象

- 最初 Default-parallel Desktop Run：21/24 通过；3 个 Process-fixture Test Timeout。
- 同一检查点的 Serial Desktop Run：24/24 通过。
- 第一次 Corrective Default-parallel Workspace Run：再次出现 2 个 AVD Lifecycle Fixture Failure；同时修正了 1 个无关的 Gate-label Assertion。
- 最终 Default-parallel Workspace 重跑：125/125 通过。
- 最终 Serial Desktop Run：28/28 通过。
- 本次 Stage Close 的新鲜 Default-parallel Workspace Run：Desktop Test 为 24/28，4 个 Timing-sensitive Fixture 失败（`adb_command_drains_large_stdout_before_process_exit`、两个 AVD Lifecycle Case，以及 `command_returns_after_success_when_descendant_keeps_stdout_open`）。
- 紧接着的 Stage Close Serial Desktop Run：28/28 通过。
- 紧接着的 Stage Close Default-parallel Workspace 重跑：125/125 通过。

后续一次绿色重跑不能证明间歇性并发敏感问题已经消失。

2026-09-22 改动前十次 Default-parallel Desktop 运行的第一次再次由同样四个用例形成 24/28，随后九次热态运行通过。四个具名用例各自单独运行 20 次通过，相关热态并发测试在八线程和 32 线程下也各通过 20 次。

## 影响

真实 M1.5 Gate、Sensitive Redaction、Packaged Tauri IPC 与 Logical Shutdown 已独立通过。因此 Issue #4 不推翻 M1.5，但不能因为一次绿色重跑而被隐藏或关闭。

## 后续验证

- 通过重复 Default-parallel Run 复现或排除 Timing Sensitivity。
- 隔离 Lifecycle Fixture 与 Shared Process Resource。
- 保持 Production Command Deadline 有界。
- 在不全局强制 Serial Execution 的前提下，让 Default-parallel Suite 变得确定。

## 验收标准

- 重复执行 Default-parallel Workspace/Desktop Run 时持续保持绿色。
- Fixture Process 与 Shared Resource 得到隔离。
- Production Deadline 保持有界且不变，除非新 Evidence 支撑经过 Review 的调整。
- 不用全局 Serial-test Workaround 隐藏并发缺陷。

## 结果

根因是成功路径使用一秒测试预算，同时把冷态并行外部进程启动耗时计入其中。各测试原本已有独立 Temp SDK/AVD Root、脚本与 Process Group；审计未发现共享路径、固定端口或全局环境变量修改。新增四夹具并发冷启动回归在原预算下失败，改为有界三秒测试专用预算后通过。成功的 Large-output 与 Inherited-pipe 夹具改用有界两秒测试预算。刻意验证超时的 30ms Command Test 和 50ms No-boot Test、生产 Deadline 与生产 Runtime Code 均未改变。没有使用全局串行或重试方案。

## 验证

历史 Evidence 保留最初 21/24 Desktop、24/24 Serial、再次出现的两个 AVD Lifecycle Failure、后续 125/125 Workspace，以及 Stage Close 的 Desktop 24/28 → Serial 28/28 → Workspace 125/125。新验证：Default-parallel Desktop 20/20 次、每次 29/29；完整 Workspace 10/10 次、每次 136 项；八线程 Desktop 29/29；Rust Formatting、禁止 Warning 的 Clippy、31 个 TypeScript 测试、Typecheck、Rust/Vite Build 和 Tauri Bundle 均 PASS。真实 Android Smoke 首次因冷启动 Launcher 不在白名单而按设计拒绝；在专用 AVD 上切换到 Settings 后，原样 Smoke 十项全部 PASS，包括 Frame、UI Tree、`MobileObservation` 与逻辑关闭。

## 边界

本 Issue 不授权 M2，也不授权削弱 Production Timeout。
