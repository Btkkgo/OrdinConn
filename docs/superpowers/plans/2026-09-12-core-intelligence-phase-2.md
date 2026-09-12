# Core Intelligence Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add continuous scheduling, bounded rolling baselines, public Binance USDⓈ-M Futures data, persistent Evidence clusters, and readiness-aware real signal publication.

**Architecture:** `collector-runtime` owns scheduler and Futures collection; `strategy-core` owns rolling history/baselines/readiness strategies; `evidence-core` owns durable clustering semantics; `ordinconn-app` owns SQLite repositories, lifecycle, diagnostics, and audit events. Tauri starts and gracefully shuts down the application-owned runtime without changing React.

**Tech Stack:** Rust 2024, Tokio, Reqwest, tokio-tungstenite, Serde, SQLx/SQLite, existing Tauri host.

**Spec:** `docs/superpowers/specs/2026-09-12-core-intelligence-phase-2-design.md`

## Global Constraints

- React/UI zero modification.
- Only BTCUSDT, ETHUSDT, and SOLUSDT public Spot/Futures market data.
- No API keys, user/account/order endpoints, real trading, wallets, brokers, deposits, or withdrawals.
- Source event timestamps drive market calculations; all stored times are UTC.
- A not-ready Strategy Run creates no ordinary Candidate.
- A real Signal cannot contain mock Evidence.
- Tests never depend on public internet; live and soak runs are explicit opt-ins.

---

### Task 1: Scheduler lifecycle and diagnostics

**Files:** Create `crates/collector-runtime/src/scheduler.rs`; modify `crates/collector-runtime/src/lib.rs` and workspace Tokio features.

**Interfaces:** Produce `ScheduledJob::run_once`, `CollectorScheduler::{register,start,pause,resume,shutdown,snapshot}`, `SchedulerState`, `SchedulerEvent`, and `SchedulerDiagnostics`.

- [ ] Write tests proving cadence, per-source single-flight, global concurrency, pause/resume, failure backoff, and awaited shutdown using paused Tokio time.
- [ ] Run `cargo test -p collector-runtime scheduler` and confirm failures because scheduler types do not exist.
- [ ] Implement owned task handles, watch-based lifecycle, semaphore permits, persisted-state restore inputs, and deterministic bounded jitter.
- [ ] Re-run focused tests and refactor only after green.

### Task 2: Rolling History and baseline readiness

**Files:** Create `crates/strategy-core/src/rolling.rs`; modify `crates/strategy-core/src/lib.rs`.

**Interfaces:** Produce `RollingHistoryEngine::ingest`, `metrics`, `baseline`, `bucket`, `restore_buckets`, `HistoryWindow`, `MetricSample`, `AggregateBucket`, `BaselineSnapshot`, `Readiness`.

- [ ] Write literal-result tests for all required metrics, OHLC buckets, warm-up, stale input, five-second out-of-order tolerance, discard audit result, bounded memory, and bucket restore.
- [ ] Run the focused tests and verify the expected missing-symbol failures.
- [ ] Implement bounded `VecDeque` series, sorted tolerant insert, deterministic metric formulas, bucket aggregation, and restore.
- [ ] Re-run focused tests and ensure each behavior mutation is caught.

### Task 3: Binance USDⓈ-M Futures adapter

**Files:** Create `crates/collector-runtime/src/binance_futures.rs`; modify source registry and collector tests.

**Interfaces:** Produce `BinanceFuturesClient::fetch_symbol`, `normalize_*`, `FuturesObservation`, `MarketType`, and Spot/PERP canonical instrument resolution.

- [ ] Add complete official-schema fixtures for premium index, open interest, aggregate trades, book ticker, limited depth, and 24h ticker for all three allowed symbols.
- [ ] Verify tests fail before the adapter exists.
- [ ] Implement public `fapi.binance.com` requests, bounded payloads, event-time parsing, finite-depth aggregation, spread/imbalance, volume, and strict symbol allowlisting.
- [ ] Add local WebSocket lifecycle tests for subscribe, data, stale reconnect, resubscribe, and shutdown; implement with bounded exponential backoff plus deterministic jitter.
- [ ] Re-run collector tests with no internet.

### Task 4: Readiness-aware derivatives strategies

**Files:** Create `crates/strategy-core/src/derivatives.rs`; modify `StrategyResult`, `StrategyEngine`, `signal-core`, and tests.

**Interfaces:** Produce funding, OI, price/OI, volume, imbalance, and spread evaluations accepting `MarketContext`/`BaselineSnapshot`; extend Signal provenance with real origin, baseline window, trigger metrics, source IDs, and evidence IDs.

- [ ] Write tests for readiness states, funding history/z-score, OI changes, four divergence patterns, volume baseline, finite-depth imbalance, spread baseline, and stale/schema failures.
- [ ] Add a failing test that a REAL Candidate containing MOCK Evidence cannot publish.
- [ ] Implement readiness-first evaluation and remove Phase 1's missing-history Candidate creation path.
- [ ] Re-run strategy/signal tests and retain backward-compatible Serde defaults.

### Task 5: Persistent buckets, scheduler state, clusters, and timelines

**Files:** Create migration `0004_continuous_intelligence.sql`; create `crates/ordinconn-app/src/continuous_intelligence.rs`; modify application tests.

**Interfaces:** Persist `scheduler_state`, `market_metric_buckets`, `evidence_cluster_members`, strategy audit snapshots, demand timelines, and cluster summaries; restore baselines on startup.

- [ ] Write migration/repository tests for all tables, cluster member roles, syndication non-counting, independent confirmation, demand persistence, bucket restore, and significant audit events.
- [ ] Run database tests and confirm missing-table failures.
- [ ] Implement repositories and atomic state/audit transactions without storing raw ticks.
- [ ] Re-run application tests and distinguish strategy readiness failures from Evidence Gate failures.

### Task 6: Embedded runtime ownership

**Files:** Modify `crates/ordinconn-app/src/services.rs`, `crates/ordinconn-app/src/core_intelligence.rs`, and `apps/desktop/src-tauri/src/lib.rs`.

**Interfaces:** Produce `AppRuntime::{start_continuous_intelligence,pause_continuous_intelligence,resume_continuous_intelligence,shutdown}` and runtime diagnostics.

- [ ] Write a lifecycle test proving shutdown awaits scheduler tasks and no poll occurs afterward.
- [ ] Verify the test fails against the Phase 1 detached one-shot startup.
- [ ] Store scheduler ownership in `AppRuntime`, start enabled jobs after initialization, restore persisted state/buckets, and flush aggregate state before database close.
- [ ] Replace the detached Tauri one-shot call and run app/Tauri tests.

### Task 7: Live smoke, soak, documentation, and release gate

**Files:** Extend `manual_live_smoke`; create `manual_soak_test`; update Collector, Source, Strategy, Traditional/Crypto ABC, Evidence, retention, Architecture, and Product Baseline docs.

**Interfaces:** Live output reports real Futures fields and real Strategy Runs/Candidates/Signals; soak output reports duration, bounded-memory utilization, DB growth, duplicates, reconnects, drift, and warm-up.

- [ ] Add output-contract tests around diagnostics serialization; keep both real-network examples opt-in.
- [ ] Run full Rust/TypeScript test, lint, typecheck, build, diff, and React zero-modification checks.
- [ ] Run `ORDINCONN_LIVE_SMOKE=1 cargo run -p ordinconn-app --example manual_live_smoke`.
- [ ] Run `ORDINCONN_SOAK_TEST=1 ORDINCONN_SOAK_SECONDS=<controlled>` for a bounded verification interval.
- [ ] Commit as `feat: add continuous intelligence history and derivatives`; do not push.

