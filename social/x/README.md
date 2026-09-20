# OrdinConn on X

[English](README.md) | [简体中文](README.zh-CN.md)

Stage-level Build in Public material moves through two states:

- `drafts/`: sanitized content waiting for manual owner review and publication.
- `published/`: immutable content plus publication time, X URLs, source commit, and SHA-256 content hash.

Templates live in `templates/`. Ordinary code changes belong in the daily DevLog, not in an X post.

## Publication rules

- State only repository-backed or freshly verified facts.
- Keep `Implemented`, `Designed`, `Verified`, and `Blocked` distinct.
- Run the public-log sanitizer before publishing.
- Require a verified GitHub project reference for stage threads.
- Never read browser cookies or sessions, request a password, or store authentication in Git.
- Publication is performed manually by the project owner. Repository automation does not log in to X, read browser state, or call the X API.
- Every draft contains `## English — Publication Version` for the intended public thread and `## 中文 — 参考版本` for the complete Chinese reference thread. Both sections must express the same sanitized facts naturally rather than mechanically translating private conversation.

## Manual workflow

1. Review and sanitize the file in `drafts/`.
2. Confirm the verified GitHub references and select real public-safe screenshots.
3. Publish the thread manually.
4. Add the publication time, X URLs, source commit, and content hash.
5. Move the completed file into `published/`.

No X credential belongs in this repository.
