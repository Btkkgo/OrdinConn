# OrdinConn Core Intelligence Design

## Objective

Build a deterministic, auditable collection and signal pipeline without changing the frozen React UI:

`Source -> RawRecord -> NormalizedObservation -> Dedup -> Evidence -> Strategy -> SignalCandidate -> Evidence Gate -> Published Signal -> Agent Analysis`

The system uses public data only, fails closed when source identity or schema is uncertain, and never turns missing data into a positive signal.

## Boundaries

- `collector-runtime` owns source definitions, public-data policy, REST/WebSocket/feed/HTML collection, normalization, deduplication, health, rate limiting, schema drift detection, and source/instrument/entity registries.
- `strategy-core` owns versioned deterministic strategies, rolling-window inputs, parameter snapshots, reason codes, and market-specific observations.
- `evidence-core` owns reliability tiers, freshness policies, evidence clusters, independent-confirmation accounting, contradictions, and quality scoring.
- `signal-core` remains the publication gate. Strategy output is metadata, never authority to bypass Evidence.
- `ordinconn-app` owns persistence and orchestration. Collection and strategy crates remain transport-agnostic.
- Browser and Computer collectors are interfaces only in V0.1. They must not bypass login, paywall, CAPTCHA, cookie, or private-access boundaries.
- No real-money execution, broker/exchange trading, wallet signing, private keys, or seed phrases.

## Public Source Baseline

- Binance public REST and WebSocket market data for BTCUSDT, ETHUSDT, and SOLUSDT.
- Federal Reserve official press-release RSS for Traditional B events.
- NVIDIA Newsroom official feed and public article pages for Traditional C demand observations.
- mempool.space public REST for BTC on-chain observations.

Each source explicitly declares type, classification, capabilities, reliability tier, polling policy, rate limit, authentication requirements, and health. Capabilities are never inferred from a provider name.

## Records and normalization

`RawRecord` preserves source identity, external ID, canonical URL, retrieval/capture time, content type, content hash, and a bounded raw payload. Normalizers create typed observations with schema version, canonical instrument/entity identifiers, observed time, numeric facts, textual facts, and provenance.

Deduplication uses, in order: source external ID, canonical URL, content hash, then conservative title-and-time similarity. Matching records form evidence clusters classified as original, syndication, or independent confirmation. Syndication never increases the independent-confirmation count.

## Evidence quality

Evidence quality is deterministic and computed from source reliability, freshness, factual completeness, independent confirmations, and contradiction penalty. Model inference cannot set or increase the score and never qualifies as the only publication evidence. Stale or schema-drifted observations cap confidence and may fail validation.

## Strategy model

Every `StrategyDefinition` has a stable ID, semantic version, market, category, input types, rolling windows, and centralized parameters. `evaluate(context)` returns a `StrategyResult` containing triggered state, direction, score, reason codes, supporting observation IDs, and a complete parameter snapshot.

Traditional strategies cover price/volume expansion, volatility, breadth/relative strength, macro repricing, official event classification, and deterministic demand scoring. Crypto strategies cover BTC on-chain conditions, official/narrative event observations, and Binance funding, open-interest, price/OI divergence, volume, order-book imbalance, and spread anomalies.

## Persistence and recovery

Migration-managed SQLite tables store sources, raw records, collector runs, observations, snapshots, strategy definitions/runs, research tasks, evidence clusters, supply-chain relations, wallet watchlists, and website watches. State mutation and append-only audit events share a transaction where both occur. Raw payload retention is bounded by source policy; derived evidence and audit references outlive disposable payloads.

Collectors expose health and schema drift. Startup recovery marks incomplete collector/strategy runs interrupted; it does not replay side-effecting operations.

## Verification

Automated tests use local fixtures and local HTTP/WebSocket servers only. A separately invoked `manual_live_smoke` example performs actual public-network collection and records availability honestly. A missing or blocked source is reported as unavailable; no mock fallback is substituted into a real path.

