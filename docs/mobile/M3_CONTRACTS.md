# M3 Phase 2 — Persistent Mobile Execution Contracts

[English](M3_CONTRACTS.md) | [简体中文](M3_CONTRACTS.zh-CN.md)

> Historical Phase 2 scope. Bounded single-step execution is now implemented in [Phase 4](M3_EXECUTOR.md); full autonomous M3 remains NOT_COMPLETE.

## Scope and architecture

Phase 1 audited the reusable M2 observation, typed actions, safety, receipts, Stop, events, Model Gateway, and Approval engine. Phase 2 implements execution contracts and persistence only. Autonomous execution and the M3 Planner remain **NOT_IMPLEMENTED**. No model calls, provider configuration, device actions, or real AVD gates are performed in this phase.

`MobileGoal -> MobilePlan revision -> MobilePlanStep -> Observation / Action Receipt -> Step Result / Evidence references`

Canonical Rust contracts live in `crates/mobile-runtime/src/execution.rs`; `MobileGoalRepository` lives in `crates/ordinconn-app/src/mobile_goals.rs`. The existing SQLite database, sqlx migration runner, UUID v7 generation, RFC 3339 timestamps, `runtime_events`, and post-commit event bus are reused. React receives camelCase DTOs through typed IPC. It does not decide execution state or query SQLite.

## Goal, budget, and errors

A Goal has a typed stable ID, original objective, optional normalized objective, lifecycle timestamps, active-plan reference, budget, deadline, counters, and typed error code/message. Objective text is preserved exactly; whitespace-only or over-1000-character objectives are rejected. Goal creation does not create a Plan, invoke a model, or observe a device.

Defaults are owned by Rust: `max_steps=8`, `max_runtime_ms=120000`, `max_consecutive_failures=2`, `max_identical_observations=3`. Validation bounds steps to 1–100, runtime to 1–3,600,000 ms, consecutive failures to 1–max_steps, and identical observations to 1–100. Fields are persisted; no timer or autonomous budget-enforcement loop is started. A deadline is established on the first RUNNING transition and preserved when returning from approval.

| Current state | Permitted next states |
| --- | --- |
| PENDING | PLANNING, FAILED, STOPPED |
| PLANNING | RUNNING, FAILED, STOPPED |
| RUNNING | WAITING_APPROVAL, COMPLETED, FAILED, STOPPED |
| WAITING_APPROVAL | RUNNING, FAILED, STOPPED |
| COMPLETED / FAILED / STOPPED | None |

RUNNING requires an active plan. Completion requires all active-plan steps VERIFIED or SKIPPED, and atomically completes the plan. Failure/Stop ends unfinished steps and invalidates unfinished plans. Terminal goals cannot resume; future retry requires a new explicit execution attempt rather than reviving a terminal object.

Errors use separate enum codes and fixed human-readable messages. Codes include device/model/observe/action/verify/approval/policy errors, budget limits, STALLED, USER_STOPPED, INTERRUPTED_BY_RESTART, INVALID_PLAN, INVALID_STEP, and INVALID_STATE_TRANSITION. Raw provider responses, UI text, and objectives are not copied into error or lifecycle audit payloads.

## Plan revisions and steps

Plan revisions are monotonic per Goal and unique under `UNIQUE(goal_id,revision)`. Plan states are DRAFT, ACTIVE, COMPLETED, SUPERSEDED, FAILED. Activation atomically supersedes the previous active revision, activates the new revision, updates the Goal pointer, and appends events. It cannot roll back to an older revision or supersede an executing/waiting step. Prior plans are retained.

Steps have typed IDs, owner plan, unique sequence, semantic type, status, reason, persisted risk, target/input, expected result, timestamps, trace references, and errors. Types are OBSERVE, SCROLL_DOWN, SCROLL_UP, TAP_ELEMENT, INPUT_TEXT, BACK, WAIT, EXTRACT, COMPLETE, STOP. They are semantic contracts, never shell/ADB command payloads. Phase 2 does not map them to runtime actions.

| Current step state | Permitted next states |
| --- | --- |
| PENDING | EXECUTING, WAITING_APPROVAL, SKIPPED, STOPPED |
| WAITING_APPROVAL | EXECUTING, FAILED, STOPPED |
| EXECUTING | VERIFIED, FAILED, STOPPED |
| VERIFIED / FAILED / SKIPPED / STOPPED | None |

Only the active plan may advance a step, earlier steps must be VERIFIED/SKIPPED first, and at most one step per plan may be executing or waiting for approval. VERIFIED is reached through atomic result persistence rather than a standalone status update. A linked pending action intent cannot be claimed as verified execution. Result linkage must match the step; linked evidence must already belong to that step.

Risk is READ_ONLY, REVERSIBLE, APPROVAL_REQUIRED, or FORBIDDEN. FORBIDDEN cannot enter EXECUTING. Persisted risk and state are not execution permission: future Rust Safety must classify actions independently, and future Approval must validate a real capability. Phase 2 provides no execution endpoint.

Expected results are a finite tagged enum: UI_CHANGED, ELEMENT_VISIBLE, TEXT_EQUALS, ACTIVITY_CHANGED, ACTIVITY_EQUALS, NEW_DATA_OBJECT, NO_CHANGE_EXPECTED. The existing safe-input validator is reused for stored input and text-equality values. Password-field, key, seed, and private-data access remain prohibited. Future verification must still use the trusted runtime and privacy policy; these contracts do not implement semantic verification.

## Persistence and traceability

Migration `0008_mobile_goal_contracts.sql` adds `mobile_goals`, `mobile_plans`, `mobile_plan_steps`, `mobile_step_results`, and the small `mobile_step_evidence` association table. Canonical domain JSON is stored inside SQLite, with indexed relational columns and matching-value checks; no separate JSON storage or second database is introduced.

Foreign keys bind plans to goals, steps to plans, observations to existing mobile observations, action IDs to existing receipts, and evidence IDs to existing Evidence. The composite active-plan foreign key requires the plan to belong to the same Goal. A receipt can belong to only one step. Evidence associations retain the contributing observation ID. Results reference existing objects and never copy frames/UI trees into Goal or result records. Links can be queried with repository methods, including `get_step_evidence` and `get_step_result`.

Creation, activation, state changes, linkage, and result writes use `BEGIN IMMEDIATE` transactions with audit events. Events are published only after commit. Failure injection verifies rollback when audit insertion fails. Foreign keys use RESTRICT, execution records have no-delete triggers, and results/evidence associations are immutable. No retention/deletion API is provided.

Lifecycle events follow existing names: `mobile.goal_created/planning/started/waiting_approval/completed/failed/stopped`, `mobile.plan_created/activated/superseded/completed/failed`, and `mobile.step_created/started/waiting_approval/verified/failed/stopped/skipped`. The original `runtime_events` aggregate/entity references are reused. Typed payloads carry optional goalId, planId, stepId, observationId, actionId, evidenceId. There is no parallel mobile event table.

## Startup recovery

Initialization converts PLANNING, RUNNING, and WAITING_APPROVAL goals to FAILED with INTERRUPTED_BY_RESTART. Executing/waiting steps fail; remaining unfinished steps stop and unfinished plans fail. The transaction records the resulting events. Recovery is idempotent, leaves pending/terminal goals intact, and never replays an action or calls a model.

## Approval subject and model boundary

`MobileApprovalSubject` contains goal ID, plan ID, step ID, semantic action type, optional target hash, and observation ID. Its deterministic SHA-256 hash includes the versioned domain separator `ordinconn.mobile-approval-subject.v1`. Every binding field affects the hash. The immutable plan ID identifies its revision; a new revision receives a new ID.

This subject is not a capability. Trade Approval still accepts its existing TradeProposal object, hashing, expiry, nonce, issuer, and single-use consumption path. No second engine is created. A later phase must carefully extend the existing engine's subject abstraction without changing Trade approval semantics, and must invalidate stale mobile bindings after changed observation/plan/target.

The existing chat path `ordinconn-app/services.rs::finish_agent_turn` still uses MockModelAdapter; it is outside this phase. **M3 Planner MUST NOT use MockModelAdapter** and must use a real model through Model Gateway in Phase 3. The observed production Provider count is 0; MODEL_NOT_CONFIGURED is reserved, but there is no planner or execute command in Phase 2.

## Desktop and Research Task compatibility

IPC exposes only create_mobile_goal, get_mobile_goal, list_mobile_goals, get_mobile_goal_plan. Home Enter uses the same form submission path as the button, preserves the original objective, and persists a canonical PENDING goal. Existing card markup/classes, columns, styling, navigation, and manual controls are retained. A goal without a plan displays localized “Waiting for planning”; it is a goal card, not a fabricated Plan.

Goals reload from Rust persistence. Contract metadata events refresh goals without claiming that the device is observing. Existing ResearchTask persistence and its detail-view research command remain intact. Historical research tasks are not silently promoted to plans or duplicated into goals; their original records remain readable. Only new Home mobile objectives use MobileGoal.

## Validation and next boundary

See the [daily DevLog](../devlog/2026-09-27.md) for fresh test/build outcomes. Tests use isolated SQLite databases and synthetic observations/receipts, not real devices. Migration tests run the original migrations through version 7, seed legacy data, apply the current migration runner, and verify events, observations, saved/favorite mobile feed data, research tasks, settings, and foreign-key integrity.

The pre-existing concurrent AVD fixture timing issue remains KNOWN_TEST_INFRA_LIMITATION. This phase changes neither its budget nor workspace parallelism. Real AVD/model/autonomous acceptance remains NOT RUN / NOT_IMPLEMENTED. Production migration occurs on a future application launch; Phase 2 validation does not mutate the production database.

Phase 2 stops for owner acceptance. Phase 3 is Real Model Planner + Strict Typed Next Action; executor, Stop/Approval integration, and autonomous real-device gates remain later work. Commit NONE / Push NONE.
