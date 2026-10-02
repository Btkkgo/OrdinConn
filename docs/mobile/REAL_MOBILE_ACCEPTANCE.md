# Real Mobile Data Acquisition Acceptance

[English](REAL_MOBILE_ACCEPTANCE.md) | [简体中文](REAL_MOBILE_ACCEPTANCE.zh-CN.md)

Date: 2026-10-02 (Asia/Shanghai). Issue: [#14](https://github.com/Btkkgo/OrdinConn/issues/14). Development baseline: `5dae0cf86d023a64ddd318b448e63cc713afda57`, branch `codex/realtime-workbench`, plus the minimal manual-target repair documented below. **REAL MOBILE DATA ACQUISITION ACCEPTANCE = PASS** for this bounded run. This is not full human-operated acceptance or a claim of failure-free reliability.

## Operators and scope

H1–H2: **Human-operated**, explicitly confirmed by the owner. H3–H7: **Codex-operated under explicit user authorization**. Device: `OrdinConn_M1_5`, Pixel 8 emulator, Android 16, `emulator-5554`. Only Settings and the resolved system launcher were used. Existing UI command Audit records retain `triggeredBy=owner` as the authorization principal; they do not identify the physical operator. This report supplies that distinction without rewriting historical Audit.

The installed app bundle was not rebuilt, signed or launched. The current Tauri development process and actual SQLite were used. Desktop UI was inspected and operated through native Accessibility, not a fixture/browser replacement. Desktop screenshots were not captured because the tool lacked screen-recording permission. No account, credential, private message or private app content was opened.

## Gate evidence

| Gate | Result | Evidence |
| --- | --- | --- |
| H1 | PASS — HUMAN | Completed Observe at 13:35:15 UTC; Settings homepage, 70 elements; `mobile_observation_01a0fcd3-8437-75ab-af4e-0c5a542442cf`. |
| H2 | PASS — HUMAN | Owner confirmed real element ID/text/role/class/bounds/state inspection. Codex later inspected the real clickable parent `@e23`, LinearLayout, bounds `0,632 1080×206`, enabled/clickable, no sensitive flag. |
| H3 | PASS — CODEX | `manual_action_01a0fced-d268-7124-a6ef-967aa94c4149` completed. Before `mobile_observation_01a0fced-dfc5-770a-99ff-661015fa814f`; after `mobile_observation_01a0fcee-049c-7260-9d8d-c930b397c09e`; homepage → SubSettings; Diff +62 / -67. Running/receipt/action_diff/completed Audit exists. |
| H4 | PASS — CODEX | Settings Apps list (`spa.SpaActivity`): up `manual_action_01a0fd08-6547-746b-bed5-55b9a2ddc4bb`, Diff +38/-33; down `manual_action_01a0fd09-0417-7187-9d14-0b01a196c2b9`, Diff +33/-38, original UI-tree hash restored. Both completed with before/after IDs and Audit. UI directions describe finger motion. |
| H5 | PASS — CODEX | Real source `mobile_observation_01a0fcf0-c6e8-7424-bbf7-05751f1bec0f`, Settings homepage, captured 14:07:13.118341 UTC. Local rule/UI-tree extractor produced 40 extractions and 19 objects. Extraction run at 14:20:20.564519 UTC. No model request. |
| H6 | PASS — CODEX | Existing SQLite contains Source → Observation → Element → Extraction → Data Object relationships. Re-extraction at 14:21:30.768745 UTC produces 0 new canonical objects; 19 canonical objects, 40 deduplicated extracted rows/sightings, two extraction-run and Audit records. No orphan relation or duplicate canonical key. |
| H7 | PASS — CODEX | Native Desktop shows the real 19 list objects; List filter restores cards; Metric filter shows the honest empty state. Storage source detail opens, showing device/package/activity/time/Observation/Element/method/dedup key. Expanded original evidence shows text/resource/bounds and confidence: UI Tree 100%, list rule 65%. |

Example object: Storage (`data_01a0fcfc-cac3-77b7-b31f-4b5a75c17908`), registered source `android-manual-com.android.settings`, `el_872623eaf9a1387c680329647a9b2e5ca6f2e0640eea2e6196b1a5a272a3bdbe_1`, `android:id/title`, bounds `210,954 182×71`. Capture time is the original Observation time; extraction execution time is stored on `mobile_extraction_runs`. Repeating extraction of one Observation retains two run records but does not invent a second distinct sighting. Data remains **LOCAL ONLY**.

## Failure evidence and minimal repair

The first Tap was rejected with `TARGET_CHANGED` before an input command. The chosen Settings parent has neither Resource ID nor Content Description. The reused M3 semantic matcher therefore returned no match even though the complete UI Tree was identical. A fake-ADB regression reproduced this failure before repair.

Only `apps/desktop/src-tauri/src/mobile_collection.rs` changed: manual matching retains the existing semantic matcher, then permits an anonymous element's exact reference only when the complete element tree, session, package/activity, dimensions and redaction state match. Changed tree/text/bounds/activity or redactions reject the fallback. The M3 executor matcher, safety policy, Provider implementation and architecture remain unchanged. Two regression tests cover the anonymous-container loop and changed-context rejection.

A subsequent Tap command was sent, but its automatic post-capture failed (`ACTION_FAILED`, receipt `INTERRUPTED`, no fabricated after-state). A separately requested Observe confirmed the real destination. The old host discarded the underlying error, so its precise cause is **UNKNOWN**, not claimed fixed. Back followed by a complete Tap then passed H3. All failed records remain. Future post-capture diagnostics/reliability work should address this residual uncertainty.

The first downward swipe at the top yielded honest `NO_CHANGE`; it did not pass H4. Homepage upward scrolling exposed a sensitive settings label: value redacted, frame hidden and further navigation blocked. Protection was retained. H4 was completed on the safe Apps list; one explicit ADB Settings Intent prepared that page and is counted separately from product actions.

## Action accounting and safety

Codex acceptance window starts after 13:40 UTC; earlier human observations are not counted below.

| Action | Product requests | Actual state-changing commands |
| --- | --- | --- |
| Observe | 5 completed | Observation only |
| Tap | 3: 1 completed, 2 failed | 2 sent; first rejection sent none |
| Scroll Up | 2 completed | 2 |
| Scroll Down | 2 completed | 2; one no-change attempt |
| Back | 2 completed | 2 |
| Home | 1 completed | 1 |
| Open App | 1 completed | 1 |
| Safe Settings Apps setup | 1 external ADB Settings Intent | 1 |
| Extract | 2 local runs | No device input |

There are 34 new persisted Observations including action-internal before/after/stability captures. The 16 product action requests include two failed Tap records. Failed/timeout approval-tool attempts issued no device command and are not counted as actions. No automatic unit/integration test controlled the real emulator; the real actions above were individually selected during the explicitly authorized acceptance.

For Home/Open Settings only, the actual resolved launcher `com.google.android.apps.nexuslauncher` was temporarily added through existing Desktop mobile settings. Home and returning to Settings both completed. The original allowlist **[com.android.settings] was restored and read back from SQLite**. Final successful Observe shows Settings, 70 elements. Existing data and eight original untracked entries are preserved.

Sensitive protection: PASS for synthetic regression coverage and observed live redaction/blocking of a Settings menu label. No real password/PIN/OTP/secret was supplied or read. Input denial and repository plaintext rejection are fixture-tested, not real-account-tested. Provider Requests **0**; Signing Verification **0**; cloud data upload **0**; X Draft **NONE**.

NOT RUN: Input (no input scene was entered), Left/Right (no suitable horizontal scene), Stop (no sustained operation selected), seven opt-in real-device/final test scripts, real Provider/Planner/Gemini checks, signing/notarization/packaging/release/deployment, desktop screenshots. These are not implied PASS.

## Regression

Rust workspace **341 PASS**, seven real-device/final gates explicitly excluded; Desktop **77 PASS**; Contracts **9 PASS**; typecheck, frontend build, native Rust build, formatting and diff checks PASS. Reused successful checks for unchanged code after acceptance; real interactions did not modify source. Public-log/secret/history/identity scan PASS before publication. Next recommended episode: bounded post-action capture diagnostics and reliability.
