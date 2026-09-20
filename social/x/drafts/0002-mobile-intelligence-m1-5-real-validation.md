# X Draft 0002 — Mobile Intelligence M1.5 Real Validation

- Stage: Mobile Intelligence M1.5 real-environment acceptance
- GitHub Issue: https://github.com/Btkkgo/OrdinConn/issues/1
- Follow-up Issue: https://github.com/Btkkgo/OrdinConn/issues/4
- GitHub Commit: https://github.com/Btkkgo/OrdinConn/commit/f8705c5
- Status: DRAFT — manual owner review required; M1.5 PASS; M2 NOT STARTED
- Repository Link: https://github.com/Btkkgo/OrdinConn

## Suggested Post

OrdinConn Mobile Intelligence M1.5 now passes against a real Android 16 ARM64 Emulator. The packaged desktop also proved status, allowlist error, observation, and stop through the real Rust → Tauri → React path. Sensitive redaction passed; M2 remains intentionally not started.

## Suggested Thread

### 1/5

OrdinConn Mobile Intelligence was blocked by a missing Android runtime. The rule was simple: fixtures could prove parser behavior, but they could not substitute for a real Emulator, frame, UI tree, observation, IPC path, or shutdown.

### 2/5

The new environment is deliberately narrow: OpenJDK 21, official Android command-line tools, Android 36 Google APIs ARM64, and one dedicated Pixel 8 AVD named `OrdinConn_M1_5`. No Android Studio install and no existing AVD was modified.

### 3/5

The real device exposed three fixture-blind problems: PNG output filled a child-process pipe, Android 16 changed the useful window-dump surface, and UIAutomator emitted platform nodes with reversed bounds. Each fix received a failing regression before passing.

### 4/5

Final result: the capture gate passed with a 188,909-byte frame, 70 sanitized UI elements and refs, a real MobileObservation, persisted/audited workspace projection, and clean shutdown. Packaged Tauri GUI IPC passed separately. A real password node was redacted without serialized plaintext.

### 5/5

Technical lesson: real integration evidence is also schema discovery. Codex lesson: a truthful blocked gate creates the exact checklist needed for the eventual pass. M2 remains NOT STARTED pending owner review. https://github.com/Btkkgo/OrdinConn/issues/1

## Suggested Screenshots

1. The visible `OrdinConn_M1_5` Android Emulator showing the public-safe Android Settings home page with no account, notification, or personal data.
2. A sanitized terminal crop showing `adb devices -l` plus the ten-check M1.5 PASS matrix; remove the local username path and unnecessary host metadata.
3. The existing OrdinConn Mobile diagnostic/observation view if it can be shown without private content. Do not add product UI solely for this draft.

## Technical Lesson

Draining subprocess output is part of timeout correctness. A producer blocked on a full pipe cannot exit, so reading only after exit turns a healthy large capture into a false timeout. Real operating-system diagnostic formats must also be treated as versioned external schemas.

## Codex Lesson

The strongest contribution was preserving the first blocked result, then using the real dependency to drive three evidence-backed fixes instead of weakening the gate. A later green rerun does not erase an intermittent concurrency failure, so that test-harness risk remains public in Issue #4.

## Chinese Reference

OrdinConn Mobile Intelligence M1.5 已在真实 Android 16 ARM64 Emulator 上通过。打包桌面应用另行验证了真实 Rust → Tauri → React 的状态、白名单错误、观察与停止链路；密码节点脱敏也通过。M2 仍未开始，等待后续明确授权。

## Manual Publication Reminder

Publication is owner-controlled. Review every claim against Issue #1 and commit `f8705c5`, select only public-safe screenshots, and publish manually. Repository automation must not log in to X or publish this draft.
