# ADR-0002: Local-first perception

- Status: Accepted
- Date: 2026-09-20

## Context

Computer and mobile perception can expose private application state. Sending unrestricted captures to remote services would weaken user control and make provenance harder to audit.

## Decision

Perception is local-first where practical. Structured system and Accessibility state is preferred, captures are bounded, application access is allowlisted, and sensitive nodes are redacted before observations leave the device boundary.

## Consequences

Platform adapters remain local and permission-aware. Remote inference may consume a minimized observation, but raw private state is not treated as a general-purpose data source.
