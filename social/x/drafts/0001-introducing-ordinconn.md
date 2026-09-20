# X Draft 0001 — Introducing OrdinConn

- Stage: Project introduction / Mobile Intelligence M1.5 verified
- GitHub Issue: https://github.com/Btkkgo/OrdinConn/issues/1
- Known Reliability Issue: https://github.com/Btkkgo/OrdinConn/issues/4
- Verified Product Commit: https://github.com/Btkkgo/OrdinConn/commit/f8705c5
- Corrected Evidence Commit: https://github.com/Btkkgo/OrdinConn/commit/78dc1ef
- Status: DRAFT — ready for manual owner publication; M1.5 VERIFIED; M2 NOT STARTED
- Repository: https://github.com/Btkkgo/OrdinConn

## English — Publication Version

### 1/5

I’m building OrdinConn: an open-source, model-agnostic AI Agent runtime. It is not another chatbot. The goal is a durable path from AI Model → OrdinConn Runtime → Perception → Tools / Apps → Action → Verification.

### 2/5

Model reasoning keeps improving, but real work still needs stable permissions, screen/window context, files, mobile state, task state, audit trails, and post-action verification. OrdinConn keeps that infrastructure outside any one model provider.

### 3/5

The current direction is Mobile Intelligence: Android Emulator → Frame → UI Tree → Element Refs → MobileObservation → Agent Action. Structured perception comes before vision, app access is allowlisted, sensitive fields are redacted, and claims require real validation.

### 4/5

The first gate stopped honestly: no SDK, adb, Emulator, or AVD. After building a real Android 16 ARM64 Pixel 8 environment, M1.5 finished 15/15 PASS: frame, 70 sanitized UI elements, redaction, Tauri IPC, and shutdown. M2 is NOT STARTED.

### 5/5

Code generated ≠ task completed. Read → Scope → Implement → Real Test → Verify → Move Forward. Issue #4 tracks parallel-test flakiness. Repo: github.com/Btkkgo/OrdinConn · M1.5: github.com/Btkkgo/OrdinConn/issues/1

## 中文 — 参考版本

### 1/5

我正在开发 OrdinConn：一个开源、模型无关的 AI Agent Runtime。它不是另一个聊天机器人。目标是建立稳定链路：AI Model → OrdinConn Runtime → Perception → Tools / Apps → Action → Verification。

### 2/5

模型推理能力持续增强，但真实工作仍需要稳定的权限、屏幕与窗口上下文、文件、移动环境状态、任务状态、审计轨迹和操作后验证。OrdinConn 把这些基础设施放在任何单一模型 Provider 之外。

### 3/5

当前方向是 Mobile Intelligence：Android Emulator → Frame → UI Tree → Element Refs → MobileObservation → Agent Action。Structured Perception 优先于 Vision；App Access 使用 Allowlist；敏感字段必须脱敏；完成声明必须经过真实验证。

### 4/5

第一次 Gate 如实停止：SDK、adb、Emulator 与 AVD 均不可用。建立真实 Android 16 ARM64 Pixel 8 环境后，M1.5 最终 15/15 PASS：Frame、70 个已脱敏 UI Element、Redaction、Tauri IPC 与 Shutdown 全部通过。M2 仍为 NOT STARTED。

### 5/5

真实 Codex 经验：生成代码不等于完成任务。正确循环是 Read → Scope → Implement → Real Test → Verify → Move Forward。Issue #4 继续公开跟踪并行测试 Flakiness。仓库：github.com/Btkkgo/OrdinConn · M1.5 证据：github.com/Btkkgo/OrdinConn/issues/1

## Suggested Images

1. **Architecture main image:** `AI Model ↓ OrdinConn Runtime ↓ Computer / Android / Apps`. Keep it simple, English-primary, and do not imply unimplemented M2 actions.
2. **Real Android Emulator:** the public-safe `OrdinConn_M1_5` Android Settings screen, with no account, notification, or personal data.
3. **M1.5 acceptance matrix:** `15 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN`, covering Frame, UI Tree, Redaction, Observation, Tauri IPC, and Shutdown.

Do not fabricate screenshots. Remove local paths, account identity, notifications, and private application content before manual publication.

## Manual Publication Reminder

This draft is ready for owner review and manual publication. Repository automation must not log in to X, read cookies or browser sessions, store an X token, call the X API, or publish this draft.
