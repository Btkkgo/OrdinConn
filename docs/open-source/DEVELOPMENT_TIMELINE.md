# Development Timeline

This timeline is derived from the current Git history and repository documents. It does not reconstruct private conversations.

## 2026-09-12 — V0.1 financial-agent foundation

**Goal:** Establish the local-first financial intelligence workflow.

**Changes:** Commit `4a39696` rebuilt the foundation; `ff8ad31` finalized dashboard readability and interactions.

**Problems:** Financial intelligence required a strict boundary between model inference, Evidence, Signals, approvals, and execution.

**Solution:** Separate transport-independent Rust domains, application services, typed Tauri IPC, and React presentation. Bind Paper Execution to an exact, single-use Approval Capability.

**Result:** The repository contains the end-to-end demo flow, SQLite current state, append-only audit events, localization, and the purple/black/yellow desktop shell.

**Tests:** Current tests continue to cover the evidence gate, approval consumption, recovery, IPC error redaction, and closed-loop Paper Execution.

**Decision:** No real-money execution in V0.1.

**Codex Notes:** The implementation plan divided the work into independently testable domain, persistence, host, and UI boundaries.

## 2026-09-12 — Core Intelligence collection

**Goal:** Replace demo-only intelligence inputs with a public-data collection and strategy pipeline.

**Changes:** `b44c74b` documented the architecture; `bdd6447` implemented collectors and orchestration; `d605e15` made feed schema drift fail closed; `2950e6b` defined the next continuous-intelligence phase.

**Problems:** Raw source formats drift, duplicate content can look like independent confirmation, and incomplete history can produce false confidence.

**Solution:** Normalize source records, fingerprint schemas, deduplicate observations, preserve evidence roles, and separate strategy readiness from Evidence validation.

**Result:** REST, WebSocket, feed, and HTML collection paths enter a shared registry and Evidence pipeline.

**Tests:** The current suite covers normalization, rate limiting, schema drift, feed parsing, HTML extraction, deduplication, and publication rejection.

**Decision:** Zero published real Signals is a valid result when thresholds or readiness gates do not pass.

**Codex Notes:** Deterministic fixtures made external-source behavior testable without presenting fixtures as live evidence.

## 2026-09-13 — Continuous intelligence and derivatives

**Goal:** Add long-running scheduling, rolling baselines, public derivatives inputs, and persistent Evidence clusters.

**Changes:** Commit `2904ac3` added bounded scheduler behavior, restart-restored buckets, Binance USD-M Futures normalization, derivatives strategies, and cluster persistence.

**Problems:** Live inputs can arrive late, out of order, stale, or before a usable baseline exists.

**Solution:** Explicit readiness states, bounded history, deterministic time buckets, capped reconnect/backoff, and audit records for rejected runs.

**Result:** Strategy runs retain input snapshots, metrics, reason codes, parameters, timestamps, and provenance.

**Tests:** Current coverage includes scheduler single-flight behavior, backoff, rolling metrics, readiness, derivatives classifications, and cluster semantics.

**Decision:** Readiness failure produces an auditable run but no ordinary Candidate.

**Codex Notes:** Soak and live checks were kept opt-in and distinct from the deterministic suite.

## 2026-09-20 — Mobile Intelligence M0/M1

**Goal:** Add a safe, observe-only Android collection surface and integrate it into Home, Warehouse, and Settings.

**Changes:** `c354a0d` defined contracts; `f02a257` added the observe-only runtime; `513920d` integrated the workspace and UI.

**Problems:** A phone screen is not useful market Evidence by itself, and raw UI trees may contain sensitive fields.

**Solution:** Introduce bounded frames, semantic snapshots, scoped element references, privacy classes, sensitive-node redaction, allowlisted applications, and a distinct `MobileObservation` type.

**Result:** The code can project observed mobile state without automatically promoting it to Evidence or Signals.

**Tests:** Current tests cover snapshot construction, redaction, frame retention, observation deduplication, audit payload safety, persistence, feed projection, and UI contracts.

**Decision:** M1 is observe-only; tap, swipe, type, launch, navigation, login, posting, messaging, and ordering remain disabled.

**Codex Notes:** The reference video informed interaction structure, while unconfirmed implementation mechanisms remained reference behavior rather than invented architecture.

## 2026-09-20 — Mobile Intelligence M1.5 gate

**Goal:** Prove the production path against a real Android Emulator before implementing navigation.

**Changes:** `144c87f` added environment diagnostics, AVD lifecycle checks, bounded commands, persisted capture projection, and a ten-check real-runtime harness.

**Problems:** The local machine had no Android SDK, ADB, Emulator, AVD, or online device. External tool calls also needed explicit time bounds and consistent SDK selection.

**Solution:** Report each prerequisite independently, block dependent checks, use the same selected SDK across diagnostics and observation, kill timed-out child processes, and require persistence/events/workspace projection for IPC acceptance.

**Result:** Three prerequisites failed and seven checks were blocked. M2 was not entered.

**Tests:** At the product baseline, 126 Rust and 31 TypeScript tests passed; typecheck, frontend build, and macOS desktop bundle also passed. The real Android smoke failed truthfully because the environment was missing.

**Decision:** No M2 code until all ten real checks pass.

**Codex Notes:** Independent review found command-timeout, SDK-selection, IPC, and shutdown gaps; regression tests were observed failing before the fixes passed.
