# Mobile Intelligence Runtime

## Product role

Mobile Intelligence Runtime is a first-class OrdinConn collection surface. It observes user-authorized Android applications and produces structured observations that enter the same evidence and strategy path as API, WebSocket, RSS, HTML, browser, and desktop sources.

It is not a simulator manager, a general phone assistant, or an automation demo.

The canonical flow is:

`Device state -> Observe -> Structured UI state -> Policy check -> Agent decision -> Auditable action -> Post-action observation -> Verification -> MobileObservation -> Evidence -> Strategy -> SignalCandidate -> Published Signal`

M1 ends at observation. It does not execute taps, swipes, text input, application launches, or navigation actions.

## Phase scope

- M0: contracts, product policy, security, audit events, and documentation.
- M1: Android Emulator detection, observe-only sessions, frames, semantic UI snapshots, element references, observations, intelligence feed, warehouse, and the three-column UI.
- M2: verified navigation. Explicitly out of scope for this phase.
- M3: one production app skill.
- M4: observation-to-evidence promotion and mobile-backed strategy inputs.
- M5: physical Android devices.

## Product surfaces

The primary navigation contains Home, Warehouse, and Settings. Existing financial, signal, agent, approval, connector, and model functionality remains in the codebase and is integrated into those three surfaces rather than deleted.

Home answers three questions:

- Left: what did the system observe?
- Center: where is the mobile runtime currently observing?
- Right: what may the combined evidence mean?

The UI never displays the OrdinConn product name, version copy, or a logo wordmark. It keeps only the application icon at the top of the navigation rail.

## Architecture boundary

- `mobile-runtime` owns platform-independent domain contracts and semantic snapshot construction.
- The Tauri adapter owns ADB process execution and Android-specific parsing.
- `ordinconn-app` owns persistence, audit events, intelligence-feed projection, warehouse state, research tasks, and strategy settings.
- React consumes typed IPC contracts and never runs ADB.
- Mobile observations cannot create a Published Signal directly. They must pass through Connector Registry, Evidence validation, Strategy, and SignalCandidate in a later evidence-promotion phase.

## Acceptance boundary

Real Emulator support is verified only when `ORDINCONN_MOBILE_SMOKE=1` runs against an available Android Emulator. Absence of Android SDK, ADB, an online emulator, or an allowed application is reported as unavailable; it is never replaced with a fabricated success state.
