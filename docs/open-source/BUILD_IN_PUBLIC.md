# Build in Public

OrdinConn publishes engineering progress to make architecture, failure modes, and AI-assisted development practices inspectable.

## What is published

- Current implementation status and phase boundaries
- Architecture and security decisions
- Problems, root causes, and verified solutions
- Test and build results with their environment limits
- Sanitized daily engineering summaries
- Stage-level X threads and reusable engineering lessons

## What is not published

- Raw conversations or prompts
- User, customer, or private account data
- Credentials, tokens, cookies, sessions, or private keys
- Restricted third-party resources
- Unverified claims, fabricated benchmarks, or simulated success presented as real

## Cadence

Ordinary engineering work updates the daily DevLog and waits for the two-hour documentation sync. The primary local scheduler is the macOS LaunchAgent `com.ordinconn.github-sync`; it runs at login and every 7,200 seconds without requiring Codex or ChatGPT. A Codex heartbeat may remain paused as a recovery fallback, but it is not the primary scheduler. A meaningful stage close also updates Current Status, Problems and Solutions, Codex Field Notes, and an X draft.

The LaunchAgent starts a dedicated, ad-hoc-signed `OrdinConn GitHub Sync` launcher from `~/Library/Application Support/OrdinConn/automation/`. That launcher is the narrow macOS privacy identity responsible for reading this repository under `Documents`; the job does not grant Files and Folders or Full Disk Access to a general-purpose shell. The launcher invokes `github-sync-runner.sh`, which sets an explicit tool path, calls the repository-owned fail-closed sync script, and writes structured records to `~/Library/Logs/OrdinConn/github-sync.log`. The log rotates at 5 MiB and never records credentials.

On first installation, macOS may ask whether `OrdinConn GitHub Sync` may access the Documents folder. Allow that specific application only. To revoke it later, open **System Settings → Privacy & Security → Files & Folders**, locate **OrdinConn GitHub Sync**, and disable Documents Folder access. If macOS presents it under **Full Disk Access** instead, remove or disable only that same named application. Then run `scripts/github/uninstall-sync-launchagent.sh` to unload the job and remove its local runner and launcher.

Product-code delivery is separate. It requires the relevant test and build gate and uses a product commit, not the scheduled documentation-only commit.

Meaningful work is issue-first. The Issue defines scope and acceptance, receives the execution result, and remains open when validation is blocked. Commits use `Refs #N` until the complete acceptance criteria justify `Fixes #N`.

## Publication states

- `draft`: reviewed public material waiting for manual owner publication.
- `published`: material with timestamp, immutable content hash, source commit, and URL.
- `blocked`: material that failed sanitization, verification, authentication, or remote checks.
