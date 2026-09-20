# ADR-0003: No continuous screen recording by default

- Status: Accepted
- Date: 2026-09-20

## Context

Continuous recording increases privacy exposure, storage volume, processing cost, and the chance of collecting unrelated content.

## Decision

OrdinConn prefers event-driven state, Accessibility/UI trees, and explicit bounded captures. Continuous screen recording is not a default perception mode.

## Consequences

Runtime work must define capture triggers, scope, retention, and user-visible permission boundaries. A future exceptional recording mode would require a separate explicit decision and acceptance gate.
