# Strategy Engine

`strategy-core` evaluates normalized observations using deterministic, versioned definitions. Each definition contains a stable ID, semantic version, market/category, required observation kinds, the standard `1m`, `5m`, `15m`, `1h`, `4h`, and `24h` windows, and centralized parameters.

`StrategyResult` records readiness, trigger state, direction, score, reason codes, observation IDs, instrument, baseline window, input snapshot, trigger metrics, timestamps, parameter snapshot, and any rejection reason. That provenance is copied into a ready Signal Candidate and survives publication. A strategy result cannot bypass the Evidence gate.

`RollingHistoryEngine` maintains bounded series for 1m/5m/15m/1h/4h/24h and calculates count, sum, mean, min/max, population standard deviation, change, percentage change, z-score, EMA, and volume sum using exchange event time. Five-second out-of-order inserts are sorted; older values are discarded and diagnosed. SQLite stores aggregate OHLC/statistical buckets, not streaming ticks.

Stale data, absent history, missing metrics, and unknown instruments fail closed. Narrative acceleration is always `watch` and carries `not_a_buy_signal`. Instrument and entity alias registries canonicalize inputs before strategy evaluation.

The Phase 2 derivatives strategies use IDs under `crypto.exchange.*` at version `2.0.0`. Funding and volume use historical z-scores, OI uses historical percentage change, price/OI retains all four direction patterns, and imbalance/spread use finite-depth/best-price baselines. Not-ready results create no Candidate.

Current strategy IDs all use version `1.0.0`:

- `traditional.a.price_volume_expansion`
- `traditional.a.volatility_expansion`
- `traditional.a.breadth_relative_strength`
- `traditional.a.macro_repricing`
- `traditional.b.official_event`
- `traditional.c.demand_score`
- `crypto.a.onchain_pressure`
- `crypto.b.official_event`
- `crypto.b.narrative_observation`
- `crypto.c.funding_anomaly`
- `crypto.c.open_interest_expansion`
- `crypto.c.price_oi_divergence`
- `crypto.c.volume_expansion`
- `crypto.c.book_imbalance`
- `crypto.c.spread_anomaly`
