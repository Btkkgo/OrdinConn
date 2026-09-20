# Security and Privacy

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
