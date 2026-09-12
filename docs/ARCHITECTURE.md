# Architecture

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
- `ordinconn-app` coordinates repositories, transactions, recovery, and public application services.
- `apps/desktop/src-tauri` converts stable commands and events to Tauri IPC. React never receives a database handle, provider secret, or approval token.

## Runtime lifecycle and recovery

Startup follows database initialization, migrations, runtime initialization, connector initialization, Model Gateway initialization, event bus readiness, then UI readiness. Shutdown first stops new Agent turns, cancels active in-memory work, persists active turns as interrupted with audit events, and finally closes SQLite. Startup also fail-closes any unsafe `running` or `waiting_tool` state left by an abnormal exit.

## Evidence quality

The Signal Engine computes evidence quality from evidence freshness, source reliability, factual level, and evidence confidence. Model inference cannot satisfy the publication gate. Contradicting evidence stays attached, reduces quality and confidence, and moves the signal to watch when appropriate. The formula is intentionally simple and replaceable in V0.1.

## UI and localization

React uses locale keys for every formal UI label. The `en` and `zh-CN` dictionaries have identical key coverage; English is the default, invalid stored preferences fail back to English, and Settings can switch languages immediately without component or runtime changes. The visual system derives its purple background, black structure, and yellow action accent from the user-provided OrdinConn mark. The exact source image is retained at `apps/desktop/src/assets/ordinconn-logo-source.jpg`, and the desktop icon is a PNG conversion of the same asset.
