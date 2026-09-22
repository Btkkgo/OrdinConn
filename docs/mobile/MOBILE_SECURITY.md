# Mobile Runtime Security

[English](MOBILE_SECURITY.md) | [简体中文](MOBILE_SECURITY.zh-CN.md)

## Default policy

Application access is deny-all. A package must be placed in the user-controlled allowlist before its UI content can be observed. The runtime never collects passwords, OTP values, seed phrases, private keys, payment details, or secure-field contents.

## Prohibited behavior

- Automatic login, password or OTP reading, cookie theft, CAPTCHA bypass
- Private-message collection, rooting, APK modification, TLS interception, or private APIs
- Posting, commenting, messaging, liking, following, or account mutations
- Real orders, transfers, deposits, withdrawals, wallet signing, private keys, or seed phrases

Nodes and screens associated with password, OTP, wallet, banking, payment, buy, sell, long, short, place order, withdraw, transfer, or sign flows are redacted or blocked. Event payloads contain identifiers, hashes, status, and counts rather than complete private content.

Frames are not persisted by default. Only sanitized semantic snapshots and structured observations enter SQLite. Research tasks have explicit duration, step, scroll, page, observation, and model-call budgets.

## Environment and AVD safety

Environment discovery is read-only. OrdinConn may start an already-existing AVD after resolving its exact name, but it does not create, edit, reset, delete, or automatically terminate AVDs. It does not install Android Studio, command-line tools, system images, applications, or permissions.

The M1.5 gate fails closed. Missing tools or an offline device block every downstream frame, UI tree, observation, IPC, and shutdown acceptance claim. A malformed sensitive or financial node also fails the entire UI-tree parse; only malformed non-sensitive platform nodes may be discarded. Test fixtures prove parser behavior only and are never presented as real-device evidence.

## Real redaction acceptance

The M1.5 validation used a temporary local application containing one password input and a non-personal ephemeral test value. UIAutomator marked the node as a password and did not expose the plaintext. OrdinConn recorded the redaction, marked the snapshot `SensitiveFieldBlocked`, replaced the element text with `[REDACTED]`, and kept the supplied value out of serialized capture data. The test application and temporary artifacts were removed after the check, and the repository scan returned zero matches for the test value.

## M2 action boundary

M2 remains emulator-only, manual, allowlisted, snapshot-bound, and limited to 20 dispatched attempts per session. Production non-escape navigation is additionally limited to Android Settings and Settings Intelligence, so a user-added arbitrary app does not gain action authority. Same-activity live UI-tree changes block stale-coordinate input. Sensitive/financial screens permit only Back/Home escape; common English/Chinese payment and credential terms are redacted, but semantic matching is not a universal language guarantee. A durable sanitized Pending Receipt and audit intent precede ADB input; failed pre-input persistence prevents input, while failed finalization leaves that receipt Pending and stops the logical session. The real M2 password-field attempt was blocked before ADB input; its random test text was absent from serialized capture, receipt, workspace, and SQLite. A disposable test APK was installed by the validation procedure, not by the production runtime, and uninstalled afterward. See [M2 acceptance](M2_ACCEPTANCE.md). A failed post-action capture never verifies an action or triggers automatic retry.
