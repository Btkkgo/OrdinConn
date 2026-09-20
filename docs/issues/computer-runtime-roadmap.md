# English

## Goal

Define a staged, permission-aware Computer Runtime roadmap grounded in the existing OrdinConn contracts and product safety rules.

## Scope

Research operating-system perception, explicit permission contracts, post-action verification, local/remote boundaries, and the provenance path into Connector Registry and Evidence. This Issue does not implement the runtime.

## Current State

The repository contains bounded computer-use interfaces and a local-first perception direction. Broad production desktop operation is not implemented.

## Research Questions

- Which operating-system events and Accessibility APIs should form the structured perception layer?
- What explicit Work Session and Application Allowlist contracts are required?
- How should action receipts and post-action verification be represented across platforms?
- Which capabilities remain local, and which minimized observations may reach a model?
- How should Computer Runtime observations enter Connector Registry and Evidence without bypassing provenance?

## Constraints

- Structured perception before vision.
- API before GUI automation where appropriate.
- Event-driven perception before continuous capture.
- No password-field contents, private keys, recovery phrases, or unrestricted background recording.
- All model access through Model Gateway and all source intake through Connector Registry.

## Deliverable

An evidence-backed staged roadmap and ADR updates. This research Issue does not itself authorize implementation.

## Acceptance Criteria

- Document the staged capability boundaries and required permissions.
- Define action receipts and post-action verification across supported platforms.
- Preserve local-first privacy, Model Gateway, Connector Registry, Evidence provenance, and approval boundaries.
- Update the relevant ADRs with evidence-backed decisions.

## Result

Not completed. The Issue remains open with `status:needs-validation`; no broad production desktop operation is claimed.

## Validation

No final roadmap acceptance has been recorded yet. Existing bounded interfaces and design documents are context, not completion evidence.

# 中文

## 目标

基于现有 OrdinConn Contract 与产品安全规则，定义分阶段、感知权限的 Computer Runtime Roadmap。

## 范围

研究 Operating-system Perception、显式 Permission Contract、Post-action Verification、Local/Remote Boundary，以及进入 Connector Registry 与 Evidence 的 Provenance Path。本 Issue 不实现 Runtime。

## 当前状态

仓库包含 Bounded Computer-use Interface 与 Local-first Perception Direction。广泛的 Production Desktop Operation 尚未实现。

## 研究问题

- 哪些 Operating-system Event 与 Accessibility API 应组成 Structured Perception Layer？
- 需要什么样的显式 Work Session 与 Application Allowlist Contract？
- Action Receipt 与 Post-action Verification 应如何跨平台表示？
- 哪些 Capability 必须保留在本地，哪些最小化 Observation 可以发送给 Model？
- Computer Runtime Observation 如何在不绕过 Provenance 的前提下进入 Connector Registry 与 Evidence？

## 约束

- Structured Perception 优先于 Vision。
- 在适当情况下，API 优先于 GUI Automation。
- Event-driven Perception 优先于 Continuous Capture。
- 不采集 Password-field Content、Private Key、Recovery Phrase，也不进行 Unrestricted Background Recording。
- 所有 Model Access 通过 Model Gateway，所有 Source Intake 通过 Connector Registry。

## 交付物

一份以 Evidence 为依据的分阶段 Roadmap 与 ADR 更新。这个 Research Issue 本身不授权 Implementation。

## 验收标准

- 记录分阶段 Capability Boundary 与所需 Permission。
- 定义 Supported Platform 间的 Action Receipt 与 Post-action Verification。
- 保留 Local-first Privacy、Model Gateway、Connector Registry、Evidence Provenance 与 Approval Boundary。
- 用 Evidence-backed Decision 更新相关 ADR。

## 结果

尚未完成。Issue 保持 Open 且标记 `status:needs-validation`；没有声称已具备广泛 Production Desktop Operation。

## 验证

尚未记录最终 Roadmap Acceptance。现有 Bounded Interface 与 Design Document 只是 Context，不是 Completion Evidence。
