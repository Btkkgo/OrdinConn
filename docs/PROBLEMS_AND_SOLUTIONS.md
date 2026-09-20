# Problems and Solutions

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

Install OpenJDK 21 and the official Android command-line tools, then create `OrdinConn_M1_5` from the Android 36 Google APIs ARM64 image. Keep ordered environment discovery, bounded boot readiness, and the ten-check report that distinguishes `FAIL` from `BLOCKED`.

### Verification

The AVD cold-booted to `adb` online and `sys.boot_completed=1`. The production gate passed all ten checks, including real frame/UI capture, snapshot parse, refs, observation, typed IPC, persistence, audit events, and session shutdown. A separate real password-node test also passed.

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

Use one ordered resolver for diagnostics and observation, put process work on a blocking pool, enforce total deadlines, drain stdout/stderr concurrently, and terminate timed-out children.

### Verification

Regression tests covered standalone ADB paths, configured-SDK consistency, a deliberately hung fixture, and a 256 KiB stdout payload. Real approximately 190 KiB PNG frames then traversed the same helper successfully.

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

Read the full window dump for focus detection. Reject only nodes whose bounds cannot form a valid non-negative rectangle while preserving the remainder of the snapshot.

### Verification

Regression tests were observed failing before the fixes and passing afterward. The real Settings snapshot produced 70 valid sanitized elements while two reversed-bound platform nodes were excluded; the full ten-check gate passed.

### Reusable Lesson

Treat operating-system diagnostics as versioned external schemas. Fail closed on missing identity, but isolate malformed optional nodes instead of discarding an otherwise valid observation.
