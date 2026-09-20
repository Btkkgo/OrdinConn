# ADR-0003: No continuous screen recording by default

- Status: Accepted
- Date: 2026-09-20

## English

## Context

Continuous recording increases privacy exposure, storage volume, processing cost, and the chance of collecting unrelated content.

## Decision

OrdinConn prefers event-driven state, Accessibility/UI trees, and explicit bounded captures. Continuous screen recording is not a default perception mode.

## Consequences

Runtime work must define capture triggers, scope, retention, and user-visible permission boundaries. A future exceptional recording mode would require a separate explicit decision and acceptance gate.

## 中文

### 背景

Continuous Recording 会增加 Privacy Exposure、Storage Volume、Processing Cost，以及采集无关内容的概率。

### 决策

OrdinConn 优先使用 Event-driven State、Accessibility/UI Tree 与显式 Bounded Capture。Continuous Screen Recording 不是默认 Perception Mode。

### 影响

Runtime 工作必须定义 Capture Trigger、Scope、Retention 和用户可见的 Permission Boundary。未来若增加例外 Recording Mode，必须单独做出明确 Decision 并通过 Acceptance Gate。
