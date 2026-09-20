# Mobile Action Protocol

M1 is observe-only. Its tool capability surface advertises observation, frames, UI trees, and element inspection while Tap, Swipe, Type, Back, Home, Open App, and Search remain unavailable.

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
