# Source Registry

The Source Registry is the only entry point for collection sources. A `SourceDefinition` explicitly declares identity, endpoint, collector type, classification, capabilities, reliability tier, polling policy, rate limit, authentication requirement, retention, enabled state, and health.

V0.1 built-ins are:

- `binance-spot-24h` — public BTCUSDT, ETHUSDT, and SOLUSDT price/volume.
- `binance-btc-trade-stream` — public BTCUSDT trade WebSocket.
- `federal-reserve-press` — official Federal Reserve press RSS.
- `nvidia-newsroom` — official company news RSS for demand research.
- `mempool-space-summary` — public BTC mempool observations.

Capabilities are data, never inferred from source names. Missing authentication, schema drift, an unknown instrument, or a prohibited access boundary marks the source unavailable/degraded and produces no fabricated record.

`SearchProvider` is configurable. Discovery creates a `ResearchTask` containing public candidate URLs; it does not create Evidence or a Signal. Social providers are unavailable until a legitimate public adapter is configured.

