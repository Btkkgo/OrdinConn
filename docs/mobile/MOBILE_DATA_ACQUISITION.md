# Mobile Interaction and Data Acquisition

[English](MOBILE_DATA_ACQUISITION.md) | [简体中文](MOBILE_DATA_ACQUISITION.zh-CN.md)

Issue: [#14](https://github.com/Btkkgo/OrdinConn/issues/14). Stage: **REAL MOBILE DATA ACQUISITION ACCEPTANCE = PASS (H1–H2 human; H3–H7 explicitly authorized Codex)**. M3 state, provider configuration, historical results, retry/backoff/deadline implementation, Keychain and frozen app remain unchanged.

Latest evidence: [real acceptance report](REAL_MOBILE_ACCEPTANCE.md). Anonymous manual targets now bind only when the entire structured tree and context are unchanged; M3 matching is unchanged. The initial no-Codex-action handoff below was superseded by explicit owner authorization for H3–H7.

## Data chain

`Source → Observation → Extraction → Data Object → Insight → Plan → Action → Result`

This stage implements through **Data Object**. Insight and Plan remain future interface boundaries. The manual interaction loop separately records `Observe Before → Action → Observe After → deterministic Diff`; it does not call a model or create autonomous plans.

## Reused runtime

The existing Android environment detector, ADB subprocess bridge, UI XML parser, `MobileHost`, snapshot-bound navigation, device leases, action receipt policy and append-only Audit remain authoritative. No second Android runtime or database was added. A collection projection wraps the existing capture rather than changing the historical M3 `MobileObservation` contract. Original M3 records are not migrated into collected business objects.

Production action scope remains the existing Android Emulator / Settings / preinstalled Settings search / allowed Home navigation surface. Arbitrary apps and physical-device actions remain blocked by the existing host policy. Owner-configured allowlists still apply to observations. The manual adapter adds fresh observation and unambiguous semantic target rebinding before execution, with no automatic action replay.

## Models and extraction

- `mobile-runtime::collection::MobileObservation` binds device, package/activity, capture time, screen dimensions, normalized UI elements, UI Tree hash, previous Observation and redactions. Its ID is the underlying capture Observation ID. The old M3 Observation model remains available unchanged.
- Stable `el_…` IDs derive from resource/class/bounds plus duplicate occurrence. Snapshot-local `@e…` action references are rebound by the existing semantic resolver before input; they are not durable data identities.
- Local Diff detects app/activity, addition/removal, text, selection, hierarchy/state and scroll-content changes. No meaningful change is explicit. The action's final Diff compares its initial Observation with its stable final Observation, so stabilization polling does not hide the action's effect.
- Extraction consumes only the structured Observation. It emits visible text, title, list item, button, link-like element, numeric value, timestamp-like text, status and selected item. Rule-based title/list/time guesses carry explicit lower confidence. OCR/Vision is not used or relabeled as UI Tree.
- One generic Data Object per contributing element stores normalized content, values, source identity, original Observation, extraction IDs and element IDs. Categories are text/list/metric/status/content/unknown; no market/business schema or generated summary is invented.

## Actions and bounded stability

Unified action types: `observe`, `tap`, `scroll_up`, `scroll_down`, `scroll_left`, `scroll_right`, `back`, `home`, `input_text`, `open_app`, `stop`.

Every human-triggered action persists a running/terminal ActionResult with timestamps, device, owner actor, actual before/after Observation IDs and a fixed error code. The running record is durable before input. Existing fixed ADB commands, host safety policy and action receipts are reused; raw coordinates and shell commands never come from React. Back/Home remain escape actions. Input remains the existing bounded ASCII safe-input surface, with additional rejection of sensitive targets, code/key-shaped values, destructive/account/publication targets and ambiguous selections.

After a changing action, two consecutive matching tree hashes, package/activity and element counts establish stability. Poll intervals are 120 ms, the stabilization loop budget is five seconds, and all underlying ADB operations retain their existing bounded subprocess timeouts. One capture already in flight may finish after the loop budget; this is not a hard end-to-end five-second deadline. Stop is checked before input and after each observation, including the final stable observation. Interrupted actions retain actual evidence and never fabricate an after-state. Restart fails unfinished manual actions closed without replay.

## Local repositories and provenance

Migration `0012_mobile_data_acquisition.sql` extends the existing application SQLite database with observation, action, diff, extraction, object, sightings and extraction-run tables. The repository views provide get-by-ID/recent/device/package/observation; insert operations run through validated application services. Foreign keys and immediate transactions preserve source/Observation/extraction relationships. Runtime events use the existing append-only Audit and publish only after commit.

Sources register through the existing Source Registry's manual-local policy: exact owner allowlist, Android endpoint, no authentication, disabled scheduling. No collection data is uploaded. This stage does not promote an object into validated Evidence, Insight, Signal or Trade.

Deduplication uses device, package/activity, normalized content, resource/class and object category, excluding transient Observation identity and moving bounds. Repeated Observations/extractions are evidence; a unique object enters the stream once. Sightings link every repeated capture to its actual extraction. Original object provenance is immutable and keeps its first capture time; the source detail shows occurrence count. Repository screens show bounded recent records (100 Observations/Diffs/Actions and 200 objects); exact provenance reads retrieve the original Observation even outside this window.

Source detail answers: what the data is, device and app/package/activity, capture time, method, Observation and Element IDs, original UI text/resource/bounds, extraction confidence, deduplication key and occurrence count. These structured objects and local Observation/Diff views form the next Agent context boundary.

## Sensitive data

Password/PIN/OTP/verification/private-key/seed/payment-credential labels and existing sensitive-node flags redact text, description and resource metadata before collection persistence. Redactions record `REDACTED_SENSITIVE_ELEMENT` without the value. Editable field values are excluded from business extraction. Repository validation rejects unsanitized sensitive observations. Sensitive desktop captures hide the screenshot; screenshots otherwise remain memory-only and are not extraction inputs. Label-based detection is conservative, not a universal detector for unlabeled secrets; human acceptance must use public, non-account surfaces.

## Initial owner acceptance procedure (historical)

Code and automated fixtures stop before real Android operation. The owner may start the development app with `npm run desktop:dev` after ensuring internal M3 acceptance opt-ins are disabled. This task did not launch the app, initialize the production database, configure a provider, or change the retained frozen bundle.

1. Connect the existing dedicated emulator; configure the existing Settings allowlist if needed.
2. Observe and inspect the UI Tree. Select an element and review its text/role/bounds/clickable/scrollable/editable/resource fields.
3. Manually Tap, inspect the before/after result and Diff, then manually Scroll.
4. Extract the captured page. Verify real objects appear in the Data Stream and filter by type.
5. Open an object and verify device/package/activity/time/method/Observation/Element and original UI evidence.
6. Repeat Observe/Extract; confirm occurrence count increases without duplicated new stream entries. Stop and verify terminal state.

Do not use real accounts, passwords/codes, messaging, publication, purchases/payments, destructive settings or APK installation. Codex executes no real Android action for acceptance.

## Validation boundary

Fixture/unit/repository/UI tests and local Rust/frontend builds are distinct from human Android/Tauri acceptance. The supported `npm run desktop:build` wrapper invokes prohibited `codesign` and preserves an existing frozen M3 bundle; it was **NOT RUN / BLOCKED BY THIS TASK'S NO-SIGNING BOUNDARY**. No direct Tauri acceptance build bypass was used. Signing, notarization, live Provider/Planner and Gemini checks are **NOT RUN**. X Draft = **NONE**.
