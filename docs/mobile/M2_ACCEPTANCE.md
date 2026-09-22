# Mobile Intelligence M2 — Verified Navigation Acceptance

[English](M2_ACCEPTANCE.md) | [简体中文](M2_ACCEPTANCE.zh-CN.md)

Date: 2026-09-22. Issue: [#7](https://github.com/Btkkgo/OrdinConn/issues/7). Scope: the dedicated `OrdinConn_M1_5` Android 16 emulator, Android Settings, and a disposable password-field test application. M3, physical devices, autonomous navigation, account login, financial actions, and X publication are outside this gate.

## Contract and policy

The only public action targets are `tap(elementRef)`, `swipe(direction)`, `type(elementRef, text)`, `back`, `home`, and `open_app(packageName)`. There is no raw-coordinate, shell, ADB-argument, or free-form intent field. Rust policy requires an active emulator session, a user-controlled package allowlist, matching expected and live foreground package/activity, a snapshot no older than ten seconds, a valid snapshot-bound target, and a maximum of 20 attempted dispatched actions per session. The M2 production host additionally limits non-escape navigation to Android Settings and Settings Intelligence; a user-added package alone cannot enable arbitrary-app actions. The dynamically resolved Launcher is used only for Home verification and return to Settings. Sensitive or financial screens allow only escape actions (`back`/`home`). Tap derives coordinates from validated element bounds; Swipe derives bounded coordinates from a plausible screen size. Type requires a focused enabled EditText and currently accepts only 1–256 ASCII letters, digits, and spaces. Other text is rejected.

The host serializes observe/action/stop, rechecks device and foreground, and re-dumps the live UI tree immediately before non-escape input. Any same-activity element or bounds change blocks stale coordinates. It takes a new production observation after execution; Tap, Swipe, Type, and Back require a foreground or UI-tree change rather than incidental frame pixels, Home must reach the dynamically resolved Launcher package, and OpenApp must reach its requested package. OpenApp uses a fixed clear-top launch flag so an existing Settings search task cannot masquerade as opening Settings. A sanitized Pending Receipt and audit intent commit atomically before device input; a failed write prevents input. Result persistence updates that receipt in place and appends outcome events. If it fails after input, the Pending Receipt remains, the logical session stops, and an operator must investigate the unknown device outcome. Type plaintext is excluded from request serialization and Debug output, redacted from the post-action semantic capture, and never written to receipts or events. Frames stay in memory; screenshot pixels may still visually show non-sensitive typed text in the live view and are not the basis of the plaintext-persistence assertion.

## Real environment evidence

The explicitly gated `ORDINCONN_MOBILE_M2_SMOKE=1` test ran on the single online `OrdinConn_M1_5` AVD. It opened the real Settings Network page by a snapshot element ref, blocked reuse of the old ref before ADB input, scrolled a real Settings page, returned with Back to the prior Settings activity, opened Settings Search, typed the non-personal word `wifi`, confirmed changed search state and semantic redaction, returned Home to the dynamically resolved real Launcher, and reopened `com.android.settings`. It also blocked a wrong expected package and an unallowlisted OpenApp before input. The test persisted each receipt and checked pre-input intent plus requested/blocked/executed/verified outcome events.

The separately gated real password-field test installed a disposable local APK on the same emulator. Type toward its `password=true` EditText returned `SensitiveScreen` and `Blocked` with no ADB input. A random non-personal test string was absent from serialized capture, receipt, projected workspace, and the SQLite file; only length/hash metadata was stored. The test APK was uninstalled and its local build directory moved to Trash. Financial-action words were tested with synthetic UI fixtures, not a real financial app.

The final packaged macOS Tauri application visibly completed React inspector → typed IPC → Rust policy/ADB → persisted receipt → React projection: selecting a clickable Settings element and pressing manual Tap changed the UI tree from 70 to 65 elements and displayed `executed · VERIFIED` with distinct pre/post Snapshot IDs. Stop Session returned the UI to Disconnected while `emulator-5554` remained online. Earlier stale-snapshot manual attempts visibly showed `blocked · STALE_SNAPSHOT`; none was counted as successful navigation.

The M1.5 real smoke was rerun after the M2 changes and passed all ten environment/capture/projection/shutdown checks, including a real 70-element sanitized Settings tree. The default-parallel desktop suite passed 44/44; the full Rust workspace, Rust formatting, Clippy with warnings denied, Rust build, 30 desktop and five contract TypeScript tests, typecheck, Vite build, and macOS Tauri bundle passed.

## Mandatory acceptance matrix

Evidence keys: **U** = automated unit/fixture test, **R** = real AVD gate, **G** = packaged GUI, **S** = repository safety gate. A fixture is never represented as real device evidence.

| # | Check | Evidence | Result |
|---:|---|---|---|
| 1 | Action contracts | U: typed six-target enum and strict JSON rejection | PASS |
| 2 | Emulator-only policy | U; R: exact dedicated AVD and one online emulator | PASS |
| 3 | Session precondition | U: inactive/mismatched session denial | PASS |
| 4 | Allowlist policy | U; R: Settings/test-only packages; arbitrary user-added apps denied by production host | PASS |
| 5 | Expected-package precondition | U; R: wrong-package denial | PASS |
| 6 | Snapshot freshness | U; R/G: stale ref blocked | PASS |
| 7 | Element-ref target validation | U; R: snapshot-bound Network ref | PASS |
| 8 | Sensitive target blocking | U; R: password EditText denied | PASS |
| 9 | Financial target blocking | U: English/Chinese payment, transfer, purchase, and signing fixtures | PASS |
| 10 | Bounded TypeText | U: length, character set, focus, EditText | PASS |
| 11 | Type plaintext non-persistence | U; R: serialized state and SQLite scan | PASS |
| 12 | Action budget | U: 20-action boundary before ADB input | PASS |
| 13 | Real Tap | R; G: Settings subpage | PASS |
| 14 | Real Swipe | R: changed Settings UI/frame | PASS |
| 15 | Real Type | R: Settings Search changed | PASS |
| 16 | Real Back | R: returned from Settings subpage | PASS |
| 17 | Real Home | R: Launcher foreground | PASS |
| 18 | Real OpenApp | R: Settings target package foreground | PASS |
| 19 | Stale-ref real block | R: old snapshot ref, no command sent | PASS |
| 20 | Wrong-package block | R: no command sent | PASS |
| 21 | Unallowlisted-app block | R: no command sent | PASS |
| 22 | Action receipt persistence | U: Pending survives restart and finalizes in place; R: latest receipt projected | PASS |
| 23 | Audit events | U; R: durable pre-input intent and requested/blocked/executed/verified outcomes | PASS |
| 24 | Post-action observation | U; R: new frame/UI capture | PASS |
| 25 | Verification | U; R/G: changed state and target package | PASS |
| 26 | Tauri IPC | U: strict input; G: manual command response | PASS |
| 27 | React manual controls | U; G: inspector selection and manual Tap | PASS |
| 28 | Session shutdown regression | U; R/G: logical stop, AVD stays online | PASS |
| 29 | M1.5 regression | R: ten-check real smoke; default-parallel desktop | PASS |
| 30 | Security gate | S: public-doc validator, sanitizer, repo scan, diff check | PASS |

**Final gate: 30 PASS / 0 FAIL / 0 BLOCKED / 0 NOT RUN.**

## Commands and limitations

The real gates require explicit environment variables and are never run by default `cargo test --workspace`:

```bash
ORDINCONN_MOBILE_M2_SMOKE=1 cargo test -p ordinconn-desktop real_m2_navigation_smoke_is_explicitly_gated -- --nocapture
ORDINCONN_MOBILE_M2_SENSITIVE_SMOKE=1 cargo test -p ordinconn-desktop real_m2_sensitive_type_block_is_explicitly_gated -- --nocapture
ORDINCONN_MOBILE_SMOKE=1 ORDINCONN_MOBILE_ALLOWED_APPS=com.android.settings cargo test -p ordinconn-desktop real_emulator_smoke_is_explicitly_gated -- --nocapture
```

The Launcher package was detected dynamically, and it and Settings Intelligence were added only to the real-smoke allowlist; production defaults were not broadened. A transient early post-Tap capture interruption, a task-reuse OpenApp mismatch, and stale GUI attempts were found during validation; failed attempts remain distinct from the final passing rerun. An action whose post-capture fails remains `Failed`/`Interrupted` and is never automatically retried or claimed verified. SQLite unavailability before input prevents that input; a storage failure after input leaves a durable Pending Receipt with unknown device outcome, stops the session, and requires operator investigation. Semantic denylist coverage is deliberately conservative; the fixed M2 navigation surface prevents user-added financial apps from relying on this imperfect classifier. The M2 gate does not authorize M3 or any public X post.
