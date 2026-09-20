# ADR-0001: Model-agnostic runtime

- Status: Accepted
- Date: 2026-09-20

## English

## Context

Agent workflows need stable tool, permission, provenance, and verification contracts even when the selected model changes.

## Decision

All model providers enter through Model Gateway. Core domain crates cannot depend on a named provider response type. Model output remains inference and cannot satisfy an Evidence requirement by itself.

## Consequences

Provider adapters must normalize capabilities and errors. The runtime can replace providers without rewriting domain logic, at the cost of maintaining explicit gateway contracts.

## 中文

### 背景

即使所选模型发生变化，Agent Workflow 仍需要稳定的 Tool、Permission、Provenance 与 Verification Contract。

### 决策

所有 Model Provider 都通过 Model Gateway。Core Domain Crate 不得依赖特定 Provider Response Type。Model Output 始终属于 Inference，不能单独满足 Evidence Requirement。

### 影响

Provider Adapter 必须统一 Capability 与 Error。Runtime 可以在不重写 Domain Logic 的情况下替换 Provider，代价是必须维护显式 Gateway Contract。
