# X Draft 0001 — Introducing OrdinConn

- Stage: Open-source initialization / Mobile Intelligence M1.5
- GitHub Issue: https://github.com/Btkkgo/OrdinConn/issues/1
- Public workflow Issue: https://github.com/Btkkgo/OrdinConn/issues/2
- GitHub Commit: open-source documentation `3d9434d`; product baseline `c4f2d66`
- Status: DRAFT — manual owner review required; M1.5 NOT PASSED
- Repository Link: https://github.com/Btkkgo/OrdinConn

## Suggested Post

I’m building OrdinConn: an open-source, model-agnostic runtime for AI agents to perceive, understand, and operate computers and mobile environments—with explicit permissions, evidence provenance, and post-action verification. Development is public on GitHub.

## Suggested Thread

### 1/5

I’m building OrdinConn: an open-source, model-agnostic runtime for AI agents to perceive, understand, and operate computers and mobile environments. Not another chatbot—the runtime is the durable layer between reasoning and real tools.

### 2/5

Models can change. Permissions, provenance, tool contracts, audit trails, and post-action verification still need stable infrastructure. OrdinConn keeps those boundaries outside any one model provider.

### 3/5

The current direction is Mobile Intelligence: Emulator → Frame → UI Tree → Element Refs → MobileObservation. Structured UI state comes before vision, access is app-allowlisted, and sensitive nodes must be redacted.

### 4/5

The first real M1.5 gate failed honestly: Android SDK, ADB, and Emulator were unavailable. Result: 3 FAIL, 7 BLOCKED. No frame or UI tree was fabricated, and M2 did not start. github.com/Btkkgo/OrdinConn/issues/1

### 5/5

Codex lesson: fixture tests prove parsers and policy—not a real integration. A useful gate names prerequisites, blocks dependent claims, and preserves failure as engineering evidence. Source and DevLogs: github.com/Btkkgo/OrdinConn

## Suggested Screenshots

1. OrdinConn architecture: `AI Model → OrdinConn Runtime → Computer / Android / Apps`.
2. Real M1.5 terminal result showing Android SDK missing, ADB FAIL, Emulator FAIL, and AVD FAIL.
3. Mobile Intelligence pipeline: `Emulator → Frame → UI Tree → Element Refs → Observation → Action`.

Do not fabricate screenshots. Remove any private app content, account identity, or local home path before manual publication.

## Technical Lesson

Real-environment acceptance and fixture coverage are separate evidence classes. A downstream stage must remain blocked when its external prerequisites are absent.

## Codex Lesson

Codex was most useful when the workflow forced explicit scope, named gates, independent review, and evidence-backed completion language. It could not turn a missing Android runtime into a verified integration.

## Chinese Reference

### 项目介绍

OrdinConn 是一个开源、模型无关的 AI Agent Runtime，目标是让不同模型通过稳定运行时感知、理解并操作电脑和移动设备环境。重点不是聊天界面，而是权限、来源、工具协议、审计和操作后验证。

### 当前 Mobile Intelligence 状态

当前方向是 `Emulator → Frame → UI Tree → Element Refs → MobileObservation`。真实 M1.5 Gate 没有检测到 Android SDK、ADB 与 Emulator，因此结果是 3 FAIL、7 BLOCKED，M2 没有开始。

### 人工发布提醒

请在发布前人工检查英文措辞、选择真实且安全的截图，并确认 GitHub Issue 与仓库链接。Codex 不会自动发布 X。
