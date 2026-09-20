# Mobile Intelligence M1.5 Acceptance

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
9. `TAURI_IPC`
10. `SESSION_SHUTDOWN`

Every check must be `PASS`. Missing prerequisites make dependent checks `BLOCKED`; no skipped or unknown result is converted to `PASS`.

`TAURI_IPC` passes only after the real capture has traversed the same production helper used by `observe_mobile_device`: SQLite capture persistence, the three allowed mobile audit events, `MobileWorkspaceData` projection containing the observation/feed item, and JSON serialization must all succeed. `SESSION_SHUTDOWN` additionally requires both logical host shutdown and the persisted `ended` session transition.

## Observed local result — 2026-09-20

- Android SDK: missing. Neither `ANDROID_SDK_ROOT` nor `ANDROID_HOME` was configured, and `~/Library/Android/sdk` did not exist.
- ADB: missing from all configured, standard, and PATH candidates.
- Emulator: missing from all configured, standard, and PATH candidates.
- AVDs: none discoverable because no emulator executable was available.
- Online devices: none.
- `ADB_READY`, `EMULATOR_READY`, `DEVICE_ONLINE`: `FAIL`.
- The remaining seven checks: `BLOCKED`.

Result: **M1.5 FAIL — M2 was not entered.** No mobile navigation actions were implemented or enabled.
