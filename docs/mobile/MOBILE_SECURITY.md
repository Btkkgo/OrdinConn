# Mobile Runtime Security

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

The M1.5 gate fails closed. Missing tools or an offline device block every downstream frame, UI tree, observation, IPC, and shutdown acceptance claim. Test fixtures prove parser behavior only and are never presented as real-device evidence.
