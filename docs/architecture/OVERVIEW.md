# Public Architecture

[English](OVERVIEW.md) | [简体中文](OVERVIEW.zh-CN.md)

> Current Phase 5: bounded Planner/Executor integration is implemented. Live model acceptance is **BLOCKED_MODEL_NOT_CONFIGURED** (Production Provider Count **0**); Full Autonomous M3 is **NOT_COMPLETE**. Earlier Phase 2–4 validation records below remain historical evidence. See the [Phase 5 closeout record](../mobile/M3_PHASE5.md) for current validation.


## Product boundary

OrdinConn is an open-source, model-agnostic Agent Runtime direction. V0.1 applies that runtime to an AI Financial Intelligence and Execution Agent; the current product is not yet a general-purpose computer-control platform.

The verified financial flow is:

`Data -> Evidence -> Agent Analysis -> Signal -> Report -> User Approval -> Action -> Result -> Memory`

Real-money execution, wallet signing, transfers, private-key access, and automatic mobile control are outside V0.1.

## Runtime layers

`Core crates -> Application services -> Tauri adapter -> Typed IPC -> React UI`

- Core crates own domain rules and do not depend on Tauri or React.
- `ordinconn-app` owns SQLite, migrations, application services, recovery, and the in-memory event bus.
- Tauri owns host integration and stable command/event DTOs.
- React consumes typed IPC and never receives raw database, provider-secret, or approval-capability access.

## Replaceable models and sources

Model Gateway normalizes provider capability, request, streaming, and tool-call behavior before the Agent Runtime sees it. The current repository implements an OpenAI-compatible adapter plus a mock provider; support for other named model families is a direction, not an implemented list.

Connector Registry is the only source entry boundary. Public collectors normalize raw records, track health and schema drift, deduplicate observations, and feed deterministic strategies. A model inference cannot impersonate source Evidence.

## Evidence and execution

A `SignalCandidate` becomes a published Signal only after validation and only with at least one non-inference Evidence link. Contradictions remain attached. Signal quality is computed by the Signal Engine.

A Signal is not a trade. A versioned proposal must create an Approval Request, and a single-use capability bound to the exact proposal must be atomically consumed before the Paper Execution adapter can run.

## Mobile Intelligence

The mobile path is deliberately staged:

`Device state -> Observe -> Structured UI state -> Policy -> MobileObservation`

Later phases may add:

`Agent decision -> Action -> Post-action observation -> Verification -> Evidence -> Strategy -> SignalCandidate`

Current mobile behavior includes verified M1.5 observation and M2 manual navigation. The local [M3 Phase 4 foundation](../mobile/M3_EXECUTOR.md) adds explicit single-step execution, device leases, real before/after observation, typed verification and source-backed extraction. TEST_ONLY planning verifies the production executor on Settings AVD; Provider 0 and blocked live model acceptance keep full autonomous M3 NOT_COMPLETE. Android process execution remains in the Tauri host; transport-independent contracts/verification remain in `mobile-runtime`; persistence/orchestration and projection remain in `ordinconn-app`.

## Perception direction

The intended local perception order is:

`System events -> Accessibility/UI tree -> Bounded capture -> Change detection -> OCR/Vision`

This is a design direction, not a claim that every desktop layer is implemented. Continuous recording is not the default design. Structured state is preferred because it is cheaper to process, easier to audit, and easier to constrain with privacy policy.

## Privacy modes

The planned perception modes are Manual Capture, Application Allowlist, and explicit Work Session. Password fields, banking, payments, wallets, recovery phrases, verification codes, password managers, private browsing, and camera video are prohibited by default.

Mobile M1 already enforces application allowlisting and sensitive-node redaction. Broader desktop perception modes remain designed work.
