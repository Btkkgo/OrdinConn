# Core Intelligence Implementation Plan

> **For Codex:** Execute this plan task-by-task using test-driven development and verify the complete workspace before claiming completion.

**Goal:** Deliver the V0.1 public-data collector, normalization, evidence, strategy, and signal-publication pipeline while leaving the React UI unchanged.

**Architecture:** Add transport-agnostic `collector-runtime` and `strategy-core` crates, extend `evidence-core` and `signal-core` with deterministic provenance metadata, then orchestrate and persist the pipeline in `ordinconn-app`. Network behavior is isolated behind collector interfaces so tests remain offline and deterministic.

**Tech Stack:** Rust 2024, Tokio, Reqwest, tokio-tungstenite, feed-rs, scraper, Serde, SQLx/SQLite, existing Tauri host.

---

### Task 1: Evidence and signal contracts

**Files:** Modify `crates/evidence-core/src/lib.rs`, `crates/signal-core/src/lib.rs` and their Cargo manifests.

1. Add failing tests for reliability tiers, freshness, clusters, contradictions, strategy metadata, and stale-confidence caps.
2. Implement deterministic Evidence assessment and backward-compatible SignalCandidate/Signal strategy metadata.
3. Run focused crate tests.

### Task 2: Collector Runtime

**Files:** Create `crates/collector-runtime/` modules and tests; modify workspace dependencies.

1. Add failing tests for Source Registry validation, auth/compliance gates, canonicalization, all dedup keys, rate limiting, retry/backoff, feed parsing, HTML extraction, REST normalization, WebSocket frames, health, and schema drift.
2. Implement SourceDefinition, Collector trait, registry, records/observations, public-data policy, rate limiter, retry policy, parsers, and collector adapters.
3. Add Binance, Federal Reserve, NVIDIA, and mempool.space source definitions.
4. Run focused crate tests without internet.

### Task 3: Strategy Engine

**Files:** Create `crates/strategy-core/` modules and tests.

1. Add failing tests for rolling windows, instrument/entity aliases, Traditional A/B/C, Crypto A/B/C, stale inputs, missing values, contradictions, and insufficient evidence.
2. Implement versioned StrategyDefinition/Result, centralized parameter config, typed observations, deterministic formulas, and reason codes.
3. Run focused crate tests.

### Task 4: Persistence and orchestration

**Files:** Add `crates/ordinconn-app/migrations/0003_core_intelligence.sql`; modify `crates/ordinconn-app/src/{db,services,lib}.rs` and tests.

1. Add failing migration/repository tests for all required tables and uniqueness/foreign-key behavior.
2. Implement repositories and a one-cycle orchestrator that persists each pipeline boundary and audit event atomically.
3. Ensure candidates can remain rejected/insufficient and only Evidence-gated candidates publish.
4. Register runtime initialization without blocking Tauri commands.

### Task 5: Real paths and manual smoke

**Files:** Add `crates/ordinconn-app/examples/manual_live_smoke.rs` and fixtures/tests.

1. Implement real collection paths for Binance, Federal Reserve, NVIDIA, and mempool.space.
2. Run automated fixture tests first.
3. Run manual live smoke and save a bounded redacted JSON report; report unavailable sources truthfully.

### Task 6: Documentation and final verification

**Files:** Add collector/source/strategy/ABC/evidence/retention docs; update `docs/PRODUCT_BASELINE.md` and `docs/ARCHITECTURE.md`.

1. Document contracts, source policies, strategies, retention, and failure modes.
2. Run formatting, Rust tests/clippy, TypeScript tests/typecheck/build, migration tests, and diff checks.
3. Audit that no React UI file changed after the frozen baseline.
4. Commit the completed Core Intelligence phase and report the required 22-item execution summary. Do not push.

