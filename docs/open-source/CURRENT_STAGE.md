# Current Stage

- Date: 2026-09-20
- Version: 0.1.0
- Branch: `codex/mobile-intelligence-runtime`
- Product source baseline: `144c87f`
- Stage: Mobile Intelligence M1.5 environment validation
- Gate status: **Blocked before M2**

## Goal

OrdinConn is currently an AI Financial Intelligence and Execution Agent with a model-agnostic runtime boundary. The active stage extends its public-data collection model to user-authorized Android applications without weakening Evidence provenance, privacy, or approval rules.

M1.5 has one narrow goal: prove that the production Tauri host can observe a real Android Emulator through ADB, persist the capture, project typed IPC state, and shut down the logical session. It is not a mobile automation demo.

## Implemented

- A local-first Tauri desktop application with a React interface and typed IPC.
- Transport-independent Rust crates for market, Evidence, Signal, strategy, model, agent, approval, execution, connector, and tool domains.
- OpenAI-compatible and mock model adapters behind Model Gateway.
- Source Registry, public REST/WebSocket/feed/HTML collectors, scheduling, bounded rolling history, readiness-aware deterministic strategies, and Evidence clusters.
- Evidence-gated Signals, contextual Agent reports, exact proposal approval capabilities, and Paper Execution only.
- Mobile M0/M1 contracts for device sessions, frames, semantic UI snapshots, element references, privacy classification, and `MobileObservation`.
- An observe-only Android adapter that can discover configured tools, enumerate existing AVDs, inspect online emulator devices, capture a frame, dump the UI tree, redact sensitive nodes, persist observations, and project the Home/Warehouse/Settings workspace.
- M1.5 diagnostics, bounded subprocess execution, production capture persistence/events, and persisted logical-session shutdown.
- A sanitized public engineering record, fail-closed privacy scanner, documentation-only Git sync script, manual X content queue, and two-hour local GitHub-sync heartbeat.

## Designed

- A local perception hierarchy that prefers system events and structured Accessibility data before bounded capture, OCR, or vision.
- Manual Capture, explicit Application Allowlist, and explicit Work Session perception modes.
- M2 verified mobile navigation with action receipts and post-action verification.
- M3 production App Skills, M4 `MobileObservation -> Evidence` promotion, and M5 physical Android devices.
- Future standalone or remote runtime hosts that reuse the same domain protocols.

Designed items are not current product capabilities.

## Verified

At product source baseline `144c87f`, the most recent full local verification recorded:

- 126 Rust tests passed.
- 31 TypeScript tests passed.
- Rust formatting and Clippy with warnings denied passed.
- TypeScript type checking and the Vite production build passed.
- The macOS Tauri application bundle was produced.
- The explicit real Android smoke returned a truthful 10-check report: three environment prerequisites failed and seven dependent checks were blocked.
- Public-log sanitizer, allowlisted Git-sync, scheduler rendering, document schema, plist lint, and repository hygiene checks passed.
- The two-hour Codex heartbeat is active. A standalone LaunchAgent was tested, but macOS denied its background shell access to this checkout under `Documents`, so it was removed instead of leaving a failing duplicate scheduler.

Fixture-based Android tests verify parsers, policies, timeouts, persistence, and IPC projection. They are not evidence of a working real emulator.

## Known Problems

- No Android SDK, ADB executable, Emulator executable, existing AVD, or online emulator was available in the verified local environment.
- M1.5 therefore did not pass; real frame capture, UI-tree capture, observation creation, typed IPC traversal, and shutdown could not be accepted against an emulator.
- M2 actions are intentionally absent and must remain absent until all ten M1.5 checks pass.
- This checkout has no configured Git remote, so public GitHub synchronization cannot run yet.
- The latest default-parallel Rust desktop unit run exposed two one-second AVD lifecycle tests that time out under suite concurrency; both pass when the desktop library suite runs serially. This is recorded as unresolved product-test timing behavior, not changed by the public-log task.

## Technical Decisions

- Mobile observation remains collection input, not Evidence or a Signal.
- Every model goes through Model Gateway; every source goes through Connector Registry.
- Structured UI state is preferred to visual inference when available.
- Missing tools and devices produce unavailable or blocked states, never fabricated success.
- Mobile command processes have deadlines so an unresponsive external tool cannot hang the desktop runtime indefinitely.
- The scheduled public-log path is separate from product-code delivery.

## Lessons Learned

- A parser test proves parser behavior; it does not prove a real device path.
- Phase gates are useful only when downstream work actually stops on failure.
- External command timeouts and consistent environment selection are part of correctness, not polish.
- Public engineering notes need the same provenance discipline as market Evidence.

## Codex Role

Codex has been used to audit the repository, turn phase constraints into implementation plans, implement bounded changes, write tests, run verification, and review the resulting diff. It is effective when the task has explicit scope, safety boundaries, and acceptance commands.

Its output still requires human-owned product intent and direct runtime evidence. It cannot turn a missing SDK or unavailable emulator into a verified integration, and a written completion claim is never a substitute for command output.

## Next

Install or expose a real Android SDK with ADB, Emulator, and one existing safe AVD; then rerun the ten-check M1.5 smoke. Enter M2 only if every check passes.
