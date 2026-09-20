# Device Runtime

## M1 device contract

`MobileDeviceSession` records session and device identity, platform, device type, OS version, screen size, connection time, current application/activity, status, and the latest observation time.

The first adapter targets Android Emulator on macOS Apple Silicon using Android Studio AVD, ADB, and UIAutomator. The upper protocol remains platform-independent so physical Android and iOS adapters can be added later.

## Observation sequence

1. Locate ADB from configured SDK paths or the process path.
2. List online emulator devices.
3. Read package/activity and screen size.
4. Enforce the application allowlist.
5. Capture a UIAutomator hierarchy.
6. Sanitize sensitive nodes and assign snapshot-scoped element references.
7. Capture the latest PNG frame.
8. Build a `MobileObservation` from the sanitized semantic snapshot.
9. Persist the session, snapshot, and observation and append audit events.

Frames remain in a bounded in-memory ring buffer. M1 does not continuously record video.

## Truthful status

The runtime exposes `unavailable`, `disconnected`, `observing`, `paused`, and `error` states. A command exit code alone does not prove observation success; required package/activity, UI tree, and frame outputs must all validate.
