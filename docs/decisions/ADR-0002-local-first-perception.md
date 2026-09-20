# ADR-0002: Local-first perception

- Status: Accepted
- Date: 2026-09-20

## English

## Context

Computer and mobile perception can expose private application state. Sending unrestricted captures to remote services would weaken user control and make provenance harder to audit.

## Decision

Perception is local-first where practical. Structured system and Accessibility state is preferred, captures are bounded, application access is allowlisted, and sensitive nodes are redacted before observations leave the device boundary.

## Consequences

Platform adapters remain local and permission-aware. Remote inference may consume a minimized observation, but raw private state is not treated as a general-purpose data source.

## 中文

### 背景

Computer 与 Mobile Perception 可能暴露私有 Application State。把不受限的 Capture 发送到 Remote Service 会削弱用户控制，也让 Provenance 更难审计。

### 决策

在可行时采用 Local-first Perception。优先使用 Structured System 与 Accessibility State；Capture 必须有界；Application Access 使用 Allowlist；Sensitive Node 在 Observation 离开 Device Boundary 前脱敏。

### 影响

Platform Adapter 保持本地并感知 Permission。Remote Inference 可以使用最小化 Observation，但原始私有状态不会被当成通用 Data Source。
