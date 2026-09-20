# ADR-0004: GitHub Issues as the development ledger

- Status: Accepted
- Date: 2026-09-20

## Context

Long-running agent development needs a public record that connects scope, implementation, failed attempts, validation, and remaining work without publishing private conversations.

## Decision

Meaningful engineering work is issue-first. The Issue defines scope and acceptance; DevLogs preserve sanitized execution facts; commits use `Refs #N` until the complete acceptance criteria justify `Fixes #N`. X is a manually curated distribution channel, not an engineering source of truth.

## Consequences

Future sessions must locate or create the relevant Issue before substantive work, update it after execution, and keep public claims no more advanced than code and real validation.
