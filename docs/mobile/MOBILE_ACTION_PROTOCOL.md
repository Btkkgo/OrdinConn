# Mobile Action Protocol

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

M2 remains blocked until one real run passes all ten checks in `M1_5_ACCEPTANCE.md`. Unit fixtures and parser tests cannot substitute for that run. If any check fails or is blocked, the product remains observe-only and no Tap, Swipe, Type, Back, Home, Open App, Search, `ActionReceipt`, or navigation verification implementation may be enabled.

The 2026-09-20 local run did not enter M2 because SDK/ADB/emulator/device prerequisites were absent.
