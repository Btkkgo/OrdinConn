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

Ordinary engineering work updates the daily DevLog and waits for the two-hour documentation sync. A meaningful stage close also updates Current Stage, Problems and Solutions, Codex Field Notes, and the X queue.

Product-code delivery is separate. It requires the relevant test and build gate and uses a product commit, not the scheduled documentation-only commit.

## Publication states

- `queue`: reviewed public material waiting for a remote or X authorization.
- `published`: material with timestamp, immutable content hash, source commit, and URL.
- `blocked`: material that failed sanitization, verification, authentication, or remote checks.
