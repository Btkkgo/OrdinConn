# Public Development System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish truthful public development documentation, fail-closed GitHub devlog synchronization, a two-hour macOS scheduler, and a safe X stage-publication queue for OrdinConn.

**Architecture:** Keep public knowledge in repository Markdown, enforce disclosure policy with standalone scripts, and separate scheduled documentation commits from product-code delivery. Shell integration tests exercise Git and LaunchAgent behavior in temporary directories; a standard-library Python publisher handles X thread parsing, API calls, resumable state, and duplicate prevention.

**Tech Stack:** Markdown, POSIX-oriented Bash, Python 3 standard library, Git, macOS launchd, X API v2, existing Rust/Tauri/React workspace.

**Spec:** `docs/superpowers/specs/2026-09-20-public-development-system-design.md`

## Global Constraints

- Public facts must be supported by the current code, documents, Git history, or fresh verification.
- Raw prompts, private chat, credentials, session material, account identifiers, and raw `~/` paths must not enter public files.
- The scheduled sync may stage only public-log documentation and its directly related scripts.
- Secret detection, an invalid diff, a missing remote, or an invalid X payload fails closed.
- No force push, hard reset, clean, repository creation, password collection, cookie extraction, or browser-session extraction.
- The current product branch is `codex/mobile-intelligence-runtime`; `AGENTS.md` has user-owned changes that must be preserved and incrementally extended.
- M1.5 remains failed until a real Android environment passes all ten gates; no M2 claim is allowed.

## Review Focus

- Scanner source and policy prose mention secret names without triggering themselves, while credential-shaped values still fail.
- Existing staged product files cannot be swept into an automated documentation commit.
- Push failure leaves the local commit and files intact for recovery.
- LaunchAgent XML remains valid when the checkout path contains XML-special or whitespace characters.
- A partially published X thread resumes after the last recorded post and never duplicates a completed thread.

---

### Task 1: Public documentation and interaction policy

**Files:**
- Create: `docs/open-source/README.md`
- Create: `docs/open-source/CURRENT_STAGE.md`
- Create: `docs/open-source/ARCHITECTURE.md`
- Create: `docs/open-source/DEVELOPMENT_TIMELINE.md`
- Create: `docs/open-source/DECISIONS.md`
- Create: `docs/open-source/PROBLEMS_AND_SOLUTIONS.md`
- Create: `docs/open-source/CODEX_FIELD_NOTES.md`
- Create: `docs/open-source/INTERACTION_LOG_POLICY.md`
- Create: `docs/open-source/BUILD_IN_PUBLIC.md`
- Create: `docs/open-source/SECURITY_AND_PRIVACY.md`
- Create: `docs/open-source/ROADMAP.md`
- Create: `docs/devlog/2026-09-20.md`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `docs/mobile/M1_5_ACCEPTANCE.md`
- Modify: `docs/superpowers/plans/2026-09-20-mobile-intelligence-m1-5.md`

**Interfaces:**
- Consumes: repository audit at product commit `144c87f`, product baseline, current mobile acceptance record, and Git history.
- Produces: canonical public facts and the `Public Interaction Summary` schema consumed by Task 3 synchronization and Task 4 social content.

- [ ] **Step 1: Write the audit-backed current-stage and architecture documents**

Record implemented, designed, verified, known-problem, decision, lesson, and next sections. Use `~/` for local paths and label real Android work as blocked.

- [ ] **Step 2: Write history and engineering knowledge documents**

Derive dated stages from `git log`; record only problems evidenced by commits, tests, or current documents. Mark any weaker context as not independently verified.

- [ ] **Step 3: Add publication, privacy, and interaction policies**

Define the summary schema, the sanitization boundary, GitHub/X separation, and stage-close workflow.

- [ ] **Step 4: Add the first daily devlog and README links**

Summarize the verified Mobile Intelligence M0/M1/M1.5 work and the public-system audit without reproducing the request text.

- [ ] **Step 5: Extend `AGENTS.md` without removing the existing user changes**

Add a `PUBLIC DEVELOPMENT LOG POLICY` section requiring a sanitized summary after engineering-significant tasks and forbidding raw prompts or sensitive values.

- [ ] **Step 6: Remove known machine-identity paths from tracked public text**

Replace the two current `~/` references with `~/...` or a repository-relative spec reference.

- [ ] **Step 7: Validate files and factual markers**

Run: `test "$(find docs/open-source -maxdepth 1 -type f | wc -l | tr -d ' ')" = 11 && test -f docs/devlog/2026-09-20.md && ! rg -n '~/]+/' README.md AGENTS.md docs/open-source docs/devlog docs/mobile/M1_5_ACCEPTANCE.md docs/superpowers/plans/2026-09-20-mobile-intelligence-m1-5.md`

Expected: all required documents exist and no raw macOS home path remains in the public-document set.

- [ ] **Step 8: Commit documentation**

Run: `git add README.md AGENTS.md docs/open-source docs/devlog docs/mobile/M1_5_ACCEPTANCE.md docs/superpowers/plans/2026-09-20-mobile-intelligence-m1-5.md docs/superpowers/plans/2026-09-20-public-development-system.md && git commit -m "docs: establish open development log system"`

Expected: only documentation/policy files are committed.

### Task 2: Fail-closed sanitizer

**Files:**
- Create: `scripts/public-log/sanitize-public-log.sh`
- Create: `scripts/public-log/tests/test_sanitize_public_log.sh`
- Create: `scripts/public-log/tests/validate_public_docs.py`

**Interfaces:**
- Consumes: Task 1 privacy policy and public-document paths.
- Produces: `sanitize-public-log.sh --check PATH...` and `--redact PATH`, consumed by Tasks 3 and 4.

- [ ] **Step 1: Write failing sanitizer integration tests**

Create fixtures at runtime and assert that ordinary prose passes; private-key headers, assigned credentials, bearer authorization, token-shaped strings, cookie/session values, mnemonic-shaped assignments, `.env` files, and raw `~/` paths fail; redaction emits `~/...`.

- [ ] **Step 2: Run the tests and verify RED**

Run: `bash scripts/public-log/tests/test_sanitize_public_log.sh`

Expected: FAIL because `sanitize-public-log.sh` does not exist.

- [ ] **Step 3: Implement the minimal scanner and redactor**

The script must recurse through supplied directories, skip binary files and `.git`, avoid printing matched secret values, report only file and detector category, and return nonzero on any finding.

- [ ] **Step 4: Run sanitizer tests and verify GREEN**

Run: `bash scripts/public-log/tests/test_sanitize_public_log.sh`

Expected: all sanitizer cases pass.

- [ ] **Step 5: Add and run the public-document validator**

Run: `python3 scripts/public-log/tests/validate_public_docs.py`

Expected: all required files, links, sections, and factual status markers pass.

- [ ] **Step 6: Commit the sanitizer**

Run: `git add scripts/public-log && git commit -m "chore: add fail-closed public log sanitizer"`

Expected: sanitizer, tests, and validator are committed.

### Task 3: Git sync and macOS LaunchAgent

**Files:**
- Create: `scripts/public-log/sync-public-devlog.sh`
- Create: `scripts/public-log/install-macos-sync.sh`
- Create: `scripts/public-log/uninstall-macos-sync.sh`
- Create: `scripts/public-log/com.ordinconn.public-devlog-sync.plist.template`
- Create: `scripts/public-log/tests/test_sync_public_devlog.sh`
- Create: `scripts/public-log/tests/test_macos_sync_install.sh`

**Interfaces:**
- Consumes: Task 2 `sanitize-public-log.sh --check` and Task 1 allowed public paths.
- Produces: safe documentation-only commits, `GITHUB_REMOTE_REQUIRED`, durable push-error logs, and a LaunchAgent with a 7,200-second interval.

- [ ] **Step 1: Write failing sync tests**

Use temporary Git repositories and bare remotes to assert: missing remote fails without staging; no changes creates no commit; allowed changes commit and push; staged product code is rejected; a rejecting remote preserves the local commit and file.

- [ ] **Step 2: Run sync tests and verify RED**

Run: `bash scripts/public-log/tests/test_sync_public_devlog.sh`

Expected: FAIL because the sync script does not exist.

- [ ] **Step 3: Implement the sync script**

Resolve the repository root, acquire a nonblocking lock, verify `origin`, scan tracked and candidate text, reject staged out-of-scope files, run `git diff --check`, stage only allowed paths, no-op cleanly, commit with a UTC timestamp, and push the current branch without force.

- [ ] **Step 4: Run sync tests and verify GREEN**

Run: `bash scripts/public-log/tests/test_sync_public_devlog.sh`

Expected: all sync cases pass.

- [ ] **Step 5: Write failing LaunchAgent rendering tests**

Use test-only output/log directories and skip live launchctl. Assert valid plist, `RunAtLoad`, `StartInterval` 7200, absolute script path, and uninstall cleanup.

- [ ] **Step 6: Run installer tests and verify RED**

Run: `bash scripts/public-log/tests/test_macos_sync_install.sh`

Expected: FAIL because installer scripts and template do not exist.

- [ ] **Step 7: Implement install/uninstall scripts and template**

Render escaped local paths, lint with `plutil`, bootstrap or bootout `gui/$UID`, and never store credentials.

- [ ] **Step 8: Run installer tests and verify GREEN**

Run: `bash scripts/public-log/tests/test_macos_sync_install.sh`

Expected: all rendering and cleanup cases pass.

- [ ] **Step 9: Commit automation**

Run: `git add scripts/public-log && git commit -m "chore: automate public development log sync"`

Expected: only public-log automation files are committed.

### Task 4: X stage queue and safe publisher

**Files:**
- Create: `social/x/README.md`
- Create: `social/x/templates/STAGE_POST_TEMPLATE.md`
- Create: `social/x/queue/current-stage.md`
- Create: `social/x/published/.gitkeep`
- Create: `scripts/social/publish-x-queue.py`
- Create: `scripts/social/tests/test_publish_x_queue.py`

**Interfaces:**
- Consumes: Task 1 verified current-stage facts and Task 2 sanitizer.
- Produces: a reviewed 3–6 post thread, API authentication fallback, resumable publication state, published metadata, URLs, and a content hash.

- [ ] **Step 1: Write failing publisher integration tests**

Start a local HTTP server and assert unauthenticated fallback, 280-character enforcement, ordered reply chaining, partial-state resume, metadata writing, queue movement, and duplicate-hash rejection.

- [ ] **Step 2: Run publisher tests and verify RED**

Run: `python3 -m unittest scripts/social/tests/test_publish_x_queue.py -v`

Expected: FAIL because the publisher does not exist.

- [ ] **Step 3: Implement the publisher**

Parse `Thread N/M` blocks, obtain a user token only from `X_API_USER_TOKEN` or Keychain service `com.ordinconn.x.user-token`, invoke the sanitizer, post JSON to X API v2, persist progress after each post, and move a completed queue file into `published/` with timestamp, URL list, source commit, and SHA-256 hash.

- [ ] **Step 4: Run publisher tests and verify GREEN**

Run: `python3 -m unittest scripts/social/tests/test_publish_x_queue.py -v`

Expected: all publisher cases pass.

- [ ] **Step 5: Generate the verified current-stage thread**

Write 3–6 concise posts covering why, real problem, engineering, result, reusable lesson, and a candid Codex note. State that real Android M1.5 failed because the SDK/emulator was absent. Keep the GitHub reference pending because no repository remote is configured.

- [ ] **Step 6: Commit social assets**

Run: `git add social/x scripts/social && git commit -m "chore: add safe X stage publishing queue"`

Expected: queue, template, publisher, and tests are committed.

### Task 5: Install, validate, review, and synchronize

**Files:**
- Modify: `docs/open-source/CURRENT_STAGE.md`
- Modify: `docs/devlog/2026-09-20.md`
- Local-only: `~/Library/LaunchAgents/com.ordinconn.public-devlog-sync.plist`
- Local-only: `~/Library/Logs/OrdinConn/public-devlog-sync.log`

**Interfaces:**
- Consumes: all prior tasks.
- Produces: final verified local state, installed scheduler, GitHub/X outcome, and audit report.

- [ ] **Step 1: Run focused automation validation**

Run: `bash -n scripts/public-log/*.sh && bash scripts/public-log/tests/test_sanitize_public_log.sh && bash scripts/public-log/tests/test_sync_public_devlog.sh && bash scripts/public-log/tests/test_macos_sync_install.sh && python3 -m unittest scripts/social/tests/test_publish_x_queue.py -v && python3 scripts/public-log/tests/validate_public_docs.py`

Expected: all checks pass.

- [ ] **Step 2: Run the full product validation**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace && npm test && npm run typecheck && npm run build && npm run desktop:build`

Expected: all non-real-Android checks and builds pass; the existing explicit M1.5 real gate remains separately failed due to environment.

- [ ] **Step 3: Run repository hygiene and privacy validation**

Run: `git diff --check && scripts/public-log/sanitize-public-log.sh --check .`

Expected: PASS without exposing matched contents.

- [ ] **Step 4: Install and verify the LaunchAgent**

Run: `scripts/public-log/install-macos-sync.sh && launchctl print "gui/$UID/com.ordinconn.public-devlog-sync"`

Expected: plist lint passes and launchd reports the job; its first sync may log `GITHUB_REMOTE_REQUIRED`.

- [ ] **Step 5: Run one manual sync**

Run: `scripts/public-log/sync-public-devlog.sh`

Expected in the current audited environment: nonzero with `GITHUB_REMOTE_REQUIRED`, with no file loss or unintended staging.

- [ ] **Step 6: Check X authorization and publication eligibility**

Check official API configuration, then an existing logged-in browser session without extracting cookies. Because the required verified GitHub URL is unavailable, keep the thread queued even if browser login exists.

- [ ] **Step 7: Request whole-branch review and fix Critical/Important findings once**

Review the complete range from `fd4dcc8` through final HEAD against this plan and its spec. Use TDD for every accepted Critical/Important fix; ledger all rulings and deferred minors.

- [ ] **Step 8: Commit final rollout record**

Run: `git add docs/open-source/CURRENT_STAGE.md docs/devlog/2026-09-20.md && git commit -m "docs(devlog): record public system rollout"` when those files changed.

Expected: no unrelated product code is staged.

- [ ] **Step 9: Push only if `origin` exists and the final scan passes**

Run: `git push origin codex/mobile-intelligence-runtime`

Expected in the current audited environment: skip and report `GITHUB_REMOTE_REQUIRED`; never create a replacement repository.
