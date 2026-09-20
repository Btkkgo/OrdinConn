# Device Runtime

## M1 device contract

`MobileDeviceSession` records session and device identity, platform, device type, OS version, screen size, connection time, current application/activity, status, and the latest observation time.

The first adapter targets Android Emulator on macOS Apple Silicon using Android Studio AVD, ADB, and UIAutomator. The upper protocol remains platform-independent so physical Android and iOS adapters can be added later.

## Observation sequence

1. Detect an existing Android SDK from the saved setting, `ANDROID_SDK_ROOT`, `ANDROID_HOME`, `~/Library/Android/sdk`, and finally the process path.
2. Resolve `adb`, `emulator`, `sdkmanager`, and `avdmanager` without requiring shell PATH configuration.
3. List existing AVDs and online emulator devices.
4. Read package/activity and screen size.
5. Enforce the application allowlist.
6. Capture a UIAutomator hierarchy.
7. Sanitize sensitive nodes and assign snapshot-scoped element references.
8. Capture the latest PNG frame.
9. Build a `MobileObservation` from the sanitized semantic snapshot.
10. Persist the session, snapshot, and observation and append audit events.

Frames remain in a bounded in-memory ring buffer. M1 does not continuously record video.

## Truthful status

The runtime exposes `unavailable`, `disconnected`, `observing`, `paused`, and `error` states. A command exit code alone does not prove observation success; required package/activity, UI tree, and frame outputs must all validate.

## M1.5 environment diagnostics

Settings displays SDK, ADB, emulator, AVD, and online-device readiness plus the resolved paths. Detection never installs Android Studio, SDK packages, system images, apps, or AVDs.

`list_avds` is represented by the `availableAvds` diagnostic collection. Each entry preserves the AVD name and reports running state plus device profile and architecture when the existing `config.ini` provides them.

Starting an existing AVD uses the discovered emulator executable, then waits for both an online `emulator-*` ADB device and `sys.boot_completed=1`. The production timeout is 120 seconds. Timeout never causes OrdinConn to delete, reset, or terminate an AVD. OrdinConn shutdown ends only its logical mobile session.

Environment probes have a five-second total deadline, and individual observation commands have a ten-second deadline. Tauri runs these blocking host operations on its blocking worker pool so a hung Android tool cannot indefinitely occupy the async command executor. The 120-second AVD deadline includes discovery and boot polling rather than starting after discovery.

Logical shutdown is also persisted: the session row transitions to `ended` and a redacted `mobile.session_ended` event is appended. OrdinConn still does not kill the emulator process.
