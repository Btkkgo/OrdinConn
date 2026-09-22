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

## Safe PR merge

Maintainers must use a repository-local GitHub noreply identity and run `scripts/security/check-git-identity.sh` before merging. The GitHub-generated web merge commit path is suspended until its identity behavior is independently revalidated. After CI, review, and required tests pass, fetch the latest `main`, perform a controlled local merge, run `scripts/security/check-public-history-identity.sh` on the proposed merge, and push `main` normally. Immediately rerun the history identity scan and `scripts/security/check-public-repo.sh` on the published `main`; a failure blocks stage close. Install the versioned pre-push hook with `scripts/git/install-hooks.sh` where compatible. An unsigned local merge is acceptable when no existing GitHub-recognized signing setup is configured.

Maintainers should enable **Keep my email addresses private** and **Block command line pushes that expose my email** in GitHub account settings. Repository gates remain mandatory regardless of those account settings. Never print a detected private email.

## Security

Do not put a vulnerability report containing sensitive details into a public Issue. Follow [SECURITY.md](SECURITY.md). Never commit secrets, tokens, cookies, sessions, private keys, recovery phrases, personal data, or restricted third-party material.

## Language

Code and commit messages use English. Public documentation, Issue bodies, important Issue comments, and Pull Request bodies are maintained in English first and Simplified Chinese second. Update both language versions in the same change.
