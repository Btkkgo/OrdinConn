# ADR-0004: GitHub Issues as the development ledger

- Status: Accepted
- Date: 2026-09-20

## English

## Context

Long-running agent development needs a public record that connects scope, implementation, failed attempts, validation, and remaining work without publishing private conversations.

## Decision

Meaningful engineering work is issue-first. The Issue defines scope and acceptance; DevLogs preserve sanitized execution facts; commits use `Refs #N` until the complete acceptance criteria justify `Fixes #N`. X is a manually curated distribution channel, not an engineering source of truth.

## Consequences

Future sessions must locate or create the relevant Issue before substantive work, update it after execution, and keep public claims no more advanced than code and real validation.

## 中文

### 背景

长期 Agent Development 需要一份公开记录，在不发布私密对话的前提下连接 Scope、Implementation、Failed Attempt、Validation 与 Remaining Work。

### 决策

有意义的工程工作采用 Issue-first。Issue 定义 Scope 与 Acceptance；DevLog 保留脱敏后的执行事实；在完整 Acceptance Criteria 满足前 Commit 使用 `Refs #N`，完全满足后才可使用 `Fixes #N`。X 是人工编辑的传播渠道，不是工程事实来源。

### 影响

未来 Session 必须在实质工作前定位或创建相关 Issue，在执行后更新它，并确保公开声明不超过 Code 与 Real Validation 所证明的状态。
