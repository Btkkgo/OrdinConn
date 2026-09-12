# OrdinConn V0.1 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a runnable local-first OrdinConn V0.1 desktop application with evidence-backed Traditional Finance and Crypto signals, a model-agnostic agent runtime, explicit approvals, and an end-to-end paper-execution demo.

**Architecture:** A React client communicates only through stable typed Tauri commands and events with a single-process Rust host. Transport-independent Rust crates implement domain behavior; `ordinconn-app` owns SQLite, application services, lifecycle, recovery, and the event bus.

**Tech Stack:** Rust 2024, Tokio, SQLx/SQLite, Tauri v2, React, TypeScript, Vite, Vitest, npm workspaces.

**Spec:** `docs/superpowers/specs/2026-09-12-ordinconn-v0-1-foundation-design.md`

## Global Constraints

- English is the default and only fully enabled V0.1 locale; formal React copy uses locale keys.
- Core crates remain independent of Tauri, React, IPC, HTTP servers, and desktop windows.
- No real broker/exchange execution, funds, transfers, signing, private keys, seed phrases, or password capture.
- Signals require non-inference Evidence and expose contradictions.
- Paper execution requires a valid single-use, object-bound, version-bound, time-limited capability.
- SQLite stores current relational state; `runtime_events` is append-only; streaming deltas stay in memory.
- Chat Completions raw structures exist only in `OpenAICompatibleChatAdapter`.
- No API key, token, or secret is stored in Git, SQLite, audit events, logs, or browser storage.
- One final local commit is created only after the full test and build gate passes; nothing is pushed.

---

### Task 1: Workspace, product baseline, and transport boundaries

**Files:**
- Create: `Cargo.toml`, `package.json`, `.gitignore`, `README.md`, `AGENTS.md`
- Create: `docs/PRODUCT_BASELINE.md`, `docs/ARCHITECTURE.md`, `docs/AGENT_RUNTIME.md`, `docs/SIGNAL_ENGINE.md`, `docs/MODEL_GATEWAY.md`, `docs/SAFETY_AND_APPROVAL.md`
- Create: `apps/desktop/package.json`, `apps/desktop/tsconfig.json`, `apps/desktop/vite.config.ts`, `apps/desktop/index.html`
- Create: `packages/contracts/package.json`, `packages/contracts/src/index.ts`, `packages/contracts/src/index.test.ts`

**Interfaces:**
- Produces: stable `CommandResult<T>`, `RuntimeEventEnvelope`, market/signal/approval DTO enums, and locale identifiers.
- Consumes: no earlier tasks.

- [ ] Write contract tests proving allowed enum values, event envelopes, and error DTOs reject unknown states.
- [ ] Run `npm test --workspace @ordinconn/contracts` and confirm failure because contracts do not exist.
- [ ] Add workspace configuration, product documentation, AGENTS rules, and minimal contract types.
- [ ] Run the contract tests and `npm run typecheck --workspaces --if-present` until green.
- [ ] Check that README and product docs contain no legacy product description.

### Task 2: Domain crates and Evidence-to-Signal publication

**Files:**
- Create: `crates/market-core/{Cargo.toml,src/lib.rs}`
- Create: `crates/evidence-core/{Cargo.toml,src/lib.rs}`
- Create: `crates/signal-core/{Cargo.toml,src/lib.rs}`
- Create: `crates/traditional-finance/{Cargo.toml,src/lib.rs}`
- Create: `crates/crypto-core/{Cargo.toml,src/lib.rs}`
- Create: `crates/connector-runtime/{Cargo.toml,src/lib.rs}`
- Test: unit modules colocated in each crate.

**Interfaces:**
- Produces: `Market`, `AssetRef`, `Evidence`, `EvidenceRelation`, `SignalCandidate`, `Signal`, `SignalPublisher`, `Connector`, and `MockConnectorSet`.
- Consumes: shared ids and time types from `market-core`.

- [ ] Write failing tests for no-evidence rejection, inference-only rejection, contradiction scoring, three published signals per ABC lane, and evidence traceability.
- [ ] Run focused Cargo tests and verify each fails for the missing behavior.
- [ ] Implement the smallest domain types, deterministic evidence-quality formula, publisher, and mock connectors.
- [ ] Run focused tests, then `cargo test --workspace`, and refactor only after green.
- [ ] Document the scoring formula and candidate/publication state machine in `docs/SIGNAL_ENGINE.md`.

### Task 3: Model Gateway, Tool Runtime, and Agent Runtime

**Files:**
- Create: `crates/model-gateway/{Cargo.toml,src/lib.rs,src/openai_compatible.rs}`
- Create: `crates/tool-runtime/{Cargo.toml,src/lib.rs}`
- Create: `crates/computer-use/{Cargo.toml,src/lib.rs}`
- Create: `crates/agent-runtime/{Cargo.toml,src/lib.rs}`
- Test: adapter fixtures and unit modules in each crate.

**Interfaces:**
- Produces: `UnifiedModelRequest`, `ModelEvent`, `ProviderCapabilities`, `ModelProviderAdapter`, `OpenAICompatibleChatAdapter`, `ToolDefinition`, `ToolAvailability`, `AgentThread`, `AgentTurn`, `AgentItem`, `PageContext`, and cancellation-aware `AgentRuntime`.
- Consumes: signal/evidence context DTOs from Tasks 1 and 2.

- [ ] Write failing tests for non-stream response normalization, SSE deltas, tool-call assembly, unsupported tool-calling downgrade, normalized errors, page-context injection, and cancellation.
- [ ] Run focused tests and verify expected failures before production implementation.
- [ ] Implement provider-neutral protocols, the Chat Completions adapter, capability checks, tool registry, truthful computer-use availability, and runtime state machine.
- [ ] Run focused tests and full workspace tests until green.
- [ ] Document adapter containment and ModelEvent mappings in `docs/MODEL_GATEWAY.md` and runtime lifecycle in `docs/AGENT_RUNTIME.md`.

### Task 4: Canonical proposal, Approval Capability, and Paper Execution

**Files:**
- Create: `crates/approval-engine/{Cargo.toml,src/lib.rs}`
- Create: `crates/execution-core/{Cargo.toml,src/lib.rs}`
- Test: unit modules and concurrent consumption integration tests.

**Interfaces:**
- Produces: `CanonicalTradeProposalV1`, `TradeProposal`, `ApprovalRequest`, `ApprovalCapability`, `ApprovalTokenIssuer`, `PaperExecutionAdapter`, and typed failures.
- Consumes: Published Signal identity from `signal-core` and stable enums from `market-core`.

- [ ] Write failing tests for the approved exact proposal, double use, expiry, invalid states, every material field mutation, unknown hash version, restart invalidation, and concurrent double execution.
- [ ] Run the focused tests and verify failures arise from missing security behavior.
- [ ] Implement deterministic canonical bytes, SHA-256 hashing, opaque random tokens, digest-only records, fail-closed validation, and paper adapter behavior.
- [ ] Run focused tests and `cargo test --workspace` until green.
- [ ] Record invariants and the atomic database boundary in `docs/SAFETY_AND_APPROVAL.md`.

### Task 5: SQLite application services, audit events, and recovery

**Files:**
- Create: `crates/ordinconn-app/{Cargo.toml,src/lib.rs,src/db.rs,src/events.rs,src/services.rs}`
- Create: `crates/ordinconn-app/migrations/0001_v0_1_foundation.sql`
- Test: `crates/ordinconn-app/tests/database.rs`, `crates/ordinconn-app/tests/closed_loop.rs`

**Interfaces:**
- Produces: `AppRuntime`, `AppSnapshot`, transactional repositories, `RuntimeEventBus`, `start_agent_turn`, `create_report`, `create_trade_proposal`, `request_approval`, `approve_and_execute_paper`, and startup recovery.
- Consumes: all domain/runtime interfaces from Tasks 2-4.

- [ ] Write failing integration tests for migration shape, WAL/foreign keys, 18 published signals, state-plus-audit atomicity, interrupted-task recovery, approval history, and full BTC paper execution.
- [ ] Run the integration tests against temporary SQLite files and verify expected failures.
- [ ] Implement migration, repositories, application services, in-memory bus, startup seeding, recovery, and atomic approval consumption.
- [ ] Run database and closed-loop tests, then the full Rust suite, until green.
- [ ] Inspect test databases to verify no token or provider secret is persisted.

### Task 6: Tauri host and secure provider configuration

**Files:**
- Create: `apps/desktop/src-tauri/{Cargo.toml,build.rs,tauri.conf.json,capabilities/default.json}`
- Create: `apps/desktop/src-tauri/src/{main.rs,lib.rs,state.rs,commands.rs,events.rs,credential_store.rs}`
- Test: Rust unit tests in the adapter modules.

**Interfaces:**
- Produces: typed snapshot/query/task/model/approval commands and one stable runtime-event channel.
- Consumes: `AppRuntime` and IPC DTO conversion boundaries.

- [ ] Write failing tests proving DTO conversion, redacted errors, explicit provider capabilities, ephemeral key handling, and commands that return before long tasks finish.
- [ ] Run adapter tests and confirm expected failures.
- [ ] Implement Tauri state/lifecycle, command registration, event forwarding, OS credential abstraction, graceful shutdown, and no-listener-port configuration.
- [ ] Run `cargo test --workspace` and `cargo check -p ordinconn-desktop` until green.
- [ ] Inspect generated metadata and logs for secret leakage.

### Task 7: English-first React shell and complete V0.1 flow

**Files:**
- Create: `apps/desktop/src/{main.tsx,App.tsx,styles.css}`
- Create: `apps/desktop/src/i18n/{index.ts,en.ts,zh-CN.ts,index.test.ts}`
- Create: `apps/desktop/src/runtime/{client.ts,events.ts,state.ts,state.test.ts}`
- Create: `apps/desktop/src/components/{AppShell.tsx,TopContextBar.tsx,Navigation.tsx,AgentDock.tsx,SignalCard.tsx,SignalDetail.tsx}`
- Create: `apps/desktop/src/pages/{OverviewPage.tsx,TraditionalFinancePage.tsx,CryptoPage.tsx,SignalsPage.tsx,AgentPage.tsx,AutomationsPage.tsx,ModelsPage.tsx,DataSourcesPage.tsx,ApprovalsPage.tsx,SettingsPage.tsx}`

**Interfaces:**
- Produces: approved purple, black, and yellow desktop shell with the user-provided mark, all required navigation, ABC tabs, signal/evidence detail, contextual Agent Dock, model form, connector states, approval UI, and paper result.
- Consumes: only typed IPC contracts and runtime event envelopes.

- [ ] Write failing tests for English key completeness, Chinese fallback, no literal formal copy, page-context changes, event reduction, and the BTC report/proposal/approval/execution interaction model.
- [ ] Run Vitest and verify failures before component implementation.
- [ ] Implement the locale layer, typed client, state reducer, design tokens, shell, pages, dock, and complete demo flow.
- [ ] Run UI tests, TypeScript typecheck, and production Vite build until green.
- [ ] Verify unavailable tools/providers and Mock Model are labeled truthfully.

### Task 8: Full verification, visible smoke test, and local commit

**Files:**
- Modify only files needed to fix failures with a reproducing test first.

**Interfaces:**
- Produces: verified V0.1 repository and final commit.
- Consumes: all prior tasks.

- [ ] Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
- [ ] Run `npm test --workspaces --if-present`, `npm run typecheck --workspaces --if-present`, and `npm run build --workspace @ordinconn/desktop`.
- [ ] Run `cargo build -p ordinconn-desktop` and inspect the produced desktop artifact.
- [ ] Launch the desktop development build and manually exercise the required BTC closed loop; record any environment-limited checks as not verified.
- [ ] Run `git diff --check`, secret-pattern scans, direct-UI-copy checks, repository-boundary checks, and `git status --short`.
- [ ] Stage only the new OrdinConn repository and create `feat: rebuild OrdinConn financial agent foundation` after every gate passes.
- [ ] Report the commit hash, exact tests/builds, recoverable reset location, implemented scope, and deferred work without pushing.
