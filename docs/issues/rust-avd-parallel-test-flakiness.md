# English

## Goal

Make the AVD lifecycle and subprocess fixtures deterministic under repeated default-parallel Rust test execution without weakening production deadlines or forcing the whole suite to run serially.

## Summary

Rust AVD lifecycle and subprocess fixture tests fail intermittently under default parallel execution.

## Current State

Issue #4 remains **OPEN** with `status:needs-validation`. It does not block the completed M1.5 real-environment acceptance, but it remains an independent runtime and test-reliability risk.

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

Open and not yet resolved. The final workspace and serial reruns were green, but earlier failures were reproduced more than once.

## Validation

Current evidence consists of the initial 21/24 desktop result, the 24/24 serial result, two reproduced AVD lifecycle failures during the corrective workspace run, its later 125/125 workspace rerun, and the fresh stage-close sequence of 24/28 desktop tests on the first workspace attempt, 28/28 serial desktop tests, then 125/125 on the next workspace rerun.

## Boundary

This Issue does not authorize M2 or any weakening of production timeouts.

# 中文

## 目标

让 AVD Lifecycle 与 Subprocess Fixture 在重复的 Default-parallel Rust Test 中保持确定，同时不削弱 Production Deadline，也不强制整个 Suite 串行运行。

## 摘要

Rust AVD Lifecycle 与 Subprocess Fixture Test 在默认并行执行下存在间歇性失败。

## 当前状态

Issue #4 继续保持 **OPEN** 和 `status:needs-validation`。它不阻塞已经完成的 M1.5 真实环境验收，但仍是独立的 Runtime/Test Reliability 风险。

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

仍为 Open，尚未解决。最终 Workspace 与 Serial 重跑为绿色，但此前失败已经不止一次复现。

## 验证

当前 Evidence 包括最初 21/24 Desktop Result、24/24 Serial Result、Corrective Workspace Run 中再次出现的两个 AVD Lifecycle Failure 及之后的 125/125 Workspace Rerun，以及本次 Stage Close 的新鲜序列：第一次 Workspace Attempt 中 Desktop 24/28、Serial Desktop 28/28、下一次 Workspace Rerun 125/125。

## 边界

本 Issue 不授权 M2，也不授权削弱 Production Timeout。
