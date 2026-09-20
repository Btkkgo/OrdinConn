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

Ordinary engineering work updates the daily DevLog and waits for the two-hour documentation sync. The active local scheduler is a Codex heartbeat because macOS denied a standalone LaunchAgent access to this checkout under `Documents`; the failed LaunchAgent was removed. A meaningful stage close also updates Current Status, Problems and Solutions, Codex Field Notes, and an X draft.

Product-code delivery is separate. It requires the relevant test and build gate and uses a product commit, not the scheduled documentation-only commit.

Meaningful work is issue-first. The Issue defines scope and acceptance, receives the execution result, and remains open when validation is blocked. Commits use `Refs #N` until the complete acceptance criteria justify `Fixes #N`.

## Publication states

- `draft`: reviewed public material waiting for manual owner publication.
- `published`: material with timestamp, immutable content hash, source commit, and URL.
- `blocked`: material that failed sanitization, verification, authentication, or remote checks.
