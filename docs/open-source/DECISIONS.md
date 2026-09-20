# Engineering Decisions

## Current decisions

### Evidence is not inference

Model output may explain or synthesize, but it cannot satisfy the factual Evidence gate. Published Signals retain their Evidence links and contradictions.

### Signal is not trade

Trade Proposals pass through explicit Approval. V0.1 exposes only Paper Execution, and the approval capability is exact-object-bound, version-bound, time-limited, and single-use.

### Models and sources use gateways

All providers pass through Model Gateway. All collection sources pass through Connector Registry. Domain runtimes never depend directly on a vendor response shape or ad hoc source implementation.

### Local-first single-process runtime

V0.1 uses a Tauri single-process embedded Rust runtime, SQLite current state, append-only audit events, and typed IPC. A daemon, distributed bus, full Event Sourcing, and remote host are not current architecture.

### Readiness and Evidence validation are separate

A strategy can be unable to run because input is missing, stale, malformed, or too young. That state is recorded without fabricating a Candidate.

### Mobile observation is its own boundary

`MobileObservation` is neither Evidence nor a Signal. M4 is the earliest planned phase for an explicit observation-to-Evidence promotion path.

### Real integration gates cannot be replaced by fixtures

Fixtures verify parsing and policy. A real-environment claim requires the real environment and the production path.

### Structured perception before vision

The long-term perception design prefers events and Accessibility/UI trees before bounded image capture and OCR/vision. This reduces cost and provides clearer privacy boundaries.

### Public summaries, not raw conversations

Public development records contain technical goals, inspected areas, changes, commands, results, risks, and lessons. They do not contain raw prompts, private chats, credentials, or machine identity.

## Historical decisions

- The initial V0.1 intentionally used one embedded runtime rather than a daemon or cloud service.
- The current navigation consolidates the product around Home, Warehouse, and Settings while retaining the underlying financial capabilities.
- Mobile verified navigation was deferred when the real M1.5 environment gate failed.
