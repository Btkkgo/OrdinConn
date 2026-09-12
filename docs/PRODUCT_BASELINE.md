# OrdinConn Product Baseline — V0.1 Foundation

> All previous OrdinConn product baselines are obsolete.

This document is the only product baseline for OrdinConn V0.1.

## Product

OrdinConn is an **AI Financial Intelligence & Execution Agent**. It proactively gathers market information, creates evidence, identifies explainable signals, produces reports, and executes only permitted actions after explicit user approval.

The two markets are Traditional Finance and Crypto. The shared workflow is:

`Data -> Evidence -> Agent Analysis -> Signal -> Report -> User Approval -> Action -> Result -> Memory`

## V0.1 scope

V0.1 contains the desktop shell, Agent Runtime, Model Gateway, Tool Runtime, Approval Engine, Evidence and Signal domains, Traditional Finance and Crypto experiences, contextual Agent Dock, SQLite persistence, mock connectors, at least 18 demo signals, Agent Reports, Trade Proposals, Approval Capability, and Paper Execution.

It does not contain real brokerage or exchange execution, real-money trading, deposits, withdrawals, transfers, wallet signing, private-key or seed-phrase access, subscriptions, mobile applications, a cloud platform, high-frequency background automation, or advanced computer vision.

## Markets and ABC lanes

Traditional Finance covers stocks, ETFs, futures, oil, gold, silver, FX, bonds, and commodities through:

- A — Trading Signal
- B — Event Signal
- C — Demand Signal

Crypto covers major assets, stablecoins, NFT/Ordinals, meme assets, spot, perpetuals, and options through:

- A — On-chain Signal
- B — Event Signal
- C — Exchange Signal

Signals must explain direction, confidence, urgency, time horizon, evidence, catalysts, risks, invalidation, and observations. A buy/sell label alone is not a valid Signal.

## Evidence policy

Connectors and Agents create Signal Candidates. A Candidate can become a Published Signal only after validation and only when at least one linked Evidence record is not `MODEL_INFERENCE`. Inference may explain, synthesize, form a transmission path, or state a hypothesis; it cannot impersonate a source fact.

Evidence links are `primary`, `supporting`, `contradicting`, or `context`. Contradictions remain visible. The Signal Engine, not the model, computes final evidence quality. An insufficient Candidate cannot generate a formal report, proposal, approval, or execution.

## Agent and model policy

The Rust Agent Runtime uses OrdinConn's Thread, Turn, Item, Tool, Approval, and Event protocols. It supports streaming, normalized tool calls, context injection, interruption, cancellation, retry boundaries, typed errors, and completed-item persistence.

All models pass through Model Gateway. V0.1 implements configurable OpenAI-compatible `/v1/chat/completions`, including non-streaming, streaming, and tool calls when explicitly supported. The Agent Runtime never sees vendor response shapes. Provider capabilities are explicit. Mock Model keeps the demo usable without credentials.

English is the default and only fully enabled locale. Formal UI copy always uses locale keys. `zh-CN` exists from the first release and falls back to English. Domain models, APIs, files, code, Agent protocol, Signal, Evidence, and Report fields use English.

## Runtime and persistence

V0.1 uses a Tauri single-process embedded Rust runtime. React communicates only through typed commands and events and never directly accesses SQLite, providers, files, or core runtime state. Core crates are transport-agnostic.

SQLite relational tables store current state. `runtime_events` stores append-only audit history. Live deltas use an in-memory event bus. Startup marks unsafe unfinished work interrupted and never replays side-effecting tools automatically.

## Approval and execution policy

Signal does not equal Trade. A Published Signal can create a versioned Trade Proposal. A separate Approval Request binds an exact canonical proposal hash.

The Approval Capability is single-use, object-bound, version-bound, time-limited, and non-transferable. React cannot issue or retain it. SQLite stores only a digest. Token validation, consumption, and creation of a starting Execution Record are atomic. Every ambiguous or invalid condition fails closed.

Paper Execution is the only V0.1 execution adapter and it must pass through the real Approval path. No valid Approval means no Execution.

## UI

The desktop shell provides Overview, Traditional Finance, Crypto, Signals, Agent, Automations, Models, Data Sources, Approvals, and Settings. It uses left navigation, top context, center workspace, and a collapsible Agent Dock.

The final visual direction is derived from the user-provided `Conor右1.0.jpg`: its black-and-yellow geometric pattern is the OrdinConn mark, its purple background defines the new interface family, deep-purple surfaces preserve financial readability, black anchors navigation and Agent chrome, and yellow is reserved for selection, signals, approvals, and primary actions. The system remains minimal, professional, and restrained; it avoids casino styling and excessive motion.

## Acceptance

The app must launch and show the approved purple, black, and yellow interface with the user-provided mark. The user can navigate both markets and six ABC lanes, open a Signal, inspect Evidence, chat in a context-aware Agent Dock, create a Report, create a Paper Trade Proposal, request approval, approve it, and receive a Paper Execution Record. Model Settings can configure an OpenAI-compatible provider. Tests and builds must pass before the final local commit.
