# OrdinConn

[English](README.md) | [简体中文](README.zh-CN.md)

[GitHub Repository](https://github.com/Btkkgo/OrdinConn) · [Current Mobile Gate](https://github.com/Btkkgo/OrdinConn/issues/1)

**Open-source model-agnostic AI Agent runtime.**

OrdinConn aims to provide AI models with a runtime layer for perceiving, understanding, and operating computers and mobile environments. Its current V0.1 application applies that runtime to evidence-backed financial intelligence across traditional finance and crypto.

OrdinConn is not another chatbot. It is the auditable layer between model reasoning and real tools.

```text
AI Model
   ↓
Reasoning
   ↓
OrdinConn Runtime
   ↓
Perception
   ↓
Tools / Applications
   ↓
Action
   ↓
Verification
```

## Why OrdinConn

Models can change. Tooling, permissions, provenance, and verification still need stable contracts. OrdinConn separates those concerns so an agent can observe real state, act within explicit authority, verify the result, and preserve an audit trail.

## Architecture

The Rust core is transport-agnostic. React communicates with the Tauri host through typed IPC. Models enter through Model Gateway, and data sources enter through Connector Registry.

The current financial intelligence loop is:

```text
Data → Evidence → Signal → Agent Report → Trade Proposal → Approval → Paper Execution
```

See the [architecture overview](docs/architecture/OVERVIEW.md) and [product baseline](docs/PRODUCT_BASELINE.md).

## Current Status

- Version: `0.1.0`
- Mobile stage: M1.5 real-environment validation complete
- M1.5 gate: **PASS**
- M2: **NOT STARTED**
- Real-money execution: not implemented; V0.1 is paper-only

The real Android gate passed all ten production checks against an Android 16 ARM64 Emulator: environment readiness, frame capture, UI tree, snapshot parse, element references, `MobileObservation`, typed Tauri IPC, and session shutdown. A separate real password-node test also verified that sensitive content does not enter the serialized capture. See [Current Status](docs/CURRENT_STATUS.md) for the evidence boundary.

## Mobile Intelligence

Implemented M0/M1 foundations include device-session contracts, bounded screen frames, semantic UI snapshots, scoped element references, sensitive-node redaction policy, typed IPC projection, persistence, and `MobileObservation`.

Real frame capture, UI-tree capture, redaction verification, observation generation, typed IPC, and shutdown are now verified against the dedicated `OrdinConn_M1_5` AVD. This does not authorize M2 actions; Mobile remains observe-only.

## Computer Runtime

OrdinConn defines bounded computer-perception and action interfaces, permission gates, audit events, and post-action verification. Continuous screen recording and unrestricted automation are not the design. The current Computer Runtime is partial; see the [roadmap](docs/roadmap/README.md).

## Model Independence

Every model adapter is routed through Model Gateway. The repository currently includes an OpenAI-compatible adapter and a deterministic mock adapter. Model output remains inference; it never becomes Evidence merely because a model produced it.

## Safety and Permissions

- No private keys, recovery phrases, or password-field contents are collected.
- User-funds operations require explicit approval; V0.1 has no real-money execution.
- Trade Proposals require exact, time-bounded approval capabilities before any adapter.
- Mobile access is application-allowlisted and sensitive UI nodes are redacted.
- Public synchronization fails closed on suspected secrets, private paths, or prohibited files.

Read [Safety and Approval](docs/SAFETY_AND_APPROVAL.md), [Mobile Security](docs/mobile/MOBILE_SECURITY.md), and [Security Policy](SECURITY.md).

## Development Status

```bash
npm install
npm test
npm run typecheck
npm run build
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

The Tauri bundle is built with:

```bash
npm run desktop:build
```

Environment-dependent checks are reported separately from fixture coverage.

## Build in Public

GitHub is OrdinConn's public engineering source of truth. Issues define meaningful work; DevLogs record decisions, failures, verification, and remaining risk. X drafts are manually curated from verified GitHub records and are never published automatically.

- [Development log](docs/devlog/)
- [Problems and Solutions](docs/PROBLEMS_AND_SOLUTIONS.md)
- [Codex Field Notes](docs/codex/CODEX_FIELD_NOTES.md)
- [Build in Public policy](docs/open-source/BUILD_IN_PUBLIC.md)

## Documentation

- [Current Status](docs/CURRENT_STATUS.md) · [中文](docs/CURRENT_STATUS.zh-CN.md)
- [Architecture](docs/architecture/OVERVIEW.md)
- [Mobile Intelligence](docs/mobile/MOBILE_INTELLIGENCE.md)
- [Model Gateway](docs/MODEL_GATEWAY.md)
- [Connector and Source Registry](docs/SOURCE_REGISTRY.md)
- [Architecture Decisions](docs/decisions/)
- [Security and Privacy](docs/open-source/SECURITY_AND_PRIVACY.md)

## Roadmap

The roadmap advances only on verified gates: complete M1.5 against a real emulator, then implement verified navigation in M2, production App Skills in M3, Evidence promotion in M4, and physical Android devices in M5. See the [public roadmap](docs/roadmap/README.md).

## Contributing

Development is issue-first. Open or join a scoped issue before significant work, preserve the Evidence/inference boundary, and include relevant validation. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Known Limitations

- Mobile navigation actions are not implemented.
- Computer Runtime support is partial and permission-bounded.
- The current financial execution adapter is paper-only.
- Three process-fixture tests can time out under default parallel Rust test load; the same 24-test desktop library suite passes serially. Follow [Issue #4](https://github.com/Btkkgo/OrdinConn/issues/4).

## License

OrdinConn is licensed under the [MIT License](LICENSE).
