# M3 Phase 4 — Bounded Rust Mobile Executor

[English](M3_EXECUTOR.md) | [简体中文](M3_EXECUTOR.zh-CN.md)

> Current Phase 5: bounded Planner/Executor integration is implemented. Live model acceptance is **BLOCKED_MODEL_NOT_CONFIGURED** (Production Provider Count **0**); Full Autonomous M3 is **NOT_COMPLETE**. Earlier Phase 2–4 validation records below remain historical evidence. See the [Phase 5 closeout record](M3_PHASE5.md) for current validation.


## Scope and evidence boundary

Phase 4 implements one explicit persisted Step per invocation. The production Rust executor is real; the real-device acceptance planner is **TEST_ONLY**. Production Provider Count is **0**, so Live Planner + Executor is **NOT_RUN_MODEL_NOT_CONFIGURED**. Full Autonomous M3 is **NOT_COMPLETE**. Issue [#12](https://github.com/Btkkgo/OrdinConn/issues/12) remains OPEN / status:needs-validation; #11 remains OPEN and operator acceptance in #10 remains owner-led.

This phase adds no execution loop, model fallback, provider setup, approval consumption, Phase 5 Stop priority, real-money operation, or new UI control. Commit NONE / Push NONE. Earlier Phase 2/3 and workbench changes remain local.

## Architecture and device ownership

`execute_mobile_goal_step({ goalId })` → `AppRuntime` → `MobileGoalExecutor` → existing `MobileHost` → existing typed M2 action → real post-observation → `MobileVerificationEngine` → transactional Step Result.

The core stays transport-agnostic. The desktop adapter only supplies the existing host. React receives typed DTOs and persisted state through existing cards and runtime events; submitting a Goal does not plan or execute it. Goal refresh ignores superseded responses. Extraction refreshes the real workspace projection.

A Rust RAII device lease holds device, optional Goal, executor identity and acquisition time. The one-process application owns a single shared lease registry; it also protects manual M2 action IPC. The lease remains in blocking host work even if its caller future disappears. Durable single-use execution identity plus a UNIQUE Step constraint prevents duplicate claims. Different devices have independent leases. Errors and panics release Rust ownership; ambiguous persisted EXECUTING attempts require recovery rather than replay.

Every opt-in real AVD gate also uses one shared cross-process OS advisory file lock. Gates run separately from normal synthetic parallel tests; opt-in tests returning early are not real-device evidence.

## Observe, resolve, act and verify

The repository rejects terminal/inactive Goals, non-active Plans, a non-next Step, exhausted step/time/failure/stall budgets and existing executing or approval-waiting Steps. Claim and start audit events commit together before external work. PLANNING moves to RUNNING only when execution is actually claimed.

Fresh Observe-before verifies emulator/session, allowlist, foreground package/activity, public privacy state and snapshot age. Step references bind to that observation while the execution attempt retains the original planning observation. Resolution requires the planned page to remain structurally applicable. Labels and resource identities bind targets; an unlabelled clickable container can be rebound only by unique actual structure/bounds on the unchanged complete page. Planner coordinates, shell/ADB commands, URLs and intents are never executable IPC inputs.

SCROLL_DOWN maps to a typed upward swipe; SCROLL_UP maps to downward swipe. TAP_ELEMENT resolves a live enabled target. INPUT_TEXT requires a safe focused editable target and matching TEXT_EQUALS. BACK requires ACTIVITY_CHANGED, exact ACTIVITY_EQUALS or semantic ELEMENT_VISIBLE, never a PNG-only change. OBSERVE and EXTRACT do not mutate the device. WAIT is bounded and must fit the remaining runtime budget.

Mutation writes the existing Pending Receipt/audit intent first, then sends exactly one typed action. Host progress acknowledgements persist Action-completed and Observe-after-started at those actual phases. Foreground acquisition can wait at most ten seconds for a transient activity transition; it only rereads focus and never repeats the action. Existing per-command bounds remain enforced, and the Goal deadline is checked before mutation and final verification. This is not preemptive Phase 5 Stop.

Verification uses application structure, exact package/activity, semantic element identity, exact editable value or a newly persisted Data Object. It excludes system chrome clocks, snapshot/ref IDs and screenshot-only differences. A mutation cannot pass on a receipt alone: a matching real post-observation and receipt trace are required. Session/device changes, wrong values, wrong activities and ambiguous targets fail closed. Verification failure fails the Step and Goal and stops remaining pending Steps. A successful Step leaves the Goal RUNNING until a separate supported completion verification.

## Input and completion verification

Input values are compared exactly, without trimming or case folding, against the same unique editable control before post-capture redaction. The trusted receipt keeps only `inputValueVerified`, text length/hash and snapshot/tree bindings. The domain verifier accepts that attestation only for the correct Type target, exact expected value hash, length and Verified receipt. A changed page with the wrong input value fails. No password/OTP/seed/private-key surface is accepted.

Completion prose is only a proposal. The current supported criterion is an explicitly bound, one-step user objective: “Scroll down the current Settings page once” (or its exact Chinese equivalent). A completion proposal must reference verified before/after observations belonging to this active Goal/Plan/Step. Every required Step, receipt and execution attempt must be verified. Unsupported natural-language Goals, foreign references, sensitive evidence and insufficient support cannot complete a Goal.

## Extraction and traceability

EXTRACT registers a narrow `android://com.android.settings` Computer source through the existing Collector Source Registry. It projects bounded sanitized visible Settings labels from the real observation; editable values and model prose are excluded. Generic Settings extraction is `mobile_observation_object`, classified as Other, with no fabricated News/Stock/Customer Feedback labels or market assets.

Source, immutable Data Object, observed ComputerUse Evidence, Step/Evidence link and final Result/audit commit together. Facts, source locator and capture time must equal the actual source observation. IDs link Goal → Plan → Step → before/after Observation → Action Receipt → Step Result → Data Object → Evidence. Failure rolls back object/evidence/result projection atomically. Mobile observation deduplication does not merge a distinct persisted extraction object into its source observation; repeated IDs still deduplicate.

## Crash and audit safety

External device actions are never run inside a database transaction. Claim/start and final result/audit boundaries are transactional. An audit failure after an external action retains an unresolved EXECUTING attempt and never automatically retries it. Restart marks active Goals/Steps failed with INTERRUPTED_BY_RESTART and execution attempts interrupted. Immutable attempt identity, one result per Step and write-ahead action intent preserve traceability without claiming exactly-once behavior across an unrecorded crash.

## Real device acceptance

The independent opt-in command is:

```sh
ORDINCONN_MOBILE_M3_EXECUTOR_SMOKE=1 cargo test -p ordinconn-desktop --lib real_phase4_executor_gate_is_explicitly_gated -- --nocapture
```

It owns the dedicated `OrdinConn_M1_5` AVD lock, uses an isolated temporary database, navigates only public Settings test pages, and inserts deterministic test Plans through the same persisted Step contract. It does not initialize or migrate the production database.

Fresh real gate: **PASS**. Safe child-page Tap followed by **BACK PASS / ACTIVITY_EQUALS**; **SCROLL_DOWN PASS**, real before/action/after/typed verification, duplicate protection and supported Goal completion; **EXTRACT PASS**, real object, Evidence and projection query. **INPUT_TEXT NOT_AVAILABLE_TEST_SURFACE**: no eligible safe focused editable surface was found. Synthetic exact-value/false-success input tests pass; they are not a real input Gate.

Earlier integration attempts failed on transient missing focus, an unlabelled target and a harness duplicate snapshot insert. The fixes preserve all action safety boundaries: bounded focus observation, unique unchanged-page target binding and fresh harness observations. Those earlier failures remain recorded; no failed action was replayed by the executor.

## Validation

Earlier default-parallel Rust workspace **272/272 PASS**; final default **240 PASS / 1 FAIL / KNOWN_TEST_INFRA_LIMITATION**; final complete serial workspace **272/272 PASS**. Executor service **18/18**, domain **9/9**, source policy **1/1**, desktop **52/52**, Phase 2 repository/domain **30/6**, Phase 3 planner service/domain **20/10**, Model Gateway **9/9**, original Trade Approval regression PASS. The normal suites leave opt-in real gates inactive; the enabled independent AVD gate passed separately. Desktop TypeScript **63/63**, Contracts **9/9**, typecheck, frontend/Rust builds, macOS Tauri release app packaging, rustfmt, public security/sanitizer/docs and diff gates PASS. Earlier Phase 3 fixture failures remain historical; current default concurrency was not weakened. Provider **0 / MODEL_NOT_CONFIGURED** was rechecked through the read-only production gate with no migration or configuration/secret access. Lease panic/poison and non-next-claim tests pass. The production planner/Gateway and four existing workbench JPEG artifacts were preserved.

See [Current Status](../CURRENT_STATUS.md), [Planner](M3_PLANNER.md), [contracts](M3_CONTRACTS.md) and [daily DevLog](../devlog/2026-09-27.md). Stop after Phase 4; Phase 5 and full autonomous acceptance require a separate owner instruction.


Final parallel rerun: after expanding lease panic/poison and non-next-claim assertions, final `cargo test --workspace` stopped at **240 PASS / 1 FAIL** on the existing `mobile::tests::concurrent_avd_lifecycle_fixtures_remain_independent_during_slow_tool_startup` (Fixture_AVD_3 AvdBootTimeout); remaining suites were not run. The earlier complete Phase 4 default run passed **272/272**, but the final current default result is **FAIL / KNOWN_TEST_INFRA_LIMITATION**, not PASS. No timeout increase, forced global serialization or shared real AVD contention was introduced. Final complete serial results are recorded separately.
