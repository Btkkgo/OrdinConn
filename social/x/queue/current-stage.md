---
stage: Mobile Intelligence M0-M1.5
created_at: 2026-09-20T20:00:00+08:00
status: blocked_remote
verified_commit: 144c87f
github_reference: pending
---

Thread 1/6

I am building OrdinConn as a local-first financial intelligence agent—not another chat wrapper. Models can change. The durable layer is the runtime that turns public data into traceable Evidence, Signals, approvals, and audited results. #BuildInPublic #AIAgent

Thread 2/6

The latest problem: useful market context sometimes exists only inside mobile apps. But “let the agent see a phone” cannot mean unlimited screenshots or silent automation. The collection boundary has to be explicit, observable, and privacy-aware.

Thread 3/6

The M0/M1 design is observe-only: app allowlist, bounded frames, semantic UI snapshots, scoped element refs, sensitive-node redaction, and MobileObservation. Mobile data is not Evidence and cannot publish a Signal by itself.

Thread 4/6

At commit 144c87f, 126 Rust tests and 31 TypeScript tests passed; typecheck, frontend build, and the macOS bundle passed. The real Android gate did not: SDK, ADB, and Emulator were absent, so 3 checks failed and 7 were blocked. M2 was not started.

Thread 5/6

The reusable lesson: fixture tests prove parsers and policy, not a real integration. A useful phase gate must name every prerequisite, mark dependent work BLOCKED, and actually prevent the next phase from shipping on simulated confidence.

Thread 6/6

Codex worked best here as Read → Bound the phase → Test → Review → Verify. The review caught process timeouts, inconsistent SDK selection, and an IPC acceptance gap. GitHub reference is pending because this checkout has no configured remote. #Codex #SoftwareEngineering

## Screenshot suggestions

- Screenshot 1: The real OrdinConn Mobile Intelligence Home view with no private app content visible.
- Screenshot 2: The ten-check M1.5 diagnostic report showing three failures and seven blocked checks.

No screenshot is attached until a real public-safe capture is selected.
