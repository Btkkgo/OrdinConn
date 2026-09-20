# Architecture

[English](ARCHITECTURE.md) | [简体中文](ARCHITECTURE.zh-CN.md)

OrdinConn V0.1 uses a single-process embedded Tauri runtime with strict layers:

`Core crates -> Application services -> Tauri adapter -> Typed IPC contracts -> React UI`

## Decisions

- Core crates own domain rules and do not depend on transports or UI frameworks.
- `ordinconn-app` owns SQLite, migrations, recovery, services, and the in-memory event bus.
- Tauri is a host and transport adapter. It exposes stable DTOs rather than raw domain structs.
- Commands that start long work return identifiers immediately; events stream progress.
- One public event envelope carries Agent, Model, Signal, Approval, Execution, and System events.
- Relational tables answer current state; append-only `runtime_events` explains how state changed.
- OpenAI-compatible Chat Completions is isolated in its provider adapter.
- Approval is a one-time capability bound to one canonical proposal version.
- No localhost server, daemon, sidecar, distributed bus, CQRS, or full Event Sourcing exists in V0.1.

The future `StandaloneRuntimeHost`, `DaemonRuntimeHost`, and `RemoteRuntimeHost` must reuse the same domain, model, tool, approval, and signal protocols.

## Crate and transport boundaries

- `market-core`, `evidence-core`, `signal-core`, `traditional-finance`, and `crypto-core` own financial domain definitions.
- `model-gateway`, `agent-runtime`, `tool-runtime`, `approval-engine`, `execution-core`, and `computer-use` own replaceable runtime capabilities.
- `connector-runtime` is the only path from connector output to Evidence and Signal Candidates. Mock connectors use the same evidence gate as future live connectors.
- `collector-runtime` owns real public-source definitions, REST/WebSocket/feed/HTML adapters, Raw Records, normalization, deduplication, health, schema drift, and retention policies.
- `strategy-core` owns versioned deterministic strategies, rolling-window contracts, canonical instrument/entity aliases, parameter snapshots, reason codes, and rejected-run semantics.
- `ordinconn-app` coordinates repositories, transactions, recovery, and public application services.
- `apps/desktop/src-tauri` converts stable commands and events to Tauri IPC. React never receives a database handle, provider secret, or approval token.

## Runtime lifecycle and recovery

Startup follows database initialization, migrations, runtime initialization, connector initialization, Model Gateway initialization, event bus readiness, then UI readiness. Shutdown first stops new Agent turns, cancels active in-memory work, persists active turns as interrupted with audit events, and finally closes SQLite. Startup also fail-closes any unsafe `running` or `waiting_tool` state left by an abnormal exit.

## Evidence quality

The Signal Engine computes evidence quality from evidence freshness, source reliability, factual level, and evidence confidence. Model inference cannot satisfy the publication gate. Contradicting evidence stays attached, reduces quality and confidence, and moves the signal to watch when appropriate. The formula is intentionally simple and replaceable in V0.1.

Evidence clusters persist original, syndicated, independent, and contradicting members. Only distinct original/independent sources add confirmation weight. The application persists every collection boundary in relational tables; completed Signals retain data origin, strategy ID/version/parameters, reason codes, observation/source/Evidence IDs, baseline window, trigger metrics, input snapshot, and publication time.

## Core intelligence lifecycle

Catalog initialization registers validated sources and immutable strategy versions during application startup. The Tauri host starts the application-owned `CollectorScheduler` and Binance Futures WebSocket after event forwarding is ready. Per-source tasks are single-flight, globally bounded, pausable, and gracefully awaited on shutdown; failure cadence uses capped exponential backoff with deterministic jitter. Scheduler state and aggregate history buckets are restored after restart. Each collector run records success/failure and source health. Schema drift and missing rolling history produce not-ready Strategy Runs and no Candidate; no mock fallback is used on a real path.

## UI and localization

React uses locale keys for every formal UI label. The `en` and `zh-CN` dictionaries have identical key coverage; English is the default, invalid stored preferences fail back to English, and Settings can switch languages immediately without component or runtime changes. The visual system derives its purple background, black structure, and yellow action accent from the user-provided OrdinConn mark. The exact source image is retained at `apps/desktop/src/assets/ordinconn-logo-source.jpg`, and the desktop icon is a PNG conversion of the same asset.
