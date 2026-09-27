# M3 Phase 3 — Real Model Gateway Planner

[English](M3_PLANNER.md) | [简体中文](M3_PLANNER.zh-CN.md)

> Current Phase 5: bounded Planner/Executor integration is implemented. Live model acceptance is **BLOCKED_MODEL_NOT_CONFIGURED** (Production Provider Count **0**); Full Autonomous M3 is **NOT_COMPLETE**. Earlier Phase 2–4 validation records below remain historical evidence. See the [Phase 5 closeout record](M3_PHASE5.md) for current validation.


> Historical Phase 3 planning-only scope. The [Phase 4 executor](M3_EXECUTOR.md) now implements explicit single-step execution; full autonomous M3 remains NOT_COMPLETE.

## Scope and architecture

Phase 3 implements explicit, bounded planning. It does **not** execute a mobile action. Autonomous execution remains **NOT_IMPLEMENTED**. Phase 4 and Phase 5 are outside this change. See [Phase 2 contracts](M3_CONTRACTS.md) and [Current Status](../CURRENT_STATUS.md).

`MobileGoal + stored observation + bounded execution history -> Model Gateway -> untrusted JSON -> strict Rust decoding -> domain/safety validation -> atomic Plan/one PENDING Step or non-executing proposal`

The service is `ordinconn-app/mobile_planner.rs`. Canonical decision types, schema, validation, and the single versioned `MOBILE_PLANNER_V1` prompt are in `mobile-runtime/planner.rs` and `mobile_planner_v1.txt`. Provider-specific request JSON remains in `model-gateway`. The existing chat MockModelAdapter is outside this path; M3 never imports it or falls back to a mock. A deterministic adapter exists only inside a `#[cfg(test)]` module.

## Planner input and privacy

The context contains Goal ID, minimized objective, Goal state, latest revision, configured step budget, all steps used, remaining steps, observation-missing flag, current observation, recent steps/failure codes, allowed actions, forbidden capabilities, and the existing application allowlist. No screenshot, full database history, credential, tool, filesystem, Tauri command, shell, or ADB capability is supplied to the model.

Observations retain the stored observation/snapshot/session IDs, capture timestamp, package/activity, dimensions, and up to **80** semantic element references. No element coordinates are included. The existing runtime redaction vocabulary and redaction markers are reused; sensitive screens/targets are blocked. Stored Observation privacy must be PUBLIC or USER_ALLOWED; SENSITIVE or missing/unknown classification is rejected before a model call. All editable-element contents are withheld, including fields whose contents could be an OTP without an explicit password marker. Safe visible labels are bounded to 256 characters. Safe objectives retain their full bounded Goal text (up to 1,000 characters), including Chinese objectives; only UI labels use the shorter projection. Suspected sensitive objectives/metadata are redacted conservatively. This may omit benign long or security-related text; no semantic privacy guarantee is claimed beyond this fail-closed policy.

History contains at most **5** recent steps, state/type, error codes, and observation references, without model reasons or previously entered text. Total serialized context is at most **32 KiB**. Full history counts still enforce the step budget. A missing observation is explicitly marked; only OBSERVE/WAIT can be proposed without one. Phase 3 never captures a new observation. Future execution must revalidate device identity, freshness, foreground, bounds, focus, and permissions.

## Provider resolution and MODEL_NOT_CONFIGURED

Settings currently stores enabled providers and each provider's default model; it has no separate planner-selection field. Therefore selection requires **exactly one enabled provider with a non-empty default model**. No provider rows means `MODEL_NOT_CONFIGURED`; zero or multiple enabled rows, or an empty selected model, means `MODEL_NOT_SELECTED`. Unsupported adapter/capability and provider failures map to fixed typed errors. There is no alphabetical/random selection, hardcoded vendor, automatic provider creation, developer key, or network discovery.

M3 reuses the OpenAI-compatible adapter and existing secure credential store, resolving the configured provider ID through keyring when a credential reference exists. Credential values/references, full prompts, provider error bodies, and raw model outputs are never audited. Provider error bodies are discarded rather than conditionally copied. A reply echoing the supplied credential is rejected before persistence.

The current production provider count is **0**, checked through a read-only SQLite connection. The shared `plan_mobile_goal` service gate returns `MODEL_NOT_CONFIGURED` before Goal lookup or writes. `mobile_planner_provider_gate` does not initialize AppRuntime, run migrations, read credentials, or call a model. The typed IPC helper is separately tested with an existing Goal in an isolated migrated database; no fake Plan/Step is created and the Goal remains PENDING. The application was not launched during this gate.

## Strict output schema and local validation

The fixed root object always has exactly `decision`, `action`, `completion`, and `failure`; unused members are null. `decision` is one of `next_action`, `complete`, `cannot_proceed`. Every object forbids unknown fields. The native schema uses a root object, required fields, and nested unions compatible with the documented [structured-output subset](https://developers.openai.com/api/docs/guides/structured-outputs).

Exactly one candidate action is permitted: OBSERVE, SCROLL_DOWN, SCROLL_UP, TAP_ELEMENT, INPUT_TEXT, BACK, WAIT, EXTRACT. Each needs a non-empty bounded reason and a typed Phase 2 ExpectedResult. TAP/INPUT require an existing current element reference; INPUT also requires an editable, enabled, non-sensitive target and runtime-safe text. Scroll never accepts swipe coordinates. BACK accepts only compatible UI/activity expectations. WAIT is **1–5,000 ms**. EXTRACT persists only an extraction intent and NEW_DATA_OBJECT expectation, creating no Data Object or Evidence. COMPLETE and STOP are not planner actions.

Local Rust reclassifies OBSERVE/WAIT/EXTRACT as READ_ONLY and safe navigation/input as REVERSIBLE. Model risk declarations are unknown fields and rejected. Sensitive, financial, messaging, publishing, account-deletion, and irreversible intents/targets fail closed. The application allowlist is unchanged; planning is not execution permission or an Approval capability.

Malformed/empty/fenced JSON, multiple actions, arbitrary action names, extra security fields, coordinates, shell/ADB/JavaScript, invented references, unsafe input, incompatible expectations, unsupported completion references, oversized output, and tool/refusal/truncated provider replies are rejected. No JSON repair is performed. The native schema does not replace local validation.

## Structured output and bounds

Capabilities come from the user's existing Settings configuration; capability flags are not inferred from a vendor's name. `structured_output=true` sends strict `response_format=json_schema`. Otherwise `json_mode=true` sends `json_object`; providers declaring neither receive the same JSON-only system prompt and full schema without an unsupported response-format field. Every mode passes through the same strict decoder and domain/safety checks. A provider rejecting its declared native capability fails with MODEL_ERROR; no automatic capability downgrade hides the failure.

Limits: **2 attempts** only for local INVALID_MODEL_OUTPUT, **30 seconds total** across attempts with Gateway request timeout, **2,048 output tokens**, **64 KiB HTTP response**, **16 KiB candidate JSON**, **32 KiB context**, **5 recent steps**, **80 elements**, **5,000 ms maximum WAIT**. Schema/domain/policy failures are not repaired or retried. Budget, terminal/approval state, deadline, and existing pending/executing/waiting steps are checked before network/credential access. Application shutdown cancels in-flight planner futures; full Stop priority remains Phase 5.

## Persistence, status, and runtime events

A successful action creates the next monotonic Plan revision with exactly one PENDING Step and global sequence `steps_used + 1`. Prior verified revisions remain; the previous active plan is superseded only when no pending/executing/waiting step exists. WAIT duration and extraction intent extend canonical step JSON as optional fields compatible with old records. The original objective remains intact in Goal/Plan persistence.

Goal moves PENDING -> PLANNING only inside the successful commit and remains PLANNING while its active pending step waits for the future executor. Phase 3 does not enter RUNNING or set started_at. The existing panel displays localized “Plan ready. Waiting for execution.” without adding buttons or changing layout. A failed model request leaves the prior Goal state intact, never a new indefinitely PLANNING state. Existing restart recovery still fails closed without replaying actions.

Migration **0009** adds immutable `mobile_planner_decisions` for proposal/outcome and version/provider metadata with Goal/Plan/Step/Observation foreign keys. Only isolated databases receive this migration during validation. Plan/Step, outcome and `mobile.planner_succeeded` audit are committed atomically. The repository rechecks state, budget, allowlist, observation and revision under BEGIN IMMEDIATE. Concurrent/stale candidates cannot persist two pending steps; injected audit failure rolls the whole write back.

Events use existing names/families: `mobile.planner_started`, `mobile.planner_succeeded`, `mobile.planner_failed`, plus Phase 2 Goal/Plan/Step events. Payloads contain references, provider/model IDs, planner/schema versions, elapsed duration, outcome/error code. They omit objective, prompt, screen text, entered text, secrets and raw output. Preflight rejection before a model attempt has no started event or production write.

## Completion proposal and cannot proceed

COMPLETE requires a bounded reason and non-empty supporting IDs from the current observation or verified recent history. It persists a **Completion Proposal only**, never a COMPLETED Goal or COMPLETE Step. Phase 4 must independently verify completion criteria.

CANNOT_PROCEED accepts only NEEDS_NEW_OBSERVATION, TARGET_NOT_FOUND, INSUFFICIENT_CONTEXT, GOAL_UNSUPPORTED, SAFETY_BLOCKED. Its proposal is retained without a Plan/Step or fabricated Evidence.

## Typed IPC and test model boundary

`plan_mobile_goal` accepts `{ input: { goalId, observationId? } }` and returns an outcome DTO with decision, references, waitingExecutor and optional proposal. TypeScript only describes DTOs. No executor endpoint, new button, automatic Enter-triggered planning, autonomous loop or model-enabled UI workflow is added. Home still creates a pending Goal without model cost.

Tests use isolated SQLite databases, deterministic model outputs only under cfg(test), and local HTTP servers driving the real Gateway/production service. These prove architecture, format selection, local rejection, timeout, response size, cancellation, transactions, history/revisions and no execution; they do not prove a live paid provider's behavior. No AVD is required or used.

See the [daily DevLog](../devlog/2026-09-27.md) for fresh counts and build/gate evidence. Live-provider acceptance is **NOT_RUN_MODEL_NOT_CONFIGURED**. Full M3 remains OPEN / NEEDS VALIDATION in [Issue #12](https://github.com/Btkkgo/OrdinConn/issues/12); #11 remains OPEN. Commit **NONE** / Push **NONE**. Stop after Phase 3; Phase 4 executor and Phase 5 Stop/Approval integration remain unimplemented.
