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

The project did not install tools or fabricate a device because the phase specification prohibited both.

### Final Solution

Add ordered environment discovery, actual tool-path reporting, existing-AVD lifecycle checks, bounded boot readiness, and a ten-check report that distinguishes `FAIL` from `BLOCKED`.

### Verification

Controlled fixtures pass for discovery, filtering, parsing, timeouts, allowlisting, persistence, and shutdown. The real smoke remains failed, which is the truthful outcome.

### Reusable Lesson

An integration test that cannot reach its dependency must say “blocked,” not quietly become a unit test.

## Problem 005 — External mobile commands can hang or diverge

### Context

ADB and Emulator are external processes selected from several possible SDK locations.

### Symptom

An unresponsive tool could block the desktop host, and diagnostics could select a different ADB than observation.

### Root Cause

Initial process execution lacked a hard deadline and environment selection was duplicated.

### Failed Attempts

Independent review of the first M1.5 implementation identified both gaps before publication.

### Final Solution

Use one ordered resolver for diagnostics and observation, put process work on a blocking pool, enforce total deadlines, and terminate timed-out children.

### Verification

Regression tests covered standalone ADB paths, configured-SDK consistency, and a deliberately hung fixture before the full product suite passed.

### Reusable Lesson

Subprocess timeouts must bound the process itself, not only the caller waiting for its result.
