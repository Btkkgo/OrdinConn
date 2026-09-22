# Problems and Solutions

[English](PROBLEMS_AND_SOLUTIONS.md) | [简体中文](PROBLEMS_AND_SOLUTIONS.zh-CN.md)

## Problem 001 — Inference can look like source evidence

### Context

An Agent can produce a convincing explanation without having a factual source.

### Symptom

A generated interpretation could otherwise satisfy a publication path intended for externally grounded information.

### Root Cause

Inference and Evidence were not safe to treat as interchangeable inputs.

### Failed Attempts

No discarded implementation is recorded in the repository. The risk was handled as a foundational domain constraint.

### Final Solution

Evidence records carry factual level and source lineage. Signal publication requires at least one linked Evidence item that is not `MODEL_INFERENCE`.

### Verification

The Evidence and Signal suites reject inference-only candidates and retain contradicting evidence.

### Reusable Lesson

Generated confidence is not provenance. Encode that difference in types and gates rather than relying on prompt wording.

## Problem 002 — Source schemas drift

### Context

Public feeds and exchange payloads can change shape without warning.

### Symptom

A permissive parser could accept incomplete or misinterpreted data and feed it into a strategy.

### Root Cause

Network success does not prove semantic compatibility.

### Failed Attempts

The history contains a dedicated follow-up fix, `c71ce1a`, showing that schema drift required a stricter boundary after the initial collection pipeline.

### Final Solution

Collectors fingerprint shape, validate required fields and timestamps, update source health, and fail closed without a mock fallback on the real path.

### Verification

Collector tests cover malformed or missing timestamps, schema drift, and normalization errors.

### Reusable Lesson

Treat external schemas as versioned contracts even when the provider does not publish a version.

## Problem 003 — A strategy can run before its baseline is valid

### Context

Rolling metrics need enough fresh history, and live data may arrive late or out of order.

### Symptom

Using defaults for missing history could trigger a false high-confidence signal.

### Root Cause

Computation availability and decision readiness are different states.

### Failed Attempts

No discarded algorithm is recorded; the Phase 2 design made readiness explicit before broadening live collection.

### Final Solution

`WARMING_UP`, `MISSING_INPUT`, `STALE_INPUT`, `SCHEMA_ERROR`, and `INSUFFICIENT_HISTORY` are auditable results that do not produce an ordinary Candidate.

### Verification

Rolling-history and derivatives tests cover capacity, ordering tolerance, stale data, missing metrics, restored buckets, and readiness failure.

### Reusable Lesson

Make “not ready” a first-class result, not an exception or a zero-filled input.

## Problem 004 — Android integration could not be proven locally

### Context

M1.5 required a real Android SDK, ADB, Emulator, AVD, and online device.

### Symptom

The explicit smoke reported three failed prerequisites and seven blocked downstream checks.

### Root Cause

The required Android environment was absent from the machine.

### Failed Attempts

The first acceptance run intentionally failed closed rather than fabricating device evidence. A later authorized run installed only the required official command-line components, stable ARM64 image, and one dedicated AVD.

### Final Solution

Install OpenJDK 21 and the official Android command-line tools, then create `OrdinConn_M1_5` from the Android 36 Google APIs ARM64 image. Keep ordered environment discovery, bounded boot readiness, and the capture report that distinguishes `FAIL` from `BLOCKED`; verify real GUI IPC separately.

### Verification

The AVD cold-booted to `adb` online and `sys.boot_completed=1`. The production capture gate passed real frame/UI capture, snapshot parse, refs, observation, persistence, audit events, workspace projection, and session shutdown. The packaged desktop separately passed real typed IPC and frontend acceptance. A separate real password-node test also passed.

### Reusable Lesson

Preserve the first blocked result, then replace it only with a named real-environment rerun. Installing a dependency is not acceptance; the full production path still has to pass.

## Problem 005 — External mobile commands can hang or diverge

### Context

ADB and Emulator are external processes selected from several possible SDK locations.

### Symptom

An unresponsive tool could block the desktop host, diagnostics could select a different ADB than observation, and a large PNG could fill a child-process pipe before exit.

### Root Cause

Initial process execution lacked a hard deadline and environment selection was duplicated. The first bounded implementation also waited for child exit before draining stdout, which deadlocked once the real screencap exceeded the pipe buffer.

### Failed Attempts

Independent review of the first M1.5 implementation identified both gaps before publication.

### Final Solution

Use one ordered resolver for diagnostics and observation, put process work on a blocking pool, enforce total deadlines, drain stdout/stderr through nonblocking pipes, and place each owned command in its own process group. On timeout, terminate only that group; when the direct child exits, return without waiting for unrelated descendants that inherited a pipe.

### Verification

Regression tests covered standalone ADB paths, configured-SDK consistency, a deliberately hung fixture, descendants that retain inherited output pipes after success or timeout, and a 256 KiB stdout payload. Real approximately 190 KiB PNG frames then traversed the same helper successfully.

### Reusable Lesson

Subprocess timeouts must bound the process itself, not only the caller waiting for its result. Piped output must be drained before the producer can block.

## Problem 006 — Android 16 changed real window and UI-tree behavior

### Context

The production capture path was originally validated with controlled ADB fixtures before an Android 16 Emulator was available.

### Symptom

The real observation could not identify the foreground application from `dumpsys window windows`, and then failed the whole UI tree because two platform nodes reported reversed bounds.

### Root Cause

Android 16 exposes `mCurrentFocus` in the full `dumpsys window` output but not in the narrower `windows` section. UIAutomator can also emit off-screen platform nodes whose lower y coordinate is smaller than the upper y coordinate.

### Failed Attempts

The unchanged fixture-compatible commands passed unit tests but failed against the real API 36 image. Retrying the same capture did not change either output shape.

### Final Solution

Read the full window dump for focus detection. Discard malformed non-sensitive platform nodes while preserving the remainder of the snapshot, but fail the whole parse when the malformed node is sensitive or financial so the screen cannot be misclassified as ordinary.

### Verification

Regression tests were observed failing before the fixes and passing afterward, including a malformed password node and a financial-action classifier. The real Settings snapshot produced 70 valid sanitized elements while two reversed-bound non-sensitive platform nodes were excluded; the full gate passed.

### Reusable Lesson

Treat operating-system diagnostics as versioned external schemas. Fail closed on missing identity, but isolate only malformed non-sensitive optional nodes instead of discarding an otherwise valid observation.

## Problem 007 — Workspace serialization was not proof of Tauri IPC

### Context

M1.5 requires the real Rust Mobile Runtime → Tauri command/event → React frontend chain, including status, error, start, observation, and stop.

### Symptom

The first real-smoke harness called the persistence/projection helper directly and labeled the result `TAURI_IPC`, even though no frontend invocation or stop command was exercised.

### Root Cause

A useful lower-level integration test was given a broader acceptance label than its actual boundary, and the product exposed observation but no explicit frontend stop control.

### Final Solution

Rename the automated gate to `WORKSPACE_PROJECTION`, add a registered `stop_mobile_session` Tauri command and localized React control, project active/disconnected status from the host, and keep GUI acceptance as a separate real-desktop check.

### Verification

The packaged application first displayed the expected empty-allowlist error. After allowlisting `com.android.settings`, it showed the real emulator, package, verified observation, frame, and 70 UI elements. Stop returned the UI to `Disconnected`; SQLite recorded `mobile.session_started`, `mobile.snapshot`, `mobile.observation`, and `mobile.session_ended` for one session; ADB still reported the emulator online.

### Reusable Lesson

Serialization at a command helper boundary is not evidence that the UI invoked the Tauri command. Name lower-level gates precisely and verify GUI-only acceptance in the real desktop application.

## Problem 008 — Cold parallel fixture startup exhausted success-test budgets

### Problem and Observed Behavior

Issue #4 tracked intermittent failures in four AVD/subprocess tests. Earlier runs included 21/24 and 24/28 desktop results followed by green serial or workspace reruns. On 2026-09-22, the first of ten fresh default-parallel desktop runs again failed those same four cases at 24/28; the next nine passed. The first cold run took 1.86 seconds inside the test binary, while warm runs took about 0.5 seconds.

### Why Serial Passed and Parallel Failed

The success-path fixtures launched several short-lived shell commands but used one-second deadlines as if process startup latency were a functional contract. Serial or warm runs usually completed inside that budget. Cold parallel startup consumed enough scheduling time that unrelated success-path checks reached their deadlines. Unique `TempDir` SDK/AVD roots, script paths, and process groups were already used; the audit found no fixed port, global environment mutation, or shared fixture path. The race was between test-only wall-clock budgets and external process startup under contention, not a shared AVD state file.

### Root Cause Evidence

A new regression starts four independently named AVD fixtures at a barrier, each with its own SDK, ADB script, Emulator script, and AVD home. Controlled 200 ms latency in the fake tools made the original one-second success budget fail with `AvdBootTimeout`; a three-second test-only budget passed. This reproduces the timing mechanism without changing production command deadlines.

### Failed Approaches

The earlier green reruns and serial execution were observations, not repairs. This task did not adopt global serialization, retry-until-pass, ignored tests, a production timeout increase, or extra production sleeps.

### Correct Fix

Give successful large-output and inherited-pipe fixture calls a bounded two-second test budget, and successful AVD lifecycle fixture calls a bounded three-second test budget. Keep the deliberate 30 ms command-timeout and 50 ms no-boot checks unchanged. The new concurrent fixture regression proves independent roots and idempotent logical shutdown; production code and process-group cleanup remain unchanged.

### Regression Coverage

The four existing cases each passed 20 isolated pre-change runs, and related tests passed 20 eight-thread and 20 32-thread warm runs, confirming the cold-start pattern rather than proving it absent. After the change, default-parallel desktop passed 20/20 runs (29/29 tests each), the full workspace passed 10/10 runs, and an eight-thread desktop run passed 29/29. A first real Android smoke failed closed because the cold-boot foreground Launcher was outside the Settings allowlist; after opening Settings on the dedicated AVD, the unchanged smoke passed all ten checks, including frame, UI tree, `MobileObservation`, and logical shutdown.

### Reusable Lesson

A success test should validate behavior, not accidentally benchmark cold process startup. Keep short timeout assertions in dedicated failure-path tests, and use repeat runs plus controlled latency to distinguish deterministic coverage from a lucky green rerun.
