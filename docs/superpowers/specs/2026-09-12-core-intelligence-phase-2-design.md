# OrdinConn Core Intelligence Phase 2 Design

## Objective

Turn Phase 1's one-shot public-data pipeline into a continuously running, restart-tolerant intelligence runtime:

`Continuous Collection -> Rolling History -> Baseline -> Ready Strategy -> Candidate -> Evidence Gate -> Real Published Signal`

Zero Published Signals remains a valid outcome. Thresholds, timestamps, and history must never be altered to manufacture a demonstration.

## Scope and boundaries

- React UI remains unchanged.
- The embedded Rust process owns scheduling, WebSocket lifecycle, rolling history, aggregation, strategy readiness, clustering, diagnostics, and persistence.
- Only public market data is used. No API keys, user-data streams, account/order endpoints, wallet secrets, broker integration, or real execution.
- Binance Spot and Binance USDⓈ-M Futures remain separate sources and canonical instruments.
- The scheduler and engines remain transport-agnostic. Tauri only starts and shuts down the application-owned runtime.

## Collector Scheduler

`CollectorScheduler` owns one task per enabled source, a global semaphore, and a shared lifecycle controller. A per-source task is intrinsically single-flight: it never starts a second run before its current run returns. Cadence comes only from `SourceDefinition.poll`; failures apply bounded exponential backoff plus deterministic jitter.

Lifecycle is `start`, `pause`, `resume`, and graceful `shutdown`. Shutdown stops new polls, signals long-lived streams, awaits every owned task, and leaves no detached task. Persisted scheduler state records scheduled/start/completion/next-run times, consecutive failures, current backoff, and the last result. Restart resumes normal cadence from the saved next-run time without replaying every missed poll.

WebSocket sessions connect, subscribe, respond to protocol heartbeat, detect stale data, reconnect, resubscribe, and terminate on scheduler shutdown. A stale timeout also repairs connections invalidated by macOS sleep/wake or network changes.

## Rolling History and baselines

`RollingHistoryEngine` stores bounded, time-ordered per-instrument/per-metric samples in memory and persists only aggregate buckets. Standard windows are 1m, 5m, 15m, 1h, 4h, and 24h. Metrics are count, sum, mean, min, max, population standard deviation, change, percentage change, z-score, EMA, and volume sum.

Samples use source event time for market calculations and retain received time separately. Out-of-order samples within a five-second tolerance are inserted deterministically; older samples are discarded and reported. Every series has a hard capacity. Buckets persist instrument, metric, OHLC, sum, count, mean, standard deviation, first/last event time, and bucket time. Startup loads recent buckets to restore 1h/4h/24h baselines; short windows may warm up again.

`BaselineSnapshot` returns `READY`, `WARMING_UP`, `INSUFFICIENT_HISTORY`, `STALE`, or `SCHEMA_ERROR`, plus current value, baseline statistics, window, sample count, and timestamps. Strategies read only this contract and never issue ad hoc history queries.

## Binance USDⓈ-M Futures

The independent `binance-usdm-futures` source uses current public `fapi.binance.com` REST endpoints and `fstream.binance.com` public market streams for BTCUSDT, ETHUSDT, and SOLUSDT only. It collects mark price/funding, open interest, aggregate trades, best bid/ask, limited depth, and 24h ticker. No authenticated endpoints are registered.

Canonical IDs are `BTC-USDT-SPOT` and `BTC-USDT-PERP` (and corresponding ETH/SOL IDs). Futures observations retain source, instrument ID, market type, exchange event time, received time, schema version, and only the bounded metrics required by strategies. Full order books and all ticks are not persisted.

## Strategy readiness and explainability

Every strategy run records readiness separately from trigger state. `WARMING_UP`, `MISSING_INPUT`, `STALE_INPUT`, and `SCHEMA_ERROR` produce a not-ready Strategy Run and no ordinary Candidate. Only `READY` results may create Candidates. Evidence rejection remains a later and distinct gate.

Funding uses current funding versus a 24h mean/stddev; OI uses 5m/15m/1h changes; divergence records all four price/OI direction patterns; volume compares the current 5m bucket to prior 5m buckets; order-book imbalance uses finite bid/ask liquidity; spread uses best ask/bid and a rolling baseline. StrategyResult stores input snapshot, trigger metrics, window, reason codes, and timestamps.

Any Published real Signal stores `data_origin=REAL`, strategy/version/parameters, evidence/source IDs, baseline window, trigger metrics, and publication time. A real Signal containing mock Evidence fails closed. Agent explanations must use this provenance rather than inventing retrospective reasons.

## Evidence Cluster and domain timelines

`evidence_clusters` and `evidence_cluster_members` persist original, syndication, independent, and contradicting relationships. Independent confirmation counts distinct canonical sources after canonical URL, original URL, and content-similarity checks; syndicated copies never add confirmation weight. Cluster summary exposes source count, independent-source count, highest reliability, latest time, and contradiction count to Evidence quality.

Fed events map deterministic event classes to entities/assets. Proposed transmission paths are explicitly `INFERENCE`. NVIDIA demand records join durable topic timelines (`ai_infrastructure`, `gpu`, `memory`, `data_center`); persistence requires same-direction observations across distinct time buckets, so a single article cannot produce a high DemandScore. BTC mempool fee/count/vsize feed network baselines and remain neutral observations until a ready strategy evaluates them.

## Diagnostics and verification

Diagnostics aggregate records, observations, Evidence, Strategy Runs, Candidates, publications, parse failures, reconnects, and stale sources without persisting per-tick audit events. Runtime audit records only significant collector lifecycle and strategy readiness/trigger transitions.

Automated tests use local HTTP/WebSocket fixtures and deterministic clocks. `manual_live_smoke` uses real public endpoints only when explicitly enabled. `manual_soak_test` is excluded from CI, supports a configured 30–60 minute duration, and reports runtime counts, duplicate rate, reconnects, scheduler drift, baseline warm-up, bounded history utilization, and SQLite growth.

