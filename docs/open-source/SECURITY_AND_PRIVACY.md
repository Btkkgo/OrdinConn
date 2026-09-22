# Security and Privacy

[English](SECURITY_AND_PRIVACY.md) | [简体中文](SECURITY_AND_PRIVACY.zh-CN.md)

## Public-repository rule

The repository and its development log are treated as public. A value being present locally is not permission to publish it.

## Prohibited material

- Provider, GitHub, X, or service credentials
- Passwords, cookies, sessions, authorization headers, private keys, or recovery phrases
- Environment files containing secrets
- Private account, customer, contact, or address data
- Raw machine usernames or home-directory paths
- Raw private conversations
- Restricted third-party assets or datasets

## Fail-closed publication

Before a push or scheduled sync, the automation scans the working tree, reachable Git history, tracked paths, and commit-email metadata; validates the diff; verifies the official remote; and ensures only allowlisted public-record paths enter a scheduled commit. Any suspicious credential-shaped value stops the operation.

The scanner reports the file and detector category, not the suspected value. Redaction of a macOS home path produces `~/...`.

## Product safety boundaries

OrdinConn does not collect private keys, recovery phrases, or password-field contents. Real-money actions require explicit approval and are not implemented in V0.1. Mobile collection is application-allowlisted, sensitive UI nodes are redacted, and a `MobileObservation` cannot directly publish a Signal.

## Incident response

If a secret is suspected in Git history, stop sync and publication. Rotate or revoke the credential first, then follow the hosting provider's history-remediation process. Do not rely on a later deletion commit to make an exposed secret safe. Private commit emails and raw machine home prefixes must be removed from the public branch before its first push.

## Privacy repair record — 2026-09-20

One one-time, explicitly authorized privacy history repair replaced only the Author and Committer identity metadata of the M1.5 merge commit with the current GitHub no-reply identity. The tree, both parents, timestamps, commit message, files, code, tests, documentation, and Issue state were unchanged. A precise `--force-with-lease` updated `main` only after the remote still matched the expected old SHA; ordinary force-push remains prohibited.

The pre-repair SHA remained directly accessible through GitHub and remained referenced by PR #5 after the branch update. No further history rewrite was attempted. This is recorded as `GITHUB_CACHED_COMMIT_REMAINS`; GitHub Support cleanup may require separate evaluation.

## M2 privacy repair — 2026-09-22

The M2 merge commit `af843783` had non-noreply Author and Committer metadata. One explicitly authorized `--force-with-lease` replaced it with `5a19c25`. The tree, ordered parents, message, and timestamps are identical; `git diff` is empty. No existing signing setup was configured, so the replacement is unsigned rather than carrying an invalid copy of GitHub's signature. Reachable public `main` identity now passes the noreply preflight. The old commit remains accessible through GitHub and referenced by PR #8 (`GITHUB_CACHED_COMMIT_REMAINS`); any hosting-provider cleanup is a separate task. No further force-push is authorized.

The repository-local identity gate, public-history identity gate, and versioned pre-push hook now guard future merges. GitHub-generated web merge commits are suspended pending independent identity verification. Maintainers should also enable **Keep my email addresses private** and **Block command line pushes that expose my email**, but account settings do not replace repository checks.
