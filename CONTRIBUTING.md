# Contributing to OrdinConn

[English](CONTRIBUTING.md) | [简体中文](CONTRIBUTING.zh-CN.md)

## Issue-first development

Open or join a GitHub Issue before meaningful engineering work. The Issue defines the goal, scope, risks, and acceptance criteria. Small typo fixes may proceed directly, but they still require a clear commit and validation record.

## Bugs

Use the bug template. Include reproducible behavior, environment details, expected behavior, and the smallest safe diagnostic output. Never include credentials, private app content, customer data, or full local paths.

## Features and research

Use the feature or research template. Separate current implementation from design intent. For runtime work, name the permission boundary and the required real-environment evidence.

## Pull requests

1. Link the relevant Issue.
2. Keep the change within its scope.
3. Add or update tests.
4. Update public documentation and the daily DevLog when the change is engineering-significant.
5. Run the relevant Rust, TypeScript, typecheck, build, and packaging gates.
6. Run `scripts/security/check-public-repo.sh` and `git diff --check`.
7. Report failures and blocked checks honestly.

Use `Refs #N` while work is partial. Use `Fixes #N` only when the complete acceptance criteria are satisfied.

## Security

Do not put a vulnerability report containing sensitive details into a public Issue. Follow [SECURITY.md](SECURITY.md). Never commit secrets, tokens, cookies, sessions, private keys, recovery phrases, personal data, or restricted third-party material.

## Language

Code and commit messages use English. Public documentation, Issue bodies, important Issue comments, and Pull Request bodies are maintained in English first and Simplified Chinese second. Update both language versions in the same change.
