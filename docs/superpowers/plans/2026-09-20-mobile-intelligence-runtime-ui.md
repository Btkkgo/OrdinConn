# Mobile Intelligence Runtime and UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver OrdinConn M0, M1 observe-only Android Emulator runtime, and the linked Home/Warehouse/Settings UI without enabling mobile actions.

**Architecture:** A new transport-independent `mobile-runtime` crate owns mobile sessions, semantic snapshots, frame metadata, sensitive redaction, and observations. The Tauri host is the only Android/ADB adapter, `ordinconn-app` persists sanitized captures and projects the unified intelligence feed, and React consumes typed IPC contracts through a three-surface UI.

**Tech Stack:** Rust 2024, Tokio, serde, quick-xml, SHA-256, SQLite/sqlx migrations, Tauri 2 typed commands/events, React 19, TypeScript 5.9, Vitest, CSS.

**Spec:** `docs/mobile/MOBILE_INTELLIGENCE.md`

## Global Constraints

- Implement only M0, M1, and UI Shell; do not enter M2.
- Mobile runtime is observe-only: no Tap, Swipe, Type, Back, Home, Open App, Search, or coordinate actions.
- Android Emulator is the first adapter; upper contracts remain platform-independent.
- React never invokes ADB.
- Semantic UI tree is primary; screenshot/OCR/vision are not used to guess action coordinates.
- Application access defaults to deny-all and secure content is redacted.
- Frames remain in a bounded in-memory ring buffer and are not continuously recorded.
- MobileObservation is not Evidence and cannot bypass Source Registry, Strategy, or Evidence Gate.
- Keep existing features in the codebase while exposing only Home, Warehouse, and Settings in primary navigation.
- Do not display the product name, version, or logo text; show only the centered application icon.
- Preserve the existing purple, black, and yellow brand while making the primary workspace black.
- Do not modify or commit the user-owned `AGENTS.md` change.
- Do not push.

## Review Focus

- A missing Android SDK, missing ADB binary, offline emulator, or malformed ADB output must produce an explicit unavailable/error state rather than a mock success; Task 2 exercises each parser/error boundary.
- An empty allowlist or package not in the allowlist must prevent capture before UI content is persisted; Task 2 tests both cases.
- Password, OTP, wallet, order, transfer, and signing content must be redacted and must never enter runtime event payloads; Task 2 tests sanitized snapshots and audit payloads.
- Old element references must be snapshot-scoped and invalid after a new snapshot; Task 2 tests distinct snapshot IDs and regenerated references.
- Text scaling and saved intelligence state must survive reload without scaling the phone viewport; Task 3 tests preference resolution and separated CSS variables.

---

### Task 1: M0 Documentation and Typed Contracts

**Files:**
- Create: `docs/mobile/MOBILE_INTELLIGENCE.md`
- Create: `docs/mobile/MOBILE_UI.md`
- Create: `docs/mobile/DEVICE_RUNTIME.md`
- Create: `docs/mobile/MOBILE_OBSERVATION.md`
- Create: `docs/mobile/MOBILE_ACTION_PROTOCOL.md`
- Create: `docs/mobile/MOBILE_SECURITY.md`
- Create: `docs/mobile/APP_SKILLS.md`
- Create: `docs/mobile/VIDEO_REFERENCE_ANALYSIS.md`
- Modify: `docs/PRODUCT_BASELINE.md`
- Modify: `packages/contracts/src/index.ts`
- Modify: `packages/contracts/src/index.test.ts`

**Interfaces:**
- Consumes: existing `SignalDto`, `EvidenceDto`, `RuntimeEventEnvelope`, and the approved mobile specification.
- Produces: `MobileDeviceSessionDto`, `MobileElementDto`, `MobileUiSnapshotDto`, `MobileFrameDto`, `MobileObservationDto`, `IntelligenceItemDto`, `WarehouseEntryDto`, `StrategyDefinitionDto`, `MobileRuntimeSettingsDto`, `MobileWorkspaceDto`, `MobileRuntimeStatus`, and mobile runtime event-family parsing.

- [ ] **Step 1: Add failing contract tests**

Add tests that construct a complete `MobileWorkspaceDto`, assert that mobile observations remain `observation_only`, assert no action-capability field exists, and parse a `mobile.snapshot` event with family `mobile`.

- [ ] **Step 2: Run the contract test and confirm RED**

Run: `npm test --workspace @ordinconn/contracts`

Expected: TypeScript compilation fails because the mobile DTO exports and `mobile` event family do not exist.

- [ ] **Step 3: Add the minimum typed contracts and baseline amendment**

Define literal unions and DTOs matching the field names in the specification. Extend `EVENT_FAMILIES` with `mobile`. Amend the product baseline to name Mobile Intelligence as an authorized M0/M1 extension while retaining public-data, approval, and no-real-money constraints.

- [ ] **Step 4: Run contract tests and typecheck**

Run: `npm test --workspace @ordinconn/contracts && npm run typecheck --workspace @ordinconn/contracts`

Expected: all contract tests pass and TypeScript exits zero.

- [ ] **Step 5: Commit Task 1**

Stage only the documentation, plan, product baseline, and contract files. Commit message: `docs: define mobile intelligence contracts`.

### Task 2: M1 Observe-only Runtime, Persistence, and ADB Adapter

**Files:**
- Create: `crates/mobile-runtime/Cargo.toml`
- Create: `crates/mobile-runtime/src/lib.rs`
- Create: `crates/ordinconn-app/migrations/0005_mobile_intelligence.sql`
- Create: `crates/ordinconn-app/src/mobile_intelligence.rs`
- Create: `apps/desktop/src-tauri/src/mobile.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/ordinconn-app/Cargo.toml`
- Modify: `crates/ordinconn-app/src/lib.rs`
- Modify: `crates/ordinconn-app/src/services.rs`
- Modify: `crates/ordinconn-app/src/events.rs`
- Modify: `crates/ordinconn-app/tests/database.rs`
- Modify: `crates/agent-runtime/src/lib.rs`
- Modify: `crates/tool-runtime/src/lib.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/state.rs`
- Modify: `apps/desktop/src-tauri/src/commands.rs`

**Interfaces:**
- Consumes: Task 1 DTO field names, existing SQLite/runtime event conventions, `PageContext`, `ToolRegistry`, and `SignalView`.
- Produces: platform-independent domain constructors, `MobileHost::observe`, `AppRuntime::record_mobile_capture`, `AppRuntime::mobile_workspace_data`, `AppRuntime::set_warehouse_entry`, `AppRuntime::create_mobile_research_task`, `AppRuntime::save_mobile_settings`, `AppRuntime::set_strategy_enabled`, and Tauri commands `get_mobile_workspace`, `observe_mobile_device`, `set_warehouse_entry`, `create_mobile_research_task`, `save_mobile_settings`, and `set_strategy_enabled`.

- [ ] **Step 1: Write failing mobile-domain tests**

Test literal fixture nodes for sequential `@e1` references, snapshot-scoped regeneration, password/OTP/order redaction, visible-fact deduplication, frame hashing, observation source locators, and a six-frame insert into a five-frame ring buffer.

- [ ] **Step 2: Run the new crate tests and confirm RED**

Run: `cargo test -p mobile-runtime`

Expected: Cargo reports that package `mobile-runtime` or the tested symbols do not exist.

- [ ] **Step 3: Implement transport-independent mobile domain types**

Implement serde-enabled enums and structures, `MobileUiSnapshot::from_elements`, `MobileFrame::from_png`, `MobileObservation::from_snapshot`, sensitive-state classification, `GenericAndroidSkill`, and a bounded `ScreenFrameBuffer`. Do not define action execution methods.

- [ ] **Step 4: Run mobile-domain tests GREEN**

Run: `cargo test -p mobile-runtime`

Expected: all mobile-runtime tests pass.

- [ ] **Step 5: Write failing persistence and application-service tests**

Extend database tests to require `mobile_device_sessions`, `mobile_ui_snapshots`, `mobile_observations`, and `warehouse_entries`. Add integration tests that record a sanitized fixture capture, query a unified feed item with `observation_only`, favorite and save its object reference, persist settings, create a budgeted research task, toggle a strategy, inject observation context into an Agent request, and confirm only mobile identifiers/hashes/counts enter audit events.

- [ ] **Step 6: Run focused application tests and confirm RED**

Run: `cargo test -p ordinconn-app --test database mobile -- --nocapture && cargo test -p agent-runtime mobile_context && cargo test -p tool-runtime mobile`

Expected: tests fail because migrations, application services, context fields, and observe-only tool definitions are absent.

- [ ] **Step 7: Implement migration and application services**

Persist sanitized sessions, snapshots, observations, warehouse references, research budgets, and settings. Project mobile observations and existing Evidence into one `IntelligenceItemView` list. Map related Signals by asset without creating a new Signal. Append `mobile.session_started`, `mobile.snapshot`, `mobile.observation`, and block/failure events using only sanitized metadata.

- [ ] **Step 8: Implement observe-only tool and Agent context contracts**

Extend `PageContext` with optional mobile observation/source fields. Register read-only mobile observation, UI-tree, frame, and inspection tools. Keep every mobile action tool explicitly unavailable.

- [ ] **Step 9: Run application tests GREEN**

Run: `cargo test -p ordinconn-app --test database && cargo test -p agent-runtime && cargo test -p tool-runtime`

Expected: all focused tests pass.

- [ ] **Step 10: Write failing ADB parser and adapter tests**

Use literal `adb devices`, `dumpsys`, `wm size`, and UIAutomator XML fixtures. Test missing ADB, no online emulator, malformed size, absent focus, an empty allowlist, a disallowed package, secure-node redaction, and successful capture construction. Add an environment-gated `ORDINCONN_MOBILE_SMOKE=1` test that uses the real adapter.

- [ ] **Step 11: Run desktop Rust tests and confirm RED**

Run: `cargo test -p ordinconn-desktop mobile -- --nocapture`

Expected: tests fail because the ADB adapter, parsers, host state, and commands do not exist.

- [ ] **Step 12: Implement the ADB adapter and typed Tauri commands**

Resolve ADB from Android SDK environment paths, the conventional macOS SDK path, or process PATH. Detect online `emulator-*` devices; capture current package/activity, size, sanitized UIAutomator XML, and PNG; enforce allowlist before persisting app content; retain five frames in memory. Register read-only IPC commands and mobile events. Do not add an action command.

- [ ] **Step 13: Run desktop Rust tests GREEN**

Run: `cargo test -p ordinconn-desktop mobile -- --nocapture`

Expected: parser/adapter tests pass; the real smoke test exits without device access unless `ORDINCONN_MOBILE_SMOKE=1` is explicitly set.

- [ ] **Step 14: Run the Task 2 suite and commit**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

Expected: formatting, strict Clippy, and every Rust test pass.

Stage only Task 2 files and commit message: `feat: add observe-only mobile runtime`.

### Task 3: Mobile Intelligence UI Shell

**Files:**
- Create: `apps/desktop/src/pages/mobileIntelligence.ts`
- Create: `apps/desktop/src/pages/mobileIntelligence.test.ts`
- Create: `apps/desktop/src/pages/MobileHomePage.tsx`
- Create: `apps/desktop/src/pages/WarehousePage.tsx`
- Create: `apps/desktop/src/components/MobileDeviceView.tsx`
- Create: `apps/desktop/src/components/DataDetailModal.tsx`
- Create: `apps/desktop/src/runtime/preferences.ts`
- Create: `apps/desktop/src/runtime/preferences.test.ts`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/components/AppShell.tsx`
- Modify: `apps/desktop/src/components/Navigation.tsx`
- Modify: `apps/desktop/src/components/Navigation.test.tsx`
- Modify: `apps/desktop/src/pages/ModelsPage.tsx`
- Modify: `apps/desktop/src/pages/SettingsPage.tsx`
- Modify: `apps/desktop/src/runtime/client.ts`
- Modify: `apps/desktop/src/runtime/state.ts`
- Modify: `apps/desktop/src/runtime/state.test.ts`
- Modify: `apps/desktop/src/i18n/en.ts`
- Modify: `apps/desktop/src/i18n/zh-CN.ts`
- Modify: `apps/desktop/src/i18n/index.test.ts`
- Modify: `apps/desktop/src/styles.css`
- Modify: `apps/desktop/src/styles.test.ts`

**Interfaces:**
- Consumes: Task 1 TypeScript DTOs and Task 2 Tauri commands/events.
- Produces: three-route primary navigation, linked feed/device/signal Home workspace, Data Detail with discussion/research/favorite/save actions, searchable Warehouse, model/strategy/language/text-size/mobile Settings, and mobile-context Agent turns.

- [ ] **Step 1: Write failing pure UI-state tests**

Test feed sorting and source labels, related-signal matching by asset, explicit `No signal yet` readiness, Warehouse search/filtering, saved/favorite projection, complete mobile Agent context, and text-scale normalization for 90/100/110/120 only.

- [ ] **Step 2: Write failing rendered navigation and workspace tests**

Use `renderToStaticMarkup` with complete fixtures to assert that navigation exposes exactly Home/Warehouse/Settings, contains only the icon in its brand block, does not render product/version copy, and that Home renders feed, mobile live view, and related-signal landmarks.

- [ ] **Step 3: Run desktop tests and confirm RED**

Run: `npm test --workspace @ordinconn/desktop`

Expected: tests fail because the mobile UI helpers, routes, components, and text preferences do not exist.

- [ ] **Step 4: Implement typed runtime client and state projection**

Add typed clients for all Task 2 commands. Extend Agent context construction with the selected observation, source, and evidence IDs. Merge runtime-event updates without replacing the full workspace snapshot.

- [ ] **Step 5: Implement Home and Data Detail**

Build the three linked columns and minimal runtime status bar. Render semantic element overlays only when a current snapshot and frame dimensions match. Data Detail exposes full content, source, observation/evidence lineage, quality, related signals, discussion, research, favorite, tags, and warehouse actions. Discussion automatically includes current data context.

- [ ] **Step 6: Implement Warehouse and Settings**

Build searchable/filterable saved-data views. Integrate the existing model provider controls into Settings, add strategy enable/disable, existing language selection, persistent text scaling, and Mobile Runtime status/allowlist/budget configuration.

- [ ] **Step 7: Simplify shell and apply brand rules**

Expose only Home, Warehouse, and Settings in the primary rail. Keep the icon centered without name/version. Remove the persistent Agent Dock and top marketing/context chrome from the primary shell while leaving their source modules available. Use black main surfaces, restrained purple ambience, yellow selection, responsive three-column collapse, accessible buttons/dialogs, and a separate unscaled phone viewport variable.

- [ ] **Step 8: Run desktop tests and typecheck GREEN**

Run: `npm test --workspace @ordinconn/desktop && npm run typecheck --workspace @ordinconn/desktop`

Expected: every desktop test passes and TypeScript exits zero.

- [ ] **Step 9: Run build and browser-visible validation**

Run: `npm run build`

Expected: TypeScript and Vite production build exit zero.

Launch the real Tauri development app, confirm the running binary belongs to this checkout, and inspect Home, a Data Detail interaction, Warehouse, language/text scaling, and Settings. If Android tooling is absent, verify and display the explicit unavailable state rather than claiming a live frame.

- [ ] **Step 10: Run final project verification and commit**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && npm test && npm run typecheck && npm run build`

Expected: all commands exit zero.

If Android tooling and an online emulator are available, also run: `ORDINCONN_MOBILE_SMOKE=1 cargo test -p ordinconn-desktop real_emulator_smoke -- --nocapture`

Stage only Task 3 files and commit message: `feat: integrate mobile intelligence workspace`.
