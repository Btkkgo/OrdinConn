# Desktop Startup Diagnosis — 2026-10-02

[English](DESKTOP_STARTUP_DIAGNOSIS.md) | [简体中文](DESKTOP_STARTUP_DIAGNOSIS.zh-CN.md)

[Issue #15](https://github.com/Btkkgo/OrdinConn/issues/15), related to [Issue #14](https://github.com/Btkkgo/OrdinConn/issues/14). Source baseline: `codex/realtime-workbench` / `0eff77e6e08b1170589d397dc52645b4fc0aa132`.

## Proven cause

The retained `target/release/bundle/macos/OrdinConn.app` is the frozen binary built from `d3e6c1488135169dcfc5939d82d772606206b923`, not the current Mobile Data Acquisition executable. Its SHA256 matches the freeze manifest and its Mach-O UUID matches the reported crash: `85E502DA-0D8D-3888-B872-33E8162DFB05`.

The frozen source embeds SQLite migrations 1–11. The shared application database applied migration 12 on October 2 at 05:28:41 UTC during current development startup. A read-only comparison found no checksum differences for migrations 1–11; migration 12 is the sole applied version missing from the frozen source. SQLx 0.8.6 `validate_applied_migrations` returns `MigrateError::VersionMissing(12)` for this condition. A disposable-database test reproduces that exact error and proves the current runtime can reopen the same database without losing a sentinel record.

This is an **obsolete packaged executable / newer shared schema incompatibility**, not a failure in the current migration, MobileHost, ADB, extractor, UI or a new nested async runtime.

## Crash and startup chain

The owner-reported October 2 20:06:05 +0800 report and the 19:59 report both show Thread 0 on `com.apple.main-thread`: `__pthread_kill → pthread_kill → abort → ordinconn-desktop` offsets `0xd49c84`, `0xd49a0c`, followed by the application launch callback. The release executable is stripped; these offsets were not falsely assigned source symbols.

Terminal reproduction supplied the missing upstream source evidence:

```text
tauri-2.11.5/src/app.rs:1425:11
Failed to setup app: error encountered during setup hook: database migration error
panic in a function that cannot unwind
thread caused non-unwinding panic. aborting.
```

Source path: `apps/desktop/src-tauri/src/lib.rs::run` setup closure → `AppRuntime::initialize` → `crates/ordinconn-app/src/db.rs::open_database` line 19 (`sqlx::migrate!().run`) → `AppError::Migration` → Tauri `make_run_event_loop_callback` / `RuntimeRunEvent::Ready` line 1425. Tauri panics on the returned setup error, then aborts across the native callback boundary. Initialization fails before `MobileHost::discover`, `app.manage`, or collection commands run. WebView allocation before setup completion is not proof that the app finished startup.

A direct terminal reproduction at 20:20:45 +0800 generated a fresh SIGABRT / signal 6 report. The terminal tool reported exit status 1; the OS crash report separately proves signal 6. stdout was empty and stderr retained the panic above. Raw logs and reports stay local and are not committed.

Some terminal launches waited at macOS's recovery-window prompt. A one-second main-thread sample showed `NSPersistentUIRestorer::promptToIgnorePersistentStateWithCrashHistory → NSAlert runModal`. Choosing “Don’t Reopen” allowed setup to proceed and reproduce the migration abort. Waiting at that dialog was not a successful startup, and terminal launch did not eliminate the underlying incompatibility.

## Recovery and scope

Use the existing official `npm run desktop:dev` entry point with `ORDINCONN_INTERNAL_M3_ACCEPTANCE=0` and real acceptance opt-ins disabled. It runs the current debug executable against the existing database. No production source change, database downgrade, migration bypass, schema overwrite, bundle replacement or signing was needed for development recovery. The frozen bundle remains incompatible with this newer database and is not a valid launch target for Issue #14.

Current development startup opened the existing SQLite/WAL, created WebKit, completed main-frame load, and remained alive through the regression run. The development watcher restarted it only when the new fixture test file was edited. Native UI automation could not bind the unbundled debug process; independent visual confirmation is still owner-provided. WebKit loading/continued desktop polling and process liveness are recorded separately from H1.

The existing `OrdinConn_M1_5` emulator was started and read-only ADB inventory confirmed `emulator-5554` connected. No Observe or Android Action was executed. Human H1–H7 remain unverified. Android Actions = 0; Provider requests = 0; signing verification = 0; X Draft = NONE.

## Why earlier tests missed it

Earlier tests and compilation used current source/current migrations with fixture databases. They did not launch the retained, deliberately unreplaced pre-migration-12 bundle against the newly upgraded owner database. Desktop process existence also did not prove which app a Finder/LaunchServices entry would launch. This diagnosis adds the missing old-migrator/new-schema compatibility fixture and documents binary identity as a separate startup prerequisite.

## Validation

- Rust: 338 PASS, seven real Android/final gates explicitly excluded. The initial sandbox run could not bind local HTTP/WebSocket fixtures; the same suite passed outside that restriction.
- Desktop: 73 PASS. Contracts: 9 PASS. Typecheck, frontend build, native Rust build, Rust formatting: PASS.
- The new diagnostic fixture initially omitted a required timestamp; corrected before the passing full run. No production code was altered to accommodate the test.
- The old packaged app still reproduces SIGABRT. Development startup recovery does not claim packaged recovery or human mobile acceptance.
- Preserve the existing eight unrelated untracked entries, M3 state/history, provider configuration, Keychain and frozen executable.
