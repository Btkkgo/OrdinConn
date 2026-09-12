# OrdinConn Agent Rules

1. Read `docs/PRODUCT_BASELINE.md` before making product changes. It is the only product baseline.
2. Never restore features or product definitions from the previous OrdinConn repository.
3. Make small, testable changes and inspect the active implementation before editing it.
4. Keep Evidence distinct from model inference and keep every Published Signal traceable to Evidence.
5. User-funds operations always require explicit approval. V0.1 contains no real-money execution.
6. Never collect private keys, seed phrases, or password-field contents.
7. Route every model through Model Gateway and every data source through Connector Registry.
8. A Trade Proposal must pass through Approval before any execution adapter.
9. Keep core crates transport-agnostic and keep React behind typed IPC contracts.
10. Use the approved user-provided purple, black, and yellow brand system and locale keys for all formal UI copy.
11. Run Rust tests, TypeScript tests, typecheck, and builds before declaring completion.
12. Do not push unless the user explicitly asks.
