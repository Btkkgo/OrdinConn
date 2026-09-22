# Mobile Action Protocol

[English](MOBILE_ACTION_PROTOCOL.md) | [简体中文](MOBILE_ACTION_PROTOCOL.zh-CN.md)

M1 is observe-only. Its tool capability surface advertises observation, frames, UI trees, and element inspection while Tap, Swipe, Type, Back, Home, Open App, and Search remain unavailable.

M1.5 adds environment discovery, existing-AVD listing/start, diagnostics, and a real-runtime acceptance gate. These are host lifecycle capabilities, not Agent navigation actions.

M2 passed its 30-item real-emulator and packaged-GUI gate under [Issue #7](https://github.com/Btkkgo/OrdinConn/issues/7); see the paired [acceptance record](M2_ACCEPTANCE.md). Its platform-neutral action request, policy decision, sanitized receipt, verification contract, typed IPC, and manual controls are implemented and verified at their respective boundaries. The production host limits non-escape actions to Android Settings/Settings Intelligence even when a user adds another package to the allowlist. Every action references a current snapshot, non-escape input rechecks the live UI tree, and every dispatched device action requires a fresh post-action observation. Only `VERIFIED` may advance a task by default. No autonomous Agent navigation is available.

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

M2 remained blocked until the real capture gate and separate desktop acceptance items in `M1_5_ACCEPTANCE.md` passed. M1.5 passed 15/15 mandatory items; the owner subsequently gave the separate explicit M2 instruction tracked in Issue #7. Real M2 navigation and negative gates have now passed. A generic Search action and autonomous action loops remain out of scope; manual Tap/Type in an allowlisted Settings search field are part of the bounded action set.

The first 2026-09-20 run stopped with missing SDK/ADB/Emulator prerequisites. The later real Android 16 ARM64 acceptance passed, and the project stopped before M2 until the new owner instruction.
