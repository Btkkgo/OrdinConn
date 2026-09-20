## Goal

Establish GitHub as OrdinConn's public engineering source of truth with issue-first work, sanitized DevLogs, architecture decisions, a fail-closed security gate, and documentation-only scheduled synchronization.

## Scope

- Public English and Chinese project entry documentation.
- Current status with Implemented, Verified, Partial, Blocked, Designed, Planned, and Not Started states.
- Issue templates, labels, DevLog, ADRs, security policy, contributing guide, and changelog.
- Working-tree and Git-history secret/path checks.
- Two-hour synchronization limited to public documentation, GitHub metadata, and manual X drafts.
- Manual X draft generation only; no X authentication or publication automation.

## Out of Scope

- Product runtime refactoring.
- Automatic X publishing.
- Uploading private prompts, conversations, credentials, local runtime data, build artifacts, Android SDKs, or emulator images.

## Acceptance Criteria

- Public repository exists and Issues are enabled.
- Initial public branch is `main` and contains the current publishable project.
- Security gate passes for the working tree, Git history, tracked paths, and commit metadata.
- Public documents and templates validate.
- Scheduled sync cannot stage product code.
- First manual X draft links to verified GitHub records.
- The Issue receives a final comment with commands, validation, results, and remaining risks.
