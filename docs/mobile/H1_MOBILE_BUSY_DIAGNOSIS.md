# H1 MOBILE_BUSY diagnosis

[English](H1_MOBILE_BUSY_DIAGNOSIS.md) | [简体中文](H1_MOBILE_BUSY_DIAGNOSIS.zh-CN.md)

Issue: [#14](https://github.com/Btkkgo/OrdinConn/issues/14). Baseline: `ff33c7d5c4789d4e44ebed5917938905b989cabd`, `codex/realtime-workbench`. H1 failed during owner testing; H2 was not started. This report contains sanitized metadata only, never the captured UI text.

## Evidence and cause

Read-only inspection of the existing SQLite Repository and append-only Audit confirmed a valid real Settings Observation from `emulator-5554`: package `com.android.settings`, activity `com.android.settings.homepage.SettingsHomepageActivity`, 70 elements, captured at **2026-10-02T12:44:14.705852Z**, source `android_ui_tree`, registered source `android-manual-com.android.settings`, nonempty UI-tree hash and previous Observation relationship. No extraction had been requested. A no-change Diff is valid for repeated observation of an unchanged page.

| UTC time | Persisted evidence |
| --- | --- |
| 12:44:11.343986 | Request A started; Audit records running at 11.344265. |
| 12:44:13.163000 | A different request B started while A was running. B failed with `MOBILE_BUSY`, completed at 13.163002, Audit at 13.163874; no before/after Observation. |
| 12:44:14.705852 | A captured the 70-element Observation; creation Audit at 14.725903. |
| 12:44:14.731320 | Diff Audit records `NO_MEANINGFUL_CHANGE`, 0/0/0. |
| 12:44:14.732313 | A completed successfully, both before/after IDs point to that Observation; error code/message are null. Terminal Audit at 14.732546. |

These are **two different ActionResults**. The successful result was not overwritten. `MobileHomePage`'s manual Observe button and `App.observe` lacked pending admission protection, allowing overlapping requests. The native `interact_mobile_device` command sets `manual_mobile` before work and correctly rejects a separate overlapping request. Repository action ordering uses `captured_at = started_at` descending; `MobileObservationContext` displays `actions[0]`, so later-started B remains the latest attempt even after A completes. Existing failures are truthful and remain visible.

Call chain: button → `App.observe` → typed `runtimeClient.interactMobileDevice` → Tauri `interact_mobile_device` → `DesktopManualDriver` → existing `MobileHost`/ADB → normalized Observation → Repository/Audit → terminal ActionResult → admission release → workspace response. `run_manual_interaction` performs one capture for Observe. The five-second frontend refresh only calls the workspace read command; the collection branch does not invoke Observe from an effect or a form submission. Audit proves overlapping requests, **not that a single click caused two invokes**; there was no per-click telemetry to distinguish repeated clicks from another caller. No evidence supports a stuck lock or a mislabeled successful action.

## Minimal repair

`ManualMobileFlight`, shared by manual Observe and navigation in `App`, closes admission synchronously before any await and releases it in `finally`, after the response is applied. Pending state disables Observe/check-device and conflicting navigation controls. Stop remains available. Rejected backend results are passed through unchanged. Backend concurrency protection, result ordering, Audit and historical data are unchanged. The native command delegates to the same inner implementation solely to test it with fixture state; no second runtime or database was added.

## Validation and boundary

The UI regression failed before the fix because Observe remained enabled. Deferred-promise tests cover duplicate admission, completion, IPC failure recovery and truthful backend `MOBILE_BUSY`. A native test uses fake ADB, a bounded fixture barrier, in-memory credentials and temporary SQLite: first Observe creates evidence and completes; overlapping Observe fails without stealing the lock or inventing an after-state; the runtime returns idle and a later Observe succeeds. No fixture input commands are emitted.

Human result remains **H1 FAIL / BLOCKED pending owner retest**, with the repaired development build **READY TO RETEST** only after local checks. **OBSERVATION DATA = VALID**; the original successful ActionResult is valid, while frontend action admission was defective. H2–H7 are not run. Actual Android actions by Codex, real Provider requests and signing verification during diagnosis: **0 / 0 / 0**. No codesign, packaging, release or X draft. Existing eight untracked entries are preserved. See the [daily verification record](../devlog/2026-10-02.md).
