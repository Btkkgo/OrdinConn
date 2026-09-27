# Realtime Workbench Acceptance — Issue #11

[English](WORKBENCH_ACCEPTANCE.md) | [简体中文](WORKBENCH_ACCEPTANCE.zh-CN.md)

> Current Phase 5: bounded Planner/Executor integration is implemented. Live model acceptance is **BLOCKED_MODEL_NOT_CONFIGURED** (Production Provider Count **0**); Full Autonomous M3 is **NOT_COMPLETE**. Earlier Phase 2–4 validation records below remain historical evidence. See the [Phase 5 closeout record](M3_PHASE5.md) for current validation.


> This is the earlier workbench acceptance record. [M3 Phase 4](M3_EXECUTOR.md) now separately verifies explicit single-step device execution. Full autonomous M3 remains NOT_COMPLETE; operator acceptance remains pending.

## Historical scope and outcome

Home follows owner-provided Reference A for appearance and Reference B for structure. Warehouse, Settings, contextual detail/discussion, and the M2 manual inspector remain available. The local changes are based on `cc1d0c9` and are not committed or pushed.

**Overall: BLOCKED for complete custom-goal execution; implemented UI and manual command integration remain separately reviewable.** Existing research tasks save pending goals, not executable autonomous plans. The workbench explicitly displays `NOT_IMPLEMENTED` for autonomous mobile planning/execution. M3 is NOT STARTED. Issue #10 owner experience is still PENDING USER ACCEPTANCE.

## Real data and runtime

- Fresh ADB diagnostics determine connection and device name. Stale stored sessions do not imply a connected device. Observe refreshes the real frame, semantic snapshot, structured observation, persistence, and audit through typed IPC.
- Counts aggregate deduplicated objects in the loaded runtime window (up to 200 observations and 200 Evidence records), using local-calendar today/yesterday. They are not all-database totals. Explicit data types select categories; mobile observations and unspecified Evidence fall under Other. Missing comparison baselines do not produce invented percentages.
- Extract reuses the existing sanitized observation parser/persistence. Collect observes first, requests an existing snapshot-bound upward finger swipe, and requires the existing post-observation receipt to be executed/VERIFIED. A sensitive screen is denied; no fake collected success is returned.
- Stop interrupts further runner steps and closes only the logical session. It neither kills the AVD nor removes observations. Database failures surface as failures. A regression reproduced SQLite writer contention before the fix; acquiring `BEGIN IMMEDIATE` before reading now waits within the existing busy timeout and preserves atomic session/audit persistence.
- Research goals are persisted with a sanitized task-created event containing metadata only. Only mobile research tasks are projected; ordinary tasks with empty migration defaults are excluded. Seven plan states are rendered or hidden correctly. Dismissal is a local view preference, not a persisted cancellation.
- No frontend ADB bridge, new model adapter, autonomous loop, approval token rewrite, or expanded production action policy was introduced. Sensitive/financial targets remain denied. Critical messages, posting, deletion, account operations, and real funds are outside this manual action surface.
- Development fixture requires both development mode and explicit `workbenchFixture=1`. Its label is visible and its command controls cannot write fixture observations/tasks. The production JS excludes the fixture entry and sample data.

## Visual audit

The correction loop covered layout, card density, blue selection, font hierarchy, phone fit, command rows, input, empty plan area, and footer. Full native Tauri fixture captures are retained separately from real-device captures. Stage Manager thumbnails were rejected as evidence.

| Checklist | Review |
| --- | --- |
| Header title/subtitle hierarchy; Android status at upper right | Reviewed |
| Three panel widths, aligned tops/bottoms, center emphasis | Reviewed |
| Panel/card borders, round corners, consistent card gaps | Reviewed |
| Values at right, selected blue border, current category text | Reviewed |
| Centered phone, preserved real image aspect, reasonable space | Reviewed |
| Agent state at upper right; 2×2 text buttons below phone | Reviewed |
| Input + Execute row; roomy right panel | Reviewed |
| Observe / Capture / Plan / Approval legend order | Reviewed |
| No new icons, category colors, charts, logos, old dashboard | Reviewed |

Remaining intentional differences: preserved narrow Home/Warehouse/Settings rail and native macOS title bar; real Pixel 8 content preserves its narrower aspect inside the reference-proportioned phone surface; device-tools disclosure retains the manual inspector; live counts, device content, pending goals, and errors differ from demonstration data. These are disclosed rather than filled with fixture data.

## Validation and retained failures

Final command results and screenshot dimensions are recorded below after the actual packaged application checks. Tests are not owner UX acceptance.

Earlier attempts are retained: software-emulator crash/EMULATOR_READY failure, an interrupted real M2 action during packaging, and a later M2 search-transition capture returning no post-frame. Real action gates have a separate successful retry. A fresh default-parallel Rust run also reproduced an existing concurrent AVD lifecycle fixture timeout during release compilation. No production deadline or unrelated fixture was weakened.

## Delivery boundary

Commit NONE. Push NONE. No X draft/publication. Local Issue #11 changes remain open for review; no full autonomous-engine acceptance or owner UX acceptance is claimed.

## Final verification — 2026-09-27

| Gate | Result |
| --- | --- |
| TypeScript tests | PASS: 55 desktop + 5 contracts |
| TypeScript typecheck | PASS |
| Rust workspace, serial | PASS: 165; explicit gated real tests are reported separately |
| Rust workspace, default parallel | Earlier PASS; two later FAIL attempts in the existing concurrent AVD fixture; not hidden |
| Writer-contention Stop regression | FAIL before, PASS after; persisted shutdown idempotency also PASS |
| Explicit real Observe gate | PASS on final Rust code |
| Explicit real M2 navigation gate | PASS on final Rust code after public Settings reset; earlier failures retained |
| Packaged Tauri real Observe | PASS: frame + structured observation + audit |
| Packaged Tauri Collect | PASS: real swipe, VERIFIED receipt, completed result linked to one object |
| Packaged Tauri Stop | PASS: ended session, exactly one end event, AVD still booted |
| Enter custom goal | PASS for pending persistence/event; autonomous execution NOT_IMPLEMENTED |
| Production fixture exclusion | PASS |
| Public security/document/diff gates | PASS before final documentation closeout; final check recorded in DevLog |

The final Rust checks are reused for subsequent frontend-only phone-surface/CSS corrections; Rust logic and dependencies are identical. Full default-parallel determinism is not claimed. The complete custom autonomous loop remains BLOCKED despite working manual collection.

## Native screenshot closeout

Reference A visual review PASS; Reference B structure review PASS. Six full native screenshot reviews/corrections were performed; rejected thumbnail captures are excluded. This is manual reference comparison, not an automated pixel-diff score. Final production screenshot: `artifacts/ordinconn-workbench-live-final.jpg`, 1448×1086. Default desktop window is 1448×1086; short windows preserve footer visibility and permit panel scrolling. The real image is letterboxed inside the reference phone aspect; its Inspector is bound to the same fitted image surface. The phone remains about 9% smaller than Reference A. Native title bar, narrow preserved navigation, tool disclosure, and genuine content/status differences remain disclosed.

Frontend/Rust/Tauri desktop build PASS; final packaged native launch/Observe PASS. Production fixture exclusion, public security gate, public document validation, rustfmt, and diff check PASS. The first public security attempt blocked private diagnostic logs containing local paths; those logs and the temporary preview bundle were retained outside the public checkout, without modifying scanner rules.

### Retained image provenance

The listed JPEGs are preserved existing local artifacts and are excluded from the Phase 5 commit. New private real-device captures are not published.

- `artifacts/ordinconn-workbench-fixture-02.jpg`: 1448×1086; SHA-256 `1d6c15572889b5effaf73f0455931dafcbe9e00835078a52b8c49ba42fa8a497`.
- `artifacts/ordinconn-workbench-fixture-final.jpg`: 1444×1085; SHA-256 `924ad0a7de79267da0891737f8e05f6976a0686b5a038ccf84d5358e473358de`.
- `artifacts/ordinconn-workbench-live-final.jpg`: 1448×1086; SHA-256 `a2967e891821c60edf386c57147b6074c7429a21ed1db92a09300762111846af`.
- `artifacts/ordinconn-workbench-live-iteration-04.jpg`: 1440×920; SHA-256 `ac010b38ed80e5aefc4f335368d3b1b271218a19344e7567ec6e08f47ba5d316`.
