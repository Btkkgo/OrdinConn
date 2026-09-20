# OrdinConn

**AI Financial Intelligence & Execution Agent**

OrdinConn V0.1 is a local-first Tauri desktop foundation for evidence-backed financial intelligence across Traditional Finance and Crypto.

It demonstrates an auditable workflow:

`Data -> Evidence -> Signal -> Agent -> Report -> Proposal -> Approval -> Paper Execution`

## V0.1 Foundation

- Traditional Finance and Crypto ABC Signals
- Evidence provenance and quality validation
- Model-agnostic Agent Runtime and Model Gateway
- OpenAI-compatible Chat Completions adapter plus Mock Model
- Context-aware Agent Dock
- Explicit Approval Capability model
- Paper Execution only
- SQLite current state and append-only runtime audit events
- Bounded Computer Use interfaces

V0.1 never places real orders, transfers assets, signs wallet transactions, or reads private keys or seed phrases.

## Development

```bash
npm install
npm run test
npm run typecheck
npm run build
npm run desktop:dev
```

Rust checks:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

See `docs/PRODUCT_BASELINE.md` and `docs/ARCHITECTURE.md` before making product changes.

## Build in Public

OrdinConn publishes a sanitized engineering record covering architecture decisions, development progress, real problems and solutions, verification results, and practical AI-coding lessons.

- [Current Stage](docs/open-source/CURRENT_STAGE.md)
- [Public Architecture](docs/open-source/ARCHITECTURE.md)
- [Development Timeline](docs/open-source/DEVELOPMENT_TIMELINE.md)
- [Problems and Solutions](docs/open-source/PROBLEMS_AND_SOLUTIONS.md)
- [Codex Field Notes](docs/open-source/CODEX_FIELD_NOTES.md)
- [Daily DevLog](docs/devlog/)

The public record never includes user or customer data, credentials, private conversations, restricted resources, or unverified completion claims. See the [security and privacy policy](docs/open-source/SECURITY_AND_PRIVACY.md).
