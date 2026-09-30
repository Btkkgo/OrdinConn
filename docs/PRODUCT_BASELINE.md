# OrdinConn Product Baseline — V0.1 Foundation

[English](PRODUCT_BASELINE.md) | [简体中文](PRODUCT_BASELINE.zh-CN.md)

> All previous OrdinConn product baselines are obsolete.

This document is the only product baseline for OrdinConn V0.1.

## Product

OrdinConn is an **AI Financial Intelligence & Execution Agent**. It proactively gathers market information, creates evidence, identifies explainable signals, produces reports, and executes only permitted actions after explicit user approval.

The two markets are Traditional Finance and Crypto. The shared workflow is:

`Data -> Evidence -> Agent Analysis -> Signal -> Report -> User Approval -> Action -> Result -> Memory`

## V0.1 scope

V0.1 contains the desktop shell, Agent Runtime, Model Gateway, Tool Runtime, Approval Engine, Evidence and Signal domains, Traditional Finance and Crypto experiences, contextual Agent Dock, SQLite persistence, mock connectors, public-data Collector Runtime, Source Registry, deterministic Strategy Engine, at least 18 demo signals, Agent Reports, Trade Proposals, Approval Capability, and Paper Execution.

The Mobile Intelligence extension adds Android Emulator observation, semantic UI snapshots, sanitized Mobile Observations, and the Home/Warehouse/Settings shell. Verified M2 navigation adds six human-triggered, snapshot-bound actions (Tap, Swipe, safe Type, Back, Home, OpenApp), with deny-all/allowlist policy, sensitive/financial target rejection, write-ahead audit, post-observation, and receipts. M2 does not add autonomous mobile execution. Mobile data remains subject to the same Source Registry, Evidence, Strategy, and Signal gates as every other source.

It does not contain real brokerage or exchange execution, real-money trading, deposits, withdrawals, transfers, wallet signing, private-key or seed-phrase access, subscriptions, a cloud platform, high-frequency background automation, advanced computer vision, or autonomous login, posting, messaging, ordering, or operations outside the bounded M3 Settings goal scope.

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

Simplified Chinese is the default desktop locale. English and Simplified Chinese (`zh-CN`) are fully enabled interface languages, selectable in Settings without restarting the runtime. Formal UI copy always uses locale keys, and locale preferences fall back to Simplified Chinese when absent or invalid. Domain models, APIs, files, code, Agent protocol, Signal, Evidence, and Report fields use English.

## Runtime and persistence

V0.1 uses a Tauri single-process embedded Rust runtime. React communicates only through typed commands and events and never directly accesses SQLite, providers, files, or core runtime state. Core crates are transport-agnostic.

SQLite relational tables store current state. `runtime_events` stores append-only audit history. Live deltas use an in-memory event bus. Startup marks unsafe unfinished work interrupted and never replays side-effecting tools automatically.

Public data follows `Continuous Collection -> Rolling History -> Baseline -> Ready Strategy -> SignalCandidate -> Evidence Gate -> Published Signal -> Agent Analysis`. The embedded runtime owns per-source scheduling, bounded history, restart-restored aggregate buckets, and long-lived WebSocket reconnect/resubscribe behavior. Binance Spot and USDⓈ-M perpetuals use separate canonical instruments. REST, WebSocket, RSS/Atom, HTML, and allowlisted observe-only mobile collectors use explicit public-data policies, health, schema-drift detection, rate budgets, retention, and deterministic source reliability. Login, paywall, CAPTCHA, cookie-gated, private access, private messages, and sensitive-field capture are prohibited.

Strategy readiness is distinct from Evidence validation. `WARMING_UP`, `MISSING_INPUT`, `STALE_INPUT`, `SCHEMA_ERROR`, and `INSUFFICIENT_HISTORY` produce an auditable Strategy Run but no ordinary Candidate. A real Published Signal may contain only real Evidence; zero real Signals is valid when no ready threshold is crossed.

## Approval and execution policy

Signal does not equal Trade. A Published Signal can create a versioned Trade Proposal. A separate Approval Request binds an exact canonical proposal hash.

The Approval Capability is single-use, object-bound, version-bound, time-limited, and non-transferable. React cannot issue or retain it. SQLite stores only a digest. Token validation, consumption, and creation of a starting Execution Record are atomic. Every ambiguous or invalid condition fails closed.

Paper Execution is the only V0.1 execution adapter and it must pass through the real Approval path. No valid Approval means no Execution.

## UI

The primary desktop shell provides Home, Warehouse, and Settings. Home integrates the Intelligence Feed, Mobile Live View, related Signals, and contextual data discussion. Existing market, signal, agent, automation, model, source, and approval capabilities remain available through those consolidated workspaces rather than separate primary navigation entries.

The approved Desktop direction is Dark Intelligence Workspace: Home, Repository and Settings share one logo-free AppShell/Sidebar, neutral black/gray surface tokens, a restrained blue accent, system typography and consistent panels, cards, controls, badges, alerts and empty states. Home retains Realtime Data, Mobile Operation with Agent Commands, and Agent Plans in approximately 27/43/30 columns. Metric numbers use stable semantic colors (total blue-violet, news blue, stocks green, chat violet, feedback amber, other gray-cyan), with no fabricated data. Settings uses the same neutral theme and a balanced model configuration/saved-services layout. Formal copy uses English and Simplified Chinese locale keys. System app icons and the window title are unchanged. Development visual fixtures must never populate production state.

## M3 bounded goal extension

Owner-authorized M3 Phase 5 connects the real Model Gateway Planner to the existing real single-step Executor for Android Settings and its preinstalled search only. Typed next actions retain deny-all policy, latest Observation binding, explicit Approval, canonical step/model/action/time budgets, exact verification, Stop and fail-closed restart recovery. Owner-supplied immutable Activity/Text completion targets are required for verifiable general goals; model claims alone never complete a Goal. Existing Home submission uses the Rust runner without a UI redesign. No configured production provider means MODEL_NOT_CONFIGURED and no model/device action. Live acceptance remains BLOCKED_MODEL_NOT_CONFIGURED; Full Autonomous M3 remains NOT_COMPLETE. M4, accounts, external apps, real funds and commercial licensing are outside scope.

## Acceptance

The app must launch and show the approved dark realtime Home workbench while retaining Warehouse and Settings. Acceptance requires actual Tauri screenshots and reference comparison, real device diagnostics/frames, evidence-linked deduplicated loaded-object metrics, typed manual commands, and preserved Approval/Safety. Pending research goals are not autonomous plans or executed actions; unsupported mobile autonomy must remain explicit. The user can switch between English and Simplified Chinese, navigate both markets and six ABC lanes, open a Signal, inspect Evidence, chat in a context-aware Agent Dock, create a Report, create a Paper Trade Proposal, request approval, approve it, and receive a Paper Execution Record. Model Settings can configure an OpenAI-compatible provider. Tests and builds must pass before the final local commit.
