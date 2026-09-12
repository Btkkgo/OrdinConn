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
