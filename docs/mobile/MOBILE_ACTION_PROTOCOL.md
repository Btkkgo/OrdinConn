# Mobile Action Protocol

[English](MOBILE_ACTION_PROTOCOL.md) | [简体中文](MOBILE_ACTION_PROTOCOL.zh-CN.md)

M1 is observe-only. Its tool capability surface advertises observation, frames, UI trees, and element inspection while Tap, Swipe, Type, Back, Home, Open App, and Search remain unavailable.

M1.5 adds environment discovery, existing-AVD listing/start, diagnostics, and a real-runtime acceptance gate. These are host lifecycle capabilities, not Agent navigation actions.

M2 will introduce `ActionReceipt` and post-action verification. Every action must reference a current snapshot and every accepted device action must be followed by a new observation. Only `VERIFIED` may advance a task by default.

Reserved verification results are:

- `VERIFIED`
- `NO_CHANGE`
- `UNEXPECTED_STATE`
- `TARGET_MISSING`
- `PERMISSION_REQUIRED`
- `LOGIN_REQUIRED`
- `CAPTCHA_BLOCKED`
- `SENSITIVE_FIELD_BLOCKED`
- `FINANCIAL_ACTION_BLOCKED`
- `STALE_OBSERVATION`
- `APP_CRASHED`
- `DEVICE_OFFLINE`
- `INTERRUPTED`

React never invokes ADB or a platform device API directly.

## M2 entry gate

M2 remained blocked until the real capture gate and the separate desktop acceptance items in `M1_5_ACCEPTANCE.md` passed. Unit fixtures and parser tests could not substitute for that evidence. M1.5 has now passed 15/15 mandatory items, but this result does not itself authorize M2. No Tap, Swipe, Type, Back, Home, Open App, Search, `ActionReceipt`, or navigation verification implementation may be enabled without a new explicit instruction.

The first 2026-09-20 run stopped with missing SDK/ADB/Emulator prerequisites. The later real Android 16 ARM64 acceptance passed, and the project still stopped before M2 as required.
