# Strategy Engine

`strategy-core` evaluates normalized observations using deterministic, versioned definitions. Each definition contains a stable ID, semantic version, market/category, required observation kinds, the standard `1m`, `5m`, `15m`, `1h`, `4h`, and `24h` windows, and centralized parameters.

`StrategyResult` records trigger state, direction, score, reason codes, observation IDs, parameter snapshot, and any rejection reason. That provenance is copied into the Signal Candidate and survives publication. A strategy result cannot bypass the Evidence gate.

Stale data, absent history, missing metrics, and unknown instruments fail closed. Narrative acceleration is always `watch` and carries `not_a_buy_signal`. Instrument and entity alias registries canonicalize inputs before strategy evaluation.

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

