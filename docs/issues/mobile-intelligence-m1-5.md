## Goal

Establish a real Android Emulator environment and complete M1.5 validation.

## Current Result

- Android SDK: FAIL
- ADB: FAIL
- Emulator: FAIL
- AVD: FAIL
- Online Android Emulator: NONE
- Frame Capture: BLOCKED
- UI Tree: BLOCKED
- MobileObservation: BLOCKED
- M1.5 Gate: NOT PASSED
- M2: NOT STARTED

The standard macOS SDK locations were checked and no usable Android runtime was detected. The latest real gate reported 3 failed prerequisites and 7 blocked dependent checks.

## Required Work

- Install or locate Android SDK.
- Verify the ADB executable.
- Verify the Emulator executable.
- Create one safe AVD.
- Launch the Android Emulator.
- Confirm an online emulator through ADB.
- Run the M1 real smoke.
- Capture a real frame.
- Dump a real UIAutomator hierarchy.
- Validate sensitive-node redaction.
- Generate a real `MobileObservation`.
- Verify Snapshot Parse, Element Refs, Tauri IPC, and Session Shutdown.
- Resolve or correctly budget the two AVD lifecycle tests that time out under default suite concurrency.

## Acceptance Criteria

Every item above must run against the real production path. Mock or fixture output cannot substitute for real acceptance. Only then may M1.5 be marked passed and M2 begin.

## Current Boundary

This Issue does not authorize M2 navigation actions, real-money operations, unrestricted capture, private application access, or collection of sensitive fields.
