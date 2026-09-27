# M3 Phase 5 — Planner and Executor Integration

[English](M3_PHASE5.md) | [简体中文](M3_PHASE5.zh-CN.md)

Date: 2026-09-27. [Issue #12](https://github.com/Btkkgo/OrdinConn/issues/12) remains **OPEN / status:needs-validation**. Phase 5 integration is implemented; **Live Planner + Executor Gate: BLOCKED_MODEL_NOT_CONFIGURED**. **Full Autonomous M3: NOT_COMPLETE**.

## Configuration and production path

The desktop application resolves its own application-data database, `ordinconn-v0-1.sqlite3`. The production database was opened read-only: **Production Provider Count = 0**. No production migration, provider insertion, credential lookup, or live model request was performed during this audit.

Reuse **Settings → Models** to configure one enabled OpenAI-compatible chat provider: endpoint, default model identifier, and API key if the endpoint requires one. The existing save-provider IPC persists configuration and a credential reference; the existing system credential store keeps the key in OS Keychain. Do not put keys in Goal text, SQLite business records, logs, screenshots, or public records. Model selection is deterministic: missing, incompatible, or ambiguous configuration fails before observation or model dispatch.

The production path is **Workbench Goal → typed Tauri IPC → Rust bounded runner → fresh Observe → Model Gateway / configured provider → strict Planner DTO → canonical persisted Step → accepted Executor → Receipt / Observe After / Verify → next Planner decision or independently verified completion**. React submits identity and owner intent; it cannot submit ADB commands, coordinates, arbitrary action objects, or credentials to the Executor. Test adapters remain test-only. A localhost protocol fixture proves integration through the actual Gateway adapter but is **TEST_ONLY**, not real-model acceptance.

## Boundaries and changes

- Existing action types are retained. Model output remains strict typed JSON, with existing bounded malformed-response retries and no arbitrary tools or shell execution.
- Every outbound model attempt, including failures and malformed responses, reserves the persistent Goal model budget. Step count, execution count, retries, identical observations, consecutive failures, and one wall-clock deadline share canonical Goal state. Pausing for Approval does not reset the deadline.
- Planning commits against the newest observation. A historical requested observation is rejected. An unclaimed stale pending Step can be skipped and freshly replanned within budget; a claimed or emitted action is never replayed.
- Native UI reads use a distinct temporary XML path per call, verify the actual dump-success marker for that path, and clean up only their own file. A zero-exit failed dump cannot be treated as a new observation or read a shared/previous XML artifact. The same guard applies immediately before dispatch.
- Stop cancels a waiting model request immediately, prevents further decisions and side effects, waits for an already emitted action's receipt and verification, then stops unfinished Goals and the device session. After an approved emitted action settles, STOPPED also takes priority over completion or continuation. Device leases are released on errors and panic paths.
- Approval uses the existing Approval Engine's session-bound, expiring, single-use capability. Its subject binds Goal, Plan revision, Step, observation and canonical action intent. Persisted `APPROVAL_REQUIRED` risk is retained; authorization is consumed atomically with the execution claim. Model output cannot approve itself.
- Completion requires an explicit immutable owner criterion and a verified owned Step, action Receipt, result, and newest post-observation. Supported owner targets compare exact Settings activity or safe input text at a specific Android resource. A model completion proposal alone cannot complete a Goal. Approved final Steps use the same completion verification without an extra model call. Natural-language Goals without a bound criterion fail closed at completion.
- Static qualified Android component/resource identifiers are validated separately from user text. Password, wallet, private-key and messaging identifiers remain blocked. Input remains the existing safe ASCII policy; owner input completion targets reject secret-like tokens and seed-like phrases before persistence or model context; no broad secret-filter relaxation or external-app allowance was introduced.
- Restart after durable action intent still interrupts unfinished execution and never automatically replays a device side effect.

The sole baseline was extended only for bounded M3 execution on Android Settings and its preinstalled system search component. No M4, new action class, complex UI, commercial account, payment, external app, or general-purpose automation was added. Existing Workbench and Phase 2–4 work were preserved.

## Verification

| Gate | Current result |
| --- | --- |
| Default parallel Rust workspace | PASS: 287; Desktop 54/54 after the final test-only preparation adjustments |
| Desktop TypeScript / contracts | PASS: 64 / 9 |
| Typecheck / frontend build | PASS |
| Rust build / macOS app packaging | PASS: workspace build and current release app bundle |
| Isolated concurrent AVD fixture | PASS: 20/20 final repeated invocations |
| Related AVD lifecycle tests | PASS: 3/3 |
| Real BACK / SCROLL_DOWN / EXTRACT | PASS: Before/Action/After/Verify, completion, duplicate denial, Evidence/Projection; Planner TEST_ONLY |
| Real safe INPUT_TEXT surface | PASS: 3/3 consecutive after the fix; 2/2 after the final test-text adjustment; exact value and keyboard restoration verified |
| Production provider discovery | PASS: count 0, read-only `MODEL_NOT_CONFIGURED`, no credential access |
| Real model + Planner + Executor, at least two decisions | **BLOCKED_MODEL_NOT_CONFIGURED** |
| Full Autonomous M3 | **NOT_COMPLETE** |
| Security sanitizer / public-document links / diff | PASS |

Regression coverage includes Gateway continuation with two distinct decisions, exact completion truth, latest observation binding, stale unclaimed replanning, model/action/time budgets, zero-provider early exit, Stop during model wait and during an emitted action, Approval expiry/restart/binding/one-time consumption, duplicate execution, and restart after intent persistence. These are fixture-backed regressions unless explicitly identified as a real AVD gate.

The previously failing concurrent AVD test now owns independent temporary SDK and AVD homes, hosts, serial namespaces, delayed-tool markers and command traces. Synthetic cold-start delay occurs once per fixture; the test-only deadline changed from 3 to 5 seconds. Production timeouts and freshness rules were retained; no global test serialization was added.

Earlier failures remain evidence: the initial default run failed on the old shared-latency lifecycle fixture; a pre-M3 migration fixture incorrectly loaded later migrations without their dependencies; exact-input setup exposed rejected static metadata and an underscore-containing value outside the existing input policy. These were resolved in their respective fixture/validation paths. A real input attempt under concurrent release compilation was denied as `StaleSnapshot` before sending its input command; it was not counted as PASS or replayed. Final real-device results below supersede only that attempted run, not its historical safety evidence.

Subsequent input attempts exposed an interrupted post-click observation and the preinstalled keyboard's first-use stylus tutorial intercepting text. Exact verification correctly failed because the Settings field remained empty, even though the tutorial received the harmless test string. The observed tutorial was dismissed once with BACK during surface preparation, without repeating a failed Goal or changing the application allowlist. A workspace doc-test attempt also failed when another Cargo invocation replaced a dependency artifact; final workspace verification is run sequentially with default test parallelism. These failed attempts are not counted as successful runs.

The audit also found that [Android's native dump command](https://android.googlesource.com/platform/frameworks/testing/+/refs/heads/main/uiautomator/cmds/uiautomator/src/com/android/commands/uiautomator/DumpCommand.java) can exit normally after an idle/root failure without producing a file. Reusing one fixed path could read an old or concurrent host's dump. Unique file ownership and a verified success marker now fail closed; a regression simulates this zero-exit failure and proves that no previous XML is read. Command timeouts and freshness bounds were not increased.

The approved final-action Stop regression first reproduced Completed instead of Stopped (RED), then passed after the fix (GREEN), and is included in the final 287-test default run. The emitted action Receipt is retained, but the Goal must be STOPPED.

The final real Executor regression initially failed before dispatch because test preparation used `--activity-new-task`, which Android 16 rejects. Removing this test-only option reuses the verified clear-top preparation; the repaired real gate passed 1/1 (78.04 seconds), including BACK/SCROLL_DOWN/EXTRACT and duplicate denial. Its generic page lacked an input surface; the separate Search gate proves exact input in 3/3 recorded runs. The failure remains recorded. Production execution code, deadlines and the release bundle are unaffected.

Initial normal push was blocked because the installed pre-push hook referenced identity scripts missing from this branch. The five existing identity/history/hook/installer/test files were recovered byte-for-byte from the same repository’s `5056389` security commit. The installed hook was unchanged and matches its canonical version. Identity fixture tests, repository-local noreply identity and reachable HEAD history all PASS. No gate bypass, history rewrite or force push was used.

## Real input surface and remaining acceptance

The dedicated Android emulator exposes Settings' search entry. Its preinstalled system search activity is `com.google.android.settings.intelligence.modules.search.SearchActivity`, with focused non-password input resource `com.google.android.settings.intelligence:id/open_search_view_edit_text`. The opt-in gate prepares the observed preinstalled search activity through a system intent, observes and resolves its input field, then uses the real typed Executor to enter a harmless random `M3 <nonce>` value, checks exact Receipt attestation, and independently observes the actual field value. It then returns from the search surface. This uses spaces to retain the accepted input policy. It does not install a helper app or use personal/account fields.

The opt-in gate temporarily disables the dedicated AVD's `stylus_handwriting_enabled` setting during preparation, preserving its original value (including an unset default) and restoring and checking it before PASS. [Android defines this system setting](https://android.googlesource.com/platform/frameworks/base/+/refs/tags/android-16.0.0_r2/core/java/android/provider/Settings.java). This affects test preparation only. It retries only independent Observe failures up to three times; it never repeats an action or retries a value mismatch. Merely seeing a focused EditText behind a keyboard overlay does not prove that the intended field will receive input. The exact post-value comparison remains mandatory.

Once the owner configures the existing model settings, run a safe real Goal with **at least two meaningful Planner decisions**, using the current emulator UI and an explicit owner completion target. Record provider type and model identifier without credentials, and prove real Observe → real model planning → typed action → real Executor → post-observation → independent verification → continuation/completion. Until then, no real-model success or M3 completion may be claimed. Issue #12 remains open even after a successful gate until owner acceptance. No M3 milestone X draft was generated.

Private start snapshot, binary diff and preserved file archive were created before changes. All initially dirty paths remain present and all five initial JPEG hashes remain unchanged. Private traces and captures are excluded from publication.
