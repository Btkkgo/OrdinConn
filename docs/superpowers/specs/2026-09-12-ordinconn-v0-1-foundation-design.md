# OrdinConn V0.1 Foundation Design

## Product boundary

OrdinConn V0.1 is a local-first desktop financial intelligence agent. It covers Traditional Finance and Crypto through an evidence-backed ABC signal workflow:

`Data -> Evidence -> Signal Candidate -> Published Signal -> Agent -> Report -> Trade Proposal -> Approval -> Paper Execution`

This release does not connect to a real broker or exchange execution account, handle private keys or seed phrases, sign wallet transactions, transfer assets, or trade real funds. Those capabilities are explicitly unavailable rather than simulated as connected.

All prior OrdinConn product baselines are obsolete. This document and `docs/PRODUCT_BASELINE.md` define the new system.

## Chosen approach

V0.1 uses a Tauri v2 single-process embedded runtime. React is a presentation client. Rust owns application state, SQLite, model access, connectors, tools, approvals, signals, reports, proposals, and paper execution. Typed Tauri commands start work or query snapshots; a stable event envelope carries long-running task updates to the UI.

Core crates are transport-agnostic. They do not depend on Tauri, React, IPC, HTTP hosting, or desktop windows. The Tauri crate is an adapter and lifecycle host around application services.

The implementation deliberately avoids a daemon, sidecar, localhost server, WebSocket server, distributed bus, CQRS, full event sourcing, and generalized replay engine.

## Repository structure

- `apps/desktop`: React, TypeScript, Vite, Tauri v2 application and IPC adapter.
- `crates/market-core`: shared market identifiers and value types.
- `crates/evidence-core`: evidence, provenance, factual levels, and quality scoring.
- `crates/signal-core`: candidates, validation, signal publication, and ABC categories.
- `crates/model-gateway`: unified model protocol and provider adapters.
- `crates/tool-runtime`: tool registry, availability, risk, and invocation contracts.
- `crates/approval-engine`: approval state machine and one-time capability issuance.
- `crates/execution-core`: trade proposals, canonical hashing, and paper execution.
- `crates/agent-runtime`: thread, turn, item, context, cancellation, and normalized events.
- `crates/connector-runtime`: connector registry and deterministic mock evidence pipeline.
- `crates/computer-use`: transport-independent computer-use traits and bounded local adapters.
- `crates/traditional-finance`: Traditional Finance ABC fixtures and rules.
- `crates/crypto-core`: Crypto ABC fixtures, watchlists, and rules.
- `crates/ordinconn-app`: application services, SQLite repositories, migrations, event bus, and recovery.
- `packages/contracts`: stable TypeScript IPC DTOs, event envelopes, and UI-facing enums.
- `docs`: product baseline, architecture, runtime, model, signal, and safety documentation.

## Runtime protocol

The Agent Runtime owns `AgentThread`, `AgentTurn`, and completed `AgentItem` records. A thread has at most one active turn. Starting a long-running turn returns a thread and task identifier immediately. Work runs on Tokio tasks and emits normalized events. Cancellation tokens interrupt active tasks when the user stops a turn or the app shuts down.

The public event envelope contains an event id, event type, aggregate references, timestamp, and a tagged payload. Public event families are `AgentEvent`, `ModelEvent`, `SignalEvent`, `ApprovalEvent`, `ExecutionEvent`, and `SystemEvent`. Module-private types never cross IPC directly.

Streaming deltas remain in memory. Completed messages and tool outcomes become `agent_items`. On startup, unsafe unfinished tasks are marked `interrupted` and an append-only `runtime.task_interrupted` event is recorded. Side-effecting tools are never replayed automatically.

## Model gateway

The Agent Runtime sends a provider-neutral `UnifiedModelRequest` and receives `ModelEvent` values:

- `message_delta`
- `reasoning_delta`
- `tool_call_started`
- `tool_call_delta`
- `tool_call_completed`
- `usage`
- `completed`
- `error`

`OpenAICompatibleChatAdapter` is the only real V0.1 network adapter. It targets configurable `/v1/chat/completions` endpoints and supports non-streaming, streaming, and tool calling when explicitly enabled. Raw `messages`, `choices`, and `tool_calls` structures are confined to the adapter.

Provider capabilities are stored explicitly: `chatCompletions`, `responses`, `streaming`, `toolCalling`, `reasoning`, `vision`, `structuredOutput`, and `jsonMode`. Capabilities are never inferred from the provider name. Providers without tool calling degrade to text-only analysis and surface tools as unavailable. `OpenAIResponsesAdapter` and other vendor adapters are interface-only future extension points.

API keys are stored through an OS credential-store abstraction and are never written to SQLite, Git, events, logs, browser storage, or UI state. The database stores only a credential reference. A deterministic Mock Model keeps the demo fully usable without a key.

## Evidence and signals

Connectors create Evidence and Signal Candidates. A candidate becomes a Published Signal only after validation. Publication requires at least one linked Evidence record whose source type is not `MODEL_INFERENCE`.

Evidence records contain source, source type, market, asset, capture time, freshness, reliability, factual level, content, raw reference, confidence, and non-sensitive metadata. Signal-Evidence links declare `primary`, `supporting`, `contradicting`, or `context`.

The Signal Engine computes evidence quality from freshness, reliability, factual level, confidence, relation role, and contradictions. The model cannot set final evidence quality. Contradictions remain visible and reduce confidence or move a signal to `watch`. Insufficient candidates retain `insufficient_evidence` and cannot create formal reports, proposals, approvals, or executions.

The six V0.1 ABC lanes are:

- Traditional Finance: Trading, Event, Demand.
- Crypto: On-chain, Event, Exchange.

Deterministic mock connectors produce at least three candidates per lane. They pass through the same evidence validation and publication services as future real connectors.

## Approval and execution

A Published Signal may produce an Agent Report and a versioned Trade Proposal. A proposal is never an authorization.

`CanonicalTradeProposalV1` deterministically encodes market, instrument, action, order type, quantity, price, stop-loss, and take-profit. Decimal values use a stable textual normalization, optional values use an explicit representation, and the resulting bytes are hashed with SHA-256. The hash version is stored.

Approval uses an opaque cryptographically random capability token that is single-use, object-bound, version-bound, time-limited, and non-transferable. SQLite stores only its digest and binding fields. React submits approve, reject, or cancel decisions but never receives, signs, stores, or constructs the token.

Token verification, token consumption, and creation of a starting Execution Record happen atomically in one SQLite transaction. A token is consumed on the first valid execution attempt regardless of adapter success. Modified proposal fields, unknown hash versions, expiry, restart ambiguity, digest mismatch, database errors, and invalid approval states all fail closed.

V0.1 uses an ephemeral issuer. Outstanding capabilities are invalidated on restart. Paper Execution is the only adapter and requires the same valid capability path as future real execution. Retry requires a new approval request.

## Persistence

SQLite relational tables represent current state. `runtime_events` is append-only audit history. Live streaming uses an in-memory broadcast bus.

SQLite enables WAL, foreign keys, and a busy timeout. Migrations own the schema. Time is stored as UTC RFC 3339 text. Important state changes update business rows and append audit events in one transaction. Event payloads contain minimal structured audit context and no credentials, raw tokens, or unnecessary large text.

The data layer reserves a migration boundary for a future `outbox_events` table without implementing an outbox in V0.1.

## Tools and computer use

Tools declare id, name, description, JSON input schema, permission, risk level, and availability. Read tools, report creation, signal generation, and opening a URL can run automatically. Order preparation and every execution path require approval. Transfers, withdrawals, private-key access, seed-phrase access, and password-field capture are prohibited.

The computer-use priority is API, structured connector, browser DOM, accessibility, vision, then mouse and keyboard. V0.1 exposes bounded observe-current-window, screenshot, and open-URL adapters where supported. Click and type capabilities are registered as unavailable until a later Episode; the UI must not imply otherwise.

## Desktop experience

The shell uses left navigation, a center workspace, a top context bar, and a collapsible right Agent Dock. English remains the default locale, and English plus Simplified Chinese are fully selectable in Settings. Every formal UI string is referenced through locale keys, and both dictionaries maintain identical key coverage.

The final approved brand direction uses the black-and-yellow geometric mark from `Conor右1.0.jpg` and derives the application surfaces from its purple background. Deep purple remains the readable workspace base, black anchors navigation and Agent chrome, and yellow is reserved for selected states, approvals, primary actions, and high-value signal accents.

Navigation contains Overview, Traditional Finance, Crypto, Signals, Agent, Automations, Models, Data Sources, Approvals, and Settings. The final visual system uses the approved source mark and restrained purple, black, and yellow palette. It avoids casino styling, large gradients, and excessive motion.

The Agent Dock receives page, market, asset, signal, and evidence context automatically. It supports contextual questions, explanation, report generation, risk analysis, investigation prompts, watch creation, evidence viewing, and proposal generation. With no configured provider, Mock Model responses are clearly labeled.

The required demo path is executable end to end: select the BTC Exchange Signal, inspect evidence, ask why, create a report, create a paper proposal, request approval, approve, and receive a completed Paper Execution Record.

## Error handling

Domain errors are typed and mapped to stable IPC error DTOs. Provider errors normalize authentication, rate limit, timeout, unsupported capability, malformed response, network, and provider failures. UI surfaces explicit unavailable, pending, degraded, interrupted, and failed states.

No operation silently succeeds. Database transaction failures roll back both state and audit writes. Security-sensitive failures are redacted before logs or IPC. Provider testing never persists an API key unless the explicit save flow succeeds.

## Testing and acceptance

Rust unit and integration tests cover signal publication, evidence linkage, provider normalization, approval state transitions, canonical hashes, expiry, restart invalidation, atomic one-time consumption, concurrent execution, paper execution, migrations, recovery, and mock generation.

TypeScript tests cover locale coverage, IPC DTO parsing, UI reducers, context injection, and the closed-loop interaction model. Final acceptance includes Rust tests, TypeScript tests, typecheck, production UI build, Tauri build/check, database migration verification, `git diff --check`, and a visible desktop smoke test when the local host permits it.

## Licensing

OrdinConn adopts architecture ideas and vocabulary from the Apache-2.0 OpenAI Codex project but does not copy its UI or source implementation. If any Codex source is copied later, its license and attribution must be preserved. V0.1 is implemented independently against OrdinConn's own protocols.
