# ADR-0001: Model-agnostic runtime

- Status: Accepted
- Date: 2026-09-20

## Context

Agent workflows need stable tool, permission, provenance, and verification contracts even when the selected model changes.

## Decision

All model providers enter through Model Gateway. Core domain crates cannot depend on a named provider response type. Model output remains inference and cannot satisfy an Evidence requirement by itself.

## Consequences

Provider adapters must normalize capabilities and errors. The runtime can replace providers without rewriting domain logic, at the cost of maintaining explicit gateway contracts.
