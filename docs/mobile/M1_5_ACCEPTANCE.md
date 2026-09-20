# Mobile Intelligence M1.5 Acceptance

[English](M1_5_ACCEPTANCE.md) | [简体中文](M1_5_ACCEPTANCE.zh-CN.md)

## Purpose

M1.5 proves that the production OrdinConn host can use a real, existing Android Emulator through its typed Tauri boundary. It is the mandatory gate between observe-only M1 and verified navigation M2.

## Environment detection

The detector searches, in order:

1. The Android SDK path saved in OrdinConn Settings.
2. `ANDROID_SDK_ROOT`.
3. `ANDROID_HOME`.
4. `~/Library/Android/sdk` on macOS.
5. Individual tools in the process `PATH`.

It resolves and reports the actual paths for `adb`, `emulator`, `sdkmanager`, and `avdmanager`, the first line of `adb version`, existing AVD metadata, and online emulator devices. It never downloads or installs missing components.

## Automated behavior coverage

`cargo test -p ordinconn-desktop` covers SDK-root and direct-ADB normalization, tool discovery, AVD metadata parsing, online/offline/unauthorized/physical device filtering, unknown AVD rejection, bounded boot timeout, `sys.boot_completed` readiness, UIAutomator parsing, redaction, allowlisting, and idempotent logical session shutdown.

These tests use controlled process fixtures. They do not count as real Android evidence.

## Real gate

Run with a safe, explicitly allowlisted package visible on an existing emulator:

```bash
ORDINCONN_MOBILE_SMOKE=1 \
ORDINCONN_MOBILE_ALLOWED_APPS=com.android.settings \
cargo test -p ordinconn-desktop real_emulator_smoke -- --nocapture
```

The harness may start one existing AVD when none is online. It waits at most 120 seconds for ADB and Android boot completion. It then uses the production observation path and emits a JSON report containing exactly these checks:

1. `ADB_READY`
2. `EMULATOR_READY`
3. `DEVICE_ONLINE`
4. `FRAME_CAPTURE`
5. `UI_TREE_CAPTURE`
6. `SNAPSHOT_PARSE`
7. `ELEMENT_REFS`
8. `MOBILE_OBSERVATION`
9. `WORKSPACE_PROJECTION`
10. `SESSION_SHUTDOWN`

Every check must be `PASS`. Missing prerequisites make dependent checks `BLOCKED`; no skipped or unknown result is converted to `PASS`.

`WORKSPACE_PROJECTION` passes only after the real capture has traversed the persistence/projection helper used by `observe_mobile_device`: SQLite capture persistence, the three allowed mobile audit events, `MobileWorkspaceData` projection containing the observation/feed item, and JSON serialization must all succeed. It is not presented as proof that a Tauri command was invoked from the frontend. `SESSION_SHUTDOWN` additionally requires both logical host shutdown and the persisted `ended` session transition.

Typed Tauri IPC is a separate real-desktop acceptance item. The packaged application must visibly demonstrate the Rust → Tauri command/event → React path for initial status, a controlled error, session start, observation, and session stop. The persisted event sequence must contain `mobile.session_started`, `mobile.snapshot`, `mobile.observation`, and `mobile.session_ended` for the same session.

Sensitive-node acceptance uses a temporary local application with a password input and an ephemeral test value. The app is allowlisted only for the test. Run the separately gated check without placing the test value in source control:

```bash
ORDINCONN_MOBILE_SENSITIVE_SMOKE=1 \
ORDINCONN_MOBILE_TEST_SECRET=<local-test-value> \
cargo test -p ordinconn-desktop real_sensitive_redaction_smoke -- --nocapture
```

The check requires at least one recorded redaction, `SensitiveFieldBlocked`, a `[REDACTED]` element, and no occurrence of the supplied value in serialized capture data.

## Observed local result — 2026-09-20

- Host: macOS arm64 / Apple Silicon.
- Java: OpenJDK 21.0.12.1.
- SDK root: `~/Library/Android/sdk`.
- ADB: 37.0.1.
- Emulator: 37.1.11 with Hypervisor.Framework acceleration.
- System image: `system-images;android-36;google_apis;arm64-v8a` revision 7.
- AVD: `OrdinConn_M1_5`, Pixel 8 profile.
- Online device: `emulator-5554`, Android 16 / API 36, 1080×2400 at 420 dpi.
- Final automated rerun frame: 188,909-byte PNG with a non-empty SHA-256 hash.
- UI tree: 70 valid sanitized elements; two Android-generated nodes with reversed bounds were ignored as invalid rectangles.
- Snapshot/refs/observation/workspace projection/shutdown: `PASS` through the production persistence, audit, projection, serialization, and logical-session path.
- Tauri IPC: `PASS` in the packaged desktop application. An empty allowlist produced the expected frontend error; after allowlisting `com.android.settings`, the UI showed `Observing`, `emulator-5554`, `com.android.settings`, `VERIFIED`, a real frame, and 70 UI elements. Stop returned the UI to `Disconnected`, persisted `mobile.session_ended`, and left the emulator online.
- Sensitive-node check: one real password node, one OrdinConn redaction, and no test value in serialized capture data.

## Final mandatory acceptance matrix

1. Android SDK readiness: `PASS`
2. ADB readiness: `PASS`
3. Emulator readiness: `PASS`
4. Dedicated AVD readiness: `PASS`
5. Online emulator device: `PASS`
6. Real frame capture: `PASS`
7. Real UI-tree capture: `PASS`
8. Sensitive-node redaction: `PASS`
9. Snapshot parse: `PASS`
10. Snapshot-scoped element references: `PASS`
11. Real `MobileObservation`: `PASS`
12. Persisted and audited workspace projection: `PASS`
13. Packaged Tauri IPC path: `PASS`
14. Controlled allowlist error: `PASS`
15. Persisted logical session shutdown: `PASS`

Final count: **15 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN**.

Result: **M1.5 PASS — M2 remains NOT STARTED.** No mobile navigation actions were implemented or enabled.

The first corrective default-parallel workspace run failed two existing AVD lifecycle process fixtures plus one corrected gate-label assertion. After the assertion fix, all 125 Rust tests passed in the default-parallel workspace rerun and all 28 desktop tests passed serially. The AVD fixture nondeterminism remains tracked in Issue #4 and is not hidden by the green rerun.
