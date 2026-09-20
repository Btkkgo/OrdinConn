# OrdinConn Public Development System Design

## Objective

Create a public, auditable record of OrdinConn development without exposing secrets, private conversations, machine identity, or unverified product claims. The system separates documentation sync from product-code delivery and treats GitHub and X as independent publication targets.

## Truth sources

Public status is derived in this order:

1. Current code and tests.
2. Current product and architecture documents.
3. Git history.
4. Historical context explicitly labeled as not independently verified.

Plans are never reported as implemented. Fixture-backed behavior is distinguished from real-environment verification.

## Components

### Public documentation

`docs/open-source/` holds the current stage, public architecture, timeline, decisions, engineering problems, Codex field notes, interaction-log policy, publication policy, security policy, and roadmap. `docs/devlog/` receives one sanitized daily file with append-only interaction summaries.

### Sanitization boundary

`scripts/public-log/sanitize-public-log.sh` supports checking and redacting text files. It rejects credential-shaped values, private-key headers, sensitive environment files, authorization headers, cookies or sessions with values, mnemonic-shaped assignments, and raw macOS home paths. Redaction converts `~/` to `~/`; it never silently edits the source during a check.

The scanner examines all tracked text plus public-log candidates before a sync. Detection is fail-closed: no commit or push follows a finding.

### GitHub synchronization

`scripts/public-log/sync-public-devlog.sh` stages only the public documentation, devlog, social queue, and their automation scripts. It rejects pre-staged paths outside that allowlist, runs the sanitizer and `git diff --check`, creates a dated documentation commit only when changes exist, and pushes the current branch without force.

Missing `origin` returns `GITHUB_REMOTE_REQUIRED`. A push failure preserves both the local files and any local commit.

### macOS scheduling

The install script renders a local LaunchAgent from a username-free repository template. The local plist may contain the absolute checkout path, runs at load, and repeats every 7,200 seconds. Logs live under `~/Library/Logs/OrdinConn/`. Install and uninstall scripts use modern `launchctl` bootstrap/bootout behavior and expose test-only directory overrides.

### X publication

`social/x/queue/` contains reviewed stage threads; `social/x/published/` contains successful publication records. The publisher accepts an official user-context API token only from the runtime environment or macOS Keychain. It scans content, enforces post length, hashes the thread, records partial progress, and resumes without duplicating already-posted entries.

If official API authentication is absent, the queue remains intact and the command returns `X_AUTH_NOT_CONFIGURED`. A logged-in browser may be used only for already-sanitized public content. A missing verified GitHub project URL blocks publication when the thread specification requires that link.

## Interaction flow

Every engineering-significant task ends with a sanitized `Public Interaction Summary`. Ordinary interactions wait for the two-hour documentation sync. A meaningful stage close also updates the stage documents, creates an X thread, verifies the relevant product and automation checks, and uses a separately authorized product-code push path.

## Safety boundaries

- Never publish raw Codex or ChatGPT transcripts.
- Never collect or persist passwords, browser sessions, cookies, private keys, seed phrases, or model/provider tokens.
- Never stage unrelated product code from the scheduled sync.
- Never force-push, hard reset, or clean the working tree.
- Never state that M1.5 or M2 passed without real Android evidence.
- Never treat `MobileObservation` as Evidence or a Signal until the Evidence Pipeline performs an explicit later-phase promotion.

## Verification

Integration tests exercise real scripts in temporary Git repositories. They cover secret rejection, home-path redaction, allowlisted staging, no-op sync, push-failure preservation, missing remote behavior, LaunchAgent rendering, X authentication fallback, post-length rejection, thread posting, reply chaining, publication metadata, and duplicate prevention.

Repository-level validation additionally runs shell syntax checks, plist validation, product TypeScript and Rust checks, frontend build, desktop bundle build, `git diff --check`, and a final full-tree secret scan.
